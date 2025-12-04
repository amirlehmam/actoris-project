//! Malachite BFT Consensus Implementation (HotStuff-2 based)
//!
//! This module implements a Byzantine Fault Tolerant consensus protocol based on
//! HotStuff-2, optimized for Actoris verification workflow with:
//! - Two-phase commit (Prepare → Commit)
//! - Pipelined block processing
//! - Leader rotation
//! - View change protocol
//!
//! Reference: HotStuff-2 paper - https://eprint.iacr.org/2023/397

use crate::consensus::QuorumManager;
use actoris_common::{ActorisError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, instrument, warn};

/// View number (monotonically increasing)
pub type ViewNumber = u64;

/// Block height
pub type Height = u64;

/// Node identifier (oracle DID hash)
pub type NodeId = [u8; 32];

/// Block hash
pub type BlockHash = [u8; 32];

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Node's own identifier
    pub node_id: NodeId,
    /// All validator node IDs
    pub validators: Vec<NodeId>,
    /// Quorum threshold (typically 2f+1 for 3f+1 nodes)
    pub threshold: usize,
    /// View timeout duration
    pub view_timeout: Duration,
    /// Proposal timeout
    pub proposal_timeout: Duration,
    /// Maximum transactions per block
    pub max_txs_per_block: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            node_id: [0u8; 32],
            validators: vec![],
            threshold: 3,
            view_timeout: Duration::from_secs(2),
            proposal_timeout: Duration::from_millis(500),
            max_txs_per_block: 1000,
        }
    }
}

/// Verification request to be included in consensus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VerificationRequest {
    pub request_id: String,
    pub actor_did: String,
    pub action_type: String,
    pub input_hash: [u8; 32],
    pub output_hash: [u8; 32],
    pub compute_hc: String,
    pub submitted_at: i64,
}

/// Verification result from consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub request_id: String,
    pub passed: bool,
    pub latency_ms: u32,
    pub view: ViewNumber,
    pub height: Height,
    pub signature: Vec<u8>,
    pub signers: Vec<NodeId>,
}

/// Block containing verification requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block height
    pub height: Height,
    /// View in which block was proposed
    pub view: ViewNumber,
    /// Parent block hash
    pub parent_hash: BlockHash,
    /// Block proposer
    pub proposer: NodeId,
    /// Verification requests in this block
    pub requests: Vec<VerificationRequest>,
    /// Timestamp
    pub timestamp: i64,
    /// Block hash (computed)
    #[serde(skip)]
    pub hash: BlockHash,
}

impl Block {
    /// Create a new block
    pub fn new(
        height: Height,
        view: ViewNumber,
        parent_hash: BlockHash,
        proposer: NodeId,
        requests: Vec<VerificationRequest>,
    ) -> Self {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let mut block = Self {
            height,
            view,
            parent_hash,
            proposer,
            requests,
            timestamp,
            hash: [0u8; 32],
        };
        block.hash = block.compute_hash();
        block
    }

    /// Compute block hash
    pub fn compute_hash(&self) -> BlockHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.height.to_le_bytes());
        hasher.update(&self.view.to_le_bytes());
        hasher.update(&self.parent_hash);
        hasher.update(&self.proposer);
        hasher.update(&self.timestamp.to_le_bytes());
        for req in &self.requests {
            hasher.update(req.request_id.as_bytes());
            hasher.update(&req.input_hash);
            hasher.update(&req.output_hash);
        }
        *hasher.finalize().as_bytes()
    }

    /// Genesis block
    pub fn genesis() -> Self {
        Self {
            height: 0,
            view: 0,
            parent_hash: [0u8; 32],
            proposer: [0u8; 32],
            requests: vec![],
            timestamp: 0,
            hash: [0u8; 32],
        }
    }
}

/// Vote types in HotStuff-2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    /// First phase vote (prepare)
    Prepare,
    /// Second phase vote (commit)
    Commit,
}

/// Vote message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub vote_type: VoteType,
    pub view: ViewNumber,
    pub block_hash: BlockHash,
    pub voter: NodeId,
    pub signature: Vec<u8>,
}

impl Vote {
    /// Create a new vote
    pub fn new(vote_type: VoteType, view: ViewNumber, block_hash: BlockHash, voter: NodeId) -> Self {
        Self {
            vote_type,
            view,
            block_hash,
            voter,
            signature: vec![], // Will be filled by FROST signing
        }
    }

    /// Compute vote hash for signing
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[self.vote_type as u8]);
        hasher.update(&self.view.to_le_bytes());
        hasher.update(&self.block_hash);
        hasher.update(&self.voter);
        *hasher.finalize().as_bytes()
    }
}

/// Quorum Certificate - proof of 2f+1 votes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumCertificate {
    pub vote_type: VoteType,
    pub view: ViewNumber,
    pub block_hash: BlockHash,
    /// Aggregated FROST signature
    pub signature: Vec<u8>,
    /// Voters who contributed
    pub voters: Vec<NodeId>,
}

impl QuorumCertificate {
    /// Check if QC has enough signers
    pub fn has_quorum(&self, threshold: usize) -> bool {
        self.voters.len() >= threshold
    }
}

/// View change message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChange {
    pub new_view: ViewNumber,
    pub sender: NodeId,
    /// Highest QC known to sender
    pub high_qc: Option<QuorumCertificate>,
    pub signature: Vec<u8>,
}

/// New view message from leader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewView {
    pub view: ViewNumber,
    pub leader: NodeId,
    /// Highest QC among all view changes
    pub high_qc: Option<QuorumCertificate>,
    /// View change proofs
    pub view_changes: Vec<ViewChange>,
}

/// Consensus messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    Proposal(Block, Option<QuorumCertificate>),
    Vote(Vote),
    ViewChange(ViewChange),
    NewView(NewView),
}

/// Consensus state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusPhase {
    /// Waiting for proposal
    WaitingForProposal,
    /// Voted prepare, waiting for prepare QC
    Prepare,
    /// Voted commit, waiting for commit QC
    Commit,
    /// View change in progress
    ViewChange,
}

/// Internal consensus state
struct ConsensusState {
    /// Current view number
    view: ViewNumber,
    /// Current phase
    phase: ConsensusPhase,
    /// Current block being voted on
    current_block: Option<Block>,
    /// Highest prepare QC
    prepare_qc: Option<QuorumCertificate>,
    /// Highest commit QC (locked)
    locked_qc: Option<QuorumCertificate>,
    /// Committed blocks by height
    committed_blocks: BTreeMap<Height, Block>,
    /// Last committed height
    last_committed_height: Height,
    /// Pending verification requests
    pending_requests: Vec<VerificationRequest>,
    /// Votes received for current view
    prepare_votes: HashMap<BlockHash, Vec<Vote>>,
    commit_votes: HashMap<BlockHash, Vec<Vote>>,
    /// View change messages
    view_changes: HashMap<ViewNumber, Vec<ViewChange>>,
    /// View start time
    view_start: Instant,
}

impl ConsensusState {
    fn new() -> Self {
        Self {
            view: 0,
            phase: ConsensusPhase::WaitingForProposal,
            current_block: None,
            prepare_qc: None,
            locked_qc: None,
            committed_blocks: BTreeMap::new(),
            last_committed_height: 0,
            pending_requests: vec![],
            prepare_votes: HashMap::new(),
            commit_votes: HashMap::new(),
            view_changes: HashMap::new(),
            view_start: Instant::now(),
        }
    }
}

/// Network interface for consensus
#[async_trait]
pub trait ConsensusNetwork: Send + Sync {
    /// Broadcast message to all validators
    async fn broadcast(&self, msg: ConsensusMessage) -> Result<()>;
    /// Send message to specific node
    async fn send(&self, to: NodeId, msg: ConsensusMessage) -> Result<()>;
    /// Receive messages
    async fn receive(&self) -> Result<(NodeId, ConsensusMessage)>;
}

/// Callback for committed blocks
#[async_trait]
pub trait CommitCallback: Send + Sync {
    /// Called when a block is committed
    async fn on_commit(&self, block: &Block, qc: &QuorumCertificate) -> Result<()>;
}

/// Malachite BFT Consensus Engine
pub struct MalachiteConsensus {
    config: ConsensusConfig,
    state: Arc<RwLock<ConsensusState>>,
    network: Arc<dyn ConsensusNetwork>,
    commit_callback: Arc<dyn CommitCallback>,
    quorum: QuorumManager,
    /// Channel to submit verification requests
    request_tx: mpsc::Sender<VerificationRequest>,
    request_rx: Arc<RwLock<mpsc::Receiver<VerificationRequest>>>,
    /// Shutdown signal
    shutdown: Arc<RwLock<bool>>,
}

impl MalachiteConsensus {
    /// Create new consensus instance
    pub fn new(
        config: ConsensusConfig,
        network: Arc<dyn ConsensusNetwork>,
        commit_callback: Arc<dyn CommitCallback>,
    ) -> Self {
        let threshold = config.threshold;
        let total = config.validators.len();
        let (request_tx, request_rx) = mpsc::channel(10000);

        Self {
            config,
            state: Arc::new(RwLock::new(ConsensusState::new())),
            network,
            commit_callback,
            quorum: QuorumManager::new(threshold as u8, total as u8),
            request_tx,
            request_rx: Arc::new(RwLock::new(request_rx)),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Get request submission channel
    pub fn request_sender(&self) -> mpsc::Sender<VerificationRequest> {
        self.request_tx.clone()
    }

    /// Submit verification request
    pub async fn submit_request(&self, request: VerificationRequest) -> Result<()> {
        self.request_tx
            .send(request)
            .await
            .map_err(|e| ActorisError::Consensus(format!("Failed to submit request: {}", e)))
    }

    /// Start consensus engine
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        info!("Starting Malachite consensus engine");

        // Spawn view timer
        let view_timeout = self.config.view_timeout;
        let state = self.state.clone();
        let network = self.network.clone();
        let config = self.config.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            loop {
                if *shutdown.read().await {
                    break;
                }

                tokio::time::sleep(Duration::from_millis(100)).await;

                let should_timeout = {
                    let s = state.read().await;
                    s.view_start.elapsed() > view_timeout
                };

                if should_timeout {
                    let mut s = state.write().await;
                    let new_view = s.view + 1;
                    info!(view = new_view, "View timeout, initiating view change");
                    s.view = new_view;
                    s.phase = ConsensusPhase::ViewChange;
                    s.view_start = Instant::now();

                    let vc = ViewChange {
                        new_view,
                        sender: config.node_id,
                        high_qc: s.prepare_qc.clone(),
                        signature: vec![],
                    };
                    drop(s);

                    let _ = network.broadcast(ConsensusMessage::ViewChange(vc)).await;
                }
            }
        });

        // Spawn request collector
        let state = self.state.clone();
        let request_rx = self.request_rx.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            loop {
                if *shutdown.read().await {
                    break;
                }

                let mut rx = request_rx.write().await;
                match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
                    Ok(Some(req)) => {
                        let mut s = state.write().await;
                        s.pending_requests.push(req);
                    }
                    Ok(None) => break,
                    Err(_) => continue,
                }
            }
        });

        // Main consensus loop
        self.run_consensus_loop().await
    }

    /// Main consensus loop
    async fn run_consensus_loop(&self) -> Result<()> {
        loop {
            if *self.shutdown.read().await {
                break;
            }

            // Check if we're the leader
            if self.is_leader().await {
                self.try_propose().await?;
            }

            // Process incoming messages
            match tokio::time::timeout(Duration::from_millis(50), self.network.receive()).await {
                Ok(Ok((from, msg))) => {
                    self.handle_message(from, msg).await?;
                }
                Ok(Err(e)) => {
                    warn!("Network error: {}", e);
                }
                Err(_) => {
                    // Timeout, continue
                }
            }
        }

        Ok(())
    }

    /// Check if this node is the leader for current view
    async fn is_leader(&self) -> bool {
        let state = self.state.read().await;
        let leader_idx = (state.view as usize) % self.config.validators.len();
        self.config.validators.get(leader_idx) == Some(&self.config.node_id)
    }

    /// Get current leader
    fn get_leader(&self, view: ViewNumber) -> NodeId {
        let leader_idx = (view as usize) % self.config.validators.len();
        self.config.validators[leader_idx]
    }

    /// Try to propose a block (if leader)
    #[instrument(skip(self))]
    async fn try_propose(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if state.phase != ConsensusPhase::WaitingForProposal {
            return Ok(());
        }

        if state.pending_requests.is_empty() {
            return Ok(());
        }

        // Take requests for this block
        let requests: Vec<_> = state
            .pending_requests
            .drain(..self.config.max_txs_per_block.min(state.pending_requests.len()))
            .collect();

        let parent_hash = state
            .committed_blocks
            .values()
            .last()
            .map(|b| b.hash)
            .unwrap_or([0u8; 32]);

        let block = Block::new(
            state.last_committed_height + 1,
            state.view,
            parent_hash,
            self.config.node_id,
            requests,
        );

        info!(
            height = block.height,
            view = block.view,
            txs = block.requests.len(),
            "Proposing block"
        );

        state.current_block = Some(block.clone());
        state.phase = ConsensusPhase::Prepare;
        drop(state);

        // Broadcast proposal
        let state = self.state.read().await;
        self.network
            .broadcast(ConsensusMessage::Proposal(block, state.prepare_qc.clone()))
            .await?;

        Ok(())
    }

    /// Handle incoming consensus message
    #[instrument(skip(self, msg))]
    async fn handle_message(&self, from: NodeId, msg: ConsensusMessage) -> Result<()> {
        match msg {
            ConsensusMessage::Proposal(block, qc) => {
                self.handle_proposal(from, block, qc).await?;
            }
            ConsensusMessage::Vote(vote) => {
                self.handle_vote(from, vote).await?;
            }
            ConsensusMessage::ViewChange(vc) => {
                self.handle_view_change(from, vc).await?;
            }
            ConsensusMessage::NewView(nv) => {
                self.handle_new_view(from, nv).await?;
            }
        }
        Ok(())
    }

    /// Handle proposal
    async fn handle_proposal(
        &self,
        from: NodeId,
        block: Block,
        justify_qc: Option<QuorumCertificate>,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        // Verify proposer is legitimate leader
        if from != self.get_leader(block.view) {
            warn!("Received proposal from non-leader");
            return Ok(());
        }

        // Verify block is for current or next view
        if block.view < state.view {
            return Ok(());
        }

        // Verify justify QC
        if let Some(ref qc) = justify_qc {
            if !qc.has_quorum(self.config.threshold) {
                warn!("Invalid justify QC");
                return Ok(());
            }
        }

        // Safe to vote check (liveness vs safety)
        let safe_to_vote = match (&state.locked_qc, &justify_qc) {
            (None, _) => true,
            (Some(locked), Some(justify)) => justify.view >= locked.view,
            (Some(_), None) => false,
        };

        if !safe_to_vote {
            warn!("Not safe to vote on proposal");
            return Ok(());
        }

        info!(
            height = block.height,
            view = block.view,
            "Accepting proposal, voting prepare"
        );

        state.current_block = Some(block.clone());
        state.phase = ConsensusPhase::Prepare;
        state.view = block.view;
        state.view_start = Instant::now();

        // Create prepare vote
        let vote = Vote::new(VoteType::Prepare, block.view, block.hash, self.config.node_id);

        drop(state);

        // Send vote to leader
        self.network
            .send(from, ConsensusMessage::Vote(vote))
            .await?;

        Ok(())
    }

    /// Handle vote
    async fn handle_vote(&self, from: NodeId, vote: Vote) -> Result<()> {
        // Verify voter is a validator
        if !self.config.validators.contains(&from) {
            return Ok(());
        }

        let mut state = self.state.write().await;

        // Check vote is for current view
        if vote.view != state.view {
            return Ok(());
        }

        match vote.vote_type {
            VoteType::Prepare => {
                let votes = state
                    .prepare_votes
                    .entry(vote.block_hash)
                    .or_insert_with(Vec::new);

                // Avoid duplicates
                if votes.iter().any(|v| v.voter == from) {
                    return Ok(());
                }

                votes.push(vote.clone());

                debug!(
                    block_hash = hex::encode(&vote.block_hash[..8]),
                    votes = votes.len(),
                    threshold = self.config.threshold,
                    "Received prepare vote"
                );

                // Check if we have quorum
                if votes.len() >= self.config.threshold {
                    info!("Prepare quorum reached, creating QC");

                    let qc = QuorumCertificate {
                        vote_type: VoteType::Prepare,
                        view: vote.view,
                        block_hash: vote.block_hash,
                        signature: vec![], // Will be FROST aggregated
                        voters: votes.iter().map(|v| v.voter).collect(),
                    };

                    state.prepare_qc = Some(qc);
                    state.phase = ConsensusPhase::Commit;

                    // Broadcast commit vote
                    let commit_vote = Vote::new(
                        VoteType::Commit,
                        vote.view,
                        vote.block_hash,
                        self.config.node_id,
                    );

                    drop(state);
                    self.network
                        .broadcast(ConsensusMessage::Vote(commit_vote))
                        .await?;
                }
            }
            VoteType::Commit => {
                let votes = state
                    .commit_votes
                    .entry(vote.block_hash)
                    .or_insert_with(Vec::new);

                if votes.iter().any(|v| v.voter == from) {
                    return Ok(());
                }

                votes.push(vote.clone());

                debug!(
                    block_hash = hex::encode(&vote.block_hash[..8]),
                    votes = votes.len(),
                    threshold = self.config.threshold,
                    "Received commit vote"
                );

                // Check if we have quorum
                if votes.len() >= self.config.threshold {
                    info!("Commit quorum reached, committing block");

                    let qc = QuorumCertificate {
                        vote_type: VoteType::Commit,
                        view: vote.view,
                        block_hash: vote.block_hash,
                        signature: vec![],
                        voters: votes.iter().map(|v| v.voter).collect(),
                    };

                    state.locked_qc = Some(qc.clone());

                    // Commit the block
                    if let Some(block) = state.current_block.take() {
                        state.committed_blocks.insert(block.height, block.clone());
                        state.last_committed_height = block.height;

                        // Advance to next view
                        state.view += 1;
                        state.phase = ConsensusPhase::WaitingForProposal;
                        state.view_start = Instant::now();
                        state.prepare_votes.clear();
                        state.commit_votes.clear();

                        drop(state);

                        // Notify callback
                        self.commit_callback.on_commit(&block, &qc).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle view change
    async fn handle_view_change(&self, from: NodeId, vc: ViewChange) -> Result<()> {
        if !self.config.validators.contains(&from) {
            return Ok(());
        }

        let mut state = self.state.write().await;

        let vcs = state
            .view_changes
            .entry(vc.new_view)
            .or_insert_with(Vec::new);

        if vcs.iter().any(|v| v.sender == from) {
            return Ok(());
        }

        vcs.push(vc.clone());

        // Check if we have enough view changes to become new leader
        if vcs.len() >= self.config.threshold {
            let new_leader = self.get_leader(vc.new_view);

            if new_leader == self.config.node_id {
                info!(view = vc.new_view, "Becoming new view leader");

                // Find highest QC among view changes
                let high_qc = vcs
                    .iter()
                    .filter_map(|v| v.high_qc.as_ref())
                    .max_by_key(|qc| qc.view)
                    .cloned();

                let new_view_msg = NewView {
                    view: vc.new_view,
                    leader: self.config.node_id,
                    high_qc,
                    view_changes: vcs.clone(),
                };

                state.view = vc.new_view;
                state.phase = ConsensusPhase::WaitingForProposal;
                state.view_start = Instant::now();

                drop(state);

                self.network
                    .broadcast(ConsensusMessage::NewView(new_view_msg))
                    .await?;
            }
        }

        Ok(())
    }

    /// Handle new view message
    async fn handle_new_view(&self, from: NodeId, nv: NewView) -> Result<()> {
        if from != self.get_leader(nv.view) {
            return Ok(());
        }

        let mut state = self.state.write().await;

        if nv.view <= state.view && state.phase != ConsensusPhase::ViewChange {
            return Ok(());
        }

        info!(view = nv.view, leader = ?hex::encode(&from[..8]), "Entering new view");

        state.view = nv.view;
        state.phase = ConsensusPhase::WaitingForProposal;
        state.view_start = Instant::now();
        state.prepare_votes.clear();
        state.commit_votes.clear();

        if let Some(qc) = nv.high_qc {
            if state.prepare_qc.as_ref().map(|q| q.view).unwrap_or(0) < qc.view {
                state.prepare_qc = Some(qc);
            }
        }

        Ok(())
    }

    /// Shutdown consensus
    pub async fn shutdown(&self) {
        *self.shutdown.write().await = true;
    }

    /// Get current view
    pub async fn current_view(&self) -> ViewNumber {
        self.state.read().await.view
    }

    /// Get last committed height
    pub async fn last_committed_height(&self) -> Height {
        self.state.read().await.last_committed_height
    }

    /// Get committed block at height
    pub async fn get_block(&self, height: Height) -> Option<Block> {
        self.state.read().await.committed_blocks.get(&height).cloned()
    }

    /// Get consensus metrics
    pub async fn metrics(&self) -> ConsensusMetrics {
        let state = self.state.read().await;
        ConsensusMetrics {
            current_view: state.view,
            last_committed_height: state.last_committed_height,
            pending_requests: state.pending_requests.len(),
            committed_blocks: state.committed_blocks.len(),
            phase: format!("{:?}", state.phase),
        }
    }
}

/// Consensus metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMetrics {
    pub current_view: ViewNumber,
    pub last_committed_height: Height,
    pub pending_requests: usize,
    pub committed_blocks: usize,
    pub phase: String,
}

/// In-memory network for testing
pub struct InMemoryNetwork {
    tx: mpsc::Sender<(NodeId, NodeId, ConsensusMessage)>,
    rx: Arc<RwLock<mpsc::Receiver<(NodeId, NodeId, ConsensusMessage)>>>,
    node_id: NodeId,
    peers: Arc<RwLock<HashMap<NodeId, mpsc::Sender<(NodeId, ConsensusMessage)>>>>,
}

impl InMemoryNetwork {
    pub fn new(node_id: NodeId) -> Self {
        let (tx, rx) = mpsc::channel(10000);
        Self {
            tx,
            rx: Arc::new(RwLock::new(rx)),
            node_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn connect(&self, peer_id: NodeId, sender: mpsc::Sender<(NodeId, ConsensusMessage)>) {
        self.peers.write().await.insert(peer_id, sender);
    }

    pub fn get_sender(&self) -> mpsc::Sender<(NodeId, ConsensusMessage)> {
        let tx = self.tx.clone();
        let node_id = self.node_id;
        let (sender, mut receiver) = mpsc::channel(10000);

        tokio::spawn(async move {
            while let Some((from, msg)) = receiver.recv().await {
                let _ = tx.send((from, node_id, msg)).await;
            }
        });

        sender
    }
}

#[async_trait]
impl ConsensusNetwork for InMemoryNetwork {
    async fn broadcast(&self, msg: ConsensusMessage) -> Result<()> {
        let peers = self.peers.read().await;
        for (_, sender) in peers.iter() {
            let _ = sender.send((self.node_id, msg.clone())).await;
        }
        Ok(())
    }

    async fn send(&self, to: NodeId, msg: ConsensusMessage) -> Result<()> {
        let peers = self.peers.read().await;
        if let Some(sender) = peers.get(&to) {
            let _ = sender.send((self.node_id, msg)).await;
        }
        Ok(())
    }

    async fn receive(&self) -> Result<(NodeId, ConsensusMessage)> {
        let mut rx = self.rx.write().await;
        match rx.recv().await {
            Some((from, _to, msg)) => Ok((from, msg)),
            None => Err(ActorisError::Consensus("Network closed".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_hash() {
        let block1 = Block::new(1, 0, [0u8; 32], [1u8; 32], vec![]);
        let block2 = Block::new(1, 0, [0u8; 32], [1u8; 32], vec![]);
        assert_eq!(block1.hash, block2.hash);

        let block3 = Block::new(2, 0, [0u8; 32], [1u8; 32], vec![]);
        assert_ne!(block1.hash, block3.hash);
    }

    #[test]
    fn test_vote_hash() {
        let vote1 = Vote::new(VoteType::Prepare, 0, [0u8; 32], [1u8; 32]);
        let vote2 = Vote::new(VoteType::Prepare, 0, [0u8; 32], [1u8; 32]);
        assert_eq!(vote1.hash(), vote2.hash());

        let vote3 = Vote::new(VoteType::Commit, 0, [0u8; 32], [1u8; 32]);
        assert_ne!(vote1.hash(), vote3.hash());
    }

    #[test]
    fn test_quorum_certificate() {
        let qc = QuorumCertificate {
            vote_type: VoteType::Prepare,
            view: 0,
            block_hash: [0u8; 32],
            signature: vec![],
            voters: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
        };

        assert!(qc.has_quorum(3));
        assert!(!qc.has_quorum(4));
    }
}
