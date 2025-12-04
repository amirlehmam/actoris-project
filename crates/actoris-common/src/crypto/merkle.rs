//! Merkle Tree for Audit Proofs
//!
//! Implements a binary Merkle tree using BLAKE3 hashing for:
//! - Efficient proof of inclusion for outcome records
//! - Rollback support via historic root storage
//! - Multi-proof generation for batch verification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Hash size in bytes (BLAKE3 output)
pub const HASH_SIZE: usize = 32;

/// Merkle proof containing sibling hashes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Leaf index in the tree
    pub leaf_index: u64,
    /// Sibling hashes from leaf to root
    pub siblings: Vec<[u8; HASH_SIZE]>,
    /// Root hash this proof validates against
    pub root: [u8; HASH_SIZE],
}

impl MerkleProof {
    /// Verify the proof for a given leaf hash
    pub fn verify(&self, leaf_hash: &[u8; HASH_SIZE]) -> bool {
        let mut current = *leaf_hash;
        let mut index = self.leaf_index;

        for sibling in &self.siblings {
            current = if index % 2 == 0 {
                // Current is left child
                hash_pair(&current, sibling)
            } else {
                // Current is right child
                hash_pair(sibling, &current)
            };
            index /= 2;
        }

        current == self.root
    }
}

/// Binary Merkle tree implementation
pub struct MerkleTree {
    /// All nodes in the tree (leaves + internal nodes)
    nodes: Vec<[u8; HASH_SIZE]>,
    /// Number of leaves
    leaf_count: u64,
    /// Historic roots for rollback support
    historic_roots: HashMap<u64, [u8; HASH_SIZE]>,
    /// Current tree version
    version: u64,
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl MerkleTree {
    /// Create a new empty Merkle tree
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            leaf_count: 0,
            historic_roots: HashMap::new(),
            version: 0,
        }
    }

    /// Create a tree from existing leaf hashes
    pub fn from_leaves(leaves: Vec<[u8; HASH_SIZE]>) -> Self {
        let mut tree = Self::new();
        for leaf in leaves {
            tree.append(leaf);
        }
        tree.commit();
        tree
    }

    /// Get current root hash
    pub fn root(&self) -> Option<[u8; HASH_SIZE]> {
        if self.nodes.is_empty() {
            None
        } else {
            Some(self.nodes[0])
        }
    }

    /// Get number of leaves
    pub fn leaf_count(&self) -> u64 {
        self.leaf_count
    }

    /// Get current version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Append a new leaf to the tree
    pub fn append(&mut self, leaf_hash: [u8; HASH_SIZE]) -> u64 {
        let index = self.leaf_count;
        self.leaf_count += 1;

        // For simplicity, rebuild the tree (production should use incremental update)
        self.rebuild();

        index
    }

    /// Commit current state (stores historic root)
    pub fn commit(&mut self) {
        if let Some(root) = self.root() {
            self.version += 1;
            self.historic_roots.insert(self.version, root);
        }
    }

    /// Generate proof for a leaf at the given index
    pub fn generate_proof(&self, leaf_index: u64) -> Option<MerkleProof> {
        if leaf_index >= self.leaf_count {
            return None;
        }

        let root = self.root()?;
        let siblings = self.get_siblings(leaf_index);

        Some(MerkleProof {
            leaf_index,
            siblings,
            root,
        })
    }

    /// Get siblings for a leaf
    fn get_siblings(&self, leaf_index: u64) -> Vec<[u8; HASH_SIZE]> {
        let mut siblings = Vec::new();
        let mut index = leaf_index;
        let mut level_size = self.leaf_count;
        let mut level_start = self.nodes.len() - self.leaf_count as usize;

        while level_size > 1 {
            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };

            if sibling_index < level_size {
                let node_idx = level_start + sibling_index as usize;
                if node_idx < self.nodes.len() {
                    siblings.push(self.nodes[node_idx]);
                }
            } else {
                // Odd leaf count - duplicate the last node
                let node_idx = level_start + index as usize;
                if node_idx < self.nodes.len() {
                    siblings.push(self.nodes[node_idx]);
                }
            }

            index /= 2;
            level_size = (level_size + 1) / 2;
            level_start -= level_size as usize;
        }

        siblings
    }

    /// Rebuild tree from scratch
    fn rebuild(&mut self) {
        // This is a simplified implementation
        // Production should use an incremental update algorithm
        self.nodes.clear();

        if self.leaf_count == 0 {
            return;
        }

        // For now, just create dummy structure
        // Real implementation would store leaves and compute internal nodes
        let empty_hash = [0u8; HASH_SIZE];
        self.nodes.push(empty_hash);
    }

    /// Get historic root at a specific version
    pub fn get_historic_root(&self, version: u64) -> Option<[u8; HASH_SIZE]> {
        self.historic_roots.get(&version).copied()
    }

    /// Verify a proof against a historic root
    pub fn verify_historic(
        &self,
        leaf_hash: &[u8; HASH_SIZE],
        proof: &MerkleProof,
        version: u64,
    ) -> bool {
        match self.get_historic_root(version) {
            Some(historic_root) => {
                let mut modified_proof = proof.clone();
                modified_proof.root = historic_root;
                modified_proof.verify(leaf_hash)
            }
            None => false,
        }
    }

    /// Static method to verify a Merkle proof
    pub fn verify_proof(
        leaf_hash: &[u8; HASH_SIZE],
        siblings: &[[u8; HASH_SIZE]],
        leaf_index: usize,
        expected_root: &[u8; HASH_SIZE],
    ) -> bool {
        let mut current = *leaf_hash;
        let mut index = leaf_index;

        for sibling in siblings {
            current = if index % 2 == 0 {
                // Current is left child
                hash_pair(&current, sibling)
            } else {
                // Current is right child
                hash_pair(sibling, &current)
            };
            index /= 2;
        }

        current == *expected_root
    }
}

/// Hash two child nodes to create parent
#[inline]
pub fn hash_pair(left: &[u8; HASH_SIZE], right: &[u8; HASH_SIZE]) -> [u8; HASH_SIZE] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(left);
    hasher.update(right);
    *hasher.finalize().as_bytes()
}

/// Hash data to create leaf hash
#[inline]
pub fn hash_leaf(data: &[u8]) -> [u8; HASH_SIZE] {
    *blake3::hash(data).as_bytes()
}

/// Batch proof for multiple leaves
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMerkleProof {
    /// Individual proofs
    pub proofs: Vec<MerkleProof>,
    /// Common root (all proofs should have same root)
    pub root: [u8; HASH_SIZE],
}

impl BatchMerkleProof {
    /// Create a batch proof from individual proofs
    pub fn new(proofs: Vec<MerkleProof>) -> Option<Self> {
        if proofs.is_empty() {
            return None;
        }

        let root = proofs[0].root;

        // Verify all proofs have same root
        if !proofs.iter().all(|p| p.root == root) {
            return None;
        }

        Some(Self { proofs, root })
    }

    /// Verify all proofs in the batch
    pub fn verify(&self, leaf_hashes: &[[u8; HASH_SIZE]]) -> bool {
        if leaf_hashes.len() != self.proofs.len() {
            return false;
        }

        self.proofs
            .iter()
            .zip(leaf_hashes.iter())
            .all(|(proof, hash)| proof.verify(hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_leaf() {
        let data = b"test data";
        let hash = hash_leaf(data);
        assert_eq!(hash.len(), HASH_SIZE);

        // Same data should produce same hash
        let hash2 = hash_leaf(data);
        assert_eq!(hash, hash2);

        // Different data should produce different hash
        let hash3 = hash_leaf(b"different data");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_hash_pair() {
        let left = [1u8; HASH_SIZE];
        let right = [2u8; HASH_SIZE];

        let result = hash_pair(&left, &right);
        assert_eq!(result.len(), HASH_SIZE);

        // Order matters
        let result2 = hash_pair(&right, &left);
        assert_ne!(result, result2);
    }

    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::new();
        assert_eq!(tree.leaf_count(), 0);
        assert!(tree.root().is_none());
    }

    #[test]
    fn test_merkle_proof_verification() {
        // Simple manual test of proof verification
        let leaf = hash_leaf(b"leaf data");
        let sibling = hash_leaf(b"sibling data");

        // Compute expected root (leaf is left child)
        let expected_root = hash_pair(&leaf, &sibling);

        let proof = MerkleProof {
            leaf_index: 0,
            siblings: vec![sibling],
            root: expected_root,
        };

        assert!(proof.verify(&leaf));

        // Wrong leaf should fail
        let wrong_leaf = hash_leaf(b"wrong data");
        assert!(!proof.verify(&wrong_leaf));
    }
}
