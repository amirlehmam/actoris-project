import axios from 'axios'

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

export const api = axios.create({
  baseURL: API_URL,
  headers: {
    'Content-Type': 'application/json',
  },
})

// Types
export interface Agent {
  id: string
  name: string
  agent_type: 'human' | 'ai' | 'hybrid' | 'contract'
  trust_score: TrustScore
  wallet_id: string
  created_at: string
  metadata: Record<string, string>
}

export interface TrustScore {
  tau: number
  raw_score: number
  tier: number
  verifications: number
  disputes: number
  last_updated: string
}

export interface Wallet {
  id: string
  agent_id: string
  balance: string
  locked: string
  pending: string
  transactions: Transaction[]
}

export interface Transaction {
  id: string
  tx_type: 'deposit' | 'withdrawal' | 'action_payment' | 'verification_reward' | 'dispute_penalty' | 'stake' | 'unstake'
  amount: string
  from: string | null
  to: string | null
  action_id: string | null
  timestamp: string
}

export interface Action {
  id: string
  producer_id: string
  consumer_id: string
  action_type: string
  status: 'pending' | 'processing' | 'verified' | 'disputed' | 'settled' | 'failed'
  input_hash: string
  output_hash: string | null
  price: string | null
  created_at: string
  verified_at: string | null
  verification_proof: VerificationProof | null
}

export interface VerificationProof {
  oracle_votes: OracleVote[]
  quorum_reached: boolean
  aggregate_signature: string
  timestamp: string
}

export interface OracleVote {
  oracle_id: string
  vote: boolean
  signature: string
}

export interface Stats {
  total_agents: number
  total_actions: number
  total_verified: number
  total_hc_volume: string
  average_trust_score: number
}

export interface Health {
  status: string
  version: string
  services: Record<string, boolean>
}

// ============= PRAXIS Types =============

export interface EnvironmentState {
  textual_repr: string
  visual_hash?: string
  state_features: Record<string, string>
  element_ids: string[]
  captured_at: string
}

export interface InternalState {
  directive: string
  sub_task?: string
  progress: number
  task_tags: string[]
}

export interface AgentAction {
  action_type: string
  target?: string
  raw_action: string
  parameters: Record<string, string>
}

export interface ActionOutcome {
  type: 'success' | 'failure' | 'partial'
  description: string
  error_code?: string
  completion_pct?: number
  recoverable?: boolean
}

export interface PraxisMemory {
  id: string
  agent_id: string
  env_state_pre: EnvironmentState
  internal_state: InternalState
  action: AgentAction
  env_state_post: EnvironmentState
  outcome: ActionOutcome
  created_at: string
  retrieval_count: number
  last_retrieved?: string
  reinforcement_score: number
  source: 'agent_experience' | 'human_demonstration' | 'agent_transfer' | 'synthetic'
}

export interface RetrievedMemory {
  memory: PraxisMemory
  env_similarity: number
  internal_similarity: number
  relevance_score: number
  rank: number
}

export interface ProceduralCompetence {
  total_memories: number
  successful_memories: number
  success_rate: number
  diversity_score: number
  generalization_score: number
  learning_velocity: number
  retrieval_accuracy: number
  memory_utilization: number
  fitness_multiplier: number
  calculated_at: string
}

export interface CompetenceSnapshot {
  timestamp: string
  success_rate: number
  total_memories: number
  fitness_multiplier: number
}

export interface LearningMetrics {
  agent_id: string
  current: ProceduralCompetence
  history: CompetenceSnapshot[]
  trend: -1 | 0 | 1
  days_since_improvement: number
  is_actively_learning: boolean
  should_protect_for_learning: boolean
}

export interface SuggestedAction {
  action: string
  reasoning: string
  supporting_memories: number[]
  confidence: number
}

export interface AugmentedMemory {
  rank: number
  relevance: number
  action: string
  was_successful: boolean
  outcome_summary: string
  directive: string
  usage_count: number
}

export interface ActionAugmentation {
  memories: AugmentedMemory[]
  suggested_action?: SuggestedAction
  confidence: number
  warning?: string
  context_string: string
}

export interface PraxisStats {
  total_memories: number
  unique_agents: number
  avg_memories_per_agent: number
  max_memories_per_agent: number
  successful_memories: number
  success_rate: number
}

// API Functions
export const apiClient = {
  // Health & Stats
  getHealth: () => api.get<Health>('/health').then(r => r.data),
  getStats: () => api.get<Stats>('/stats').then(r => r.data),

  // Agents
  getAgents: () => api.get<Agent[]>('/agents').then(r => r.data),
  getAgent: (id: string) => api.get<Agent>(`/agents/${id}`).then(r => r.data),
  createAgent: (data: { name: string; agent_type: string; metadata?: Record<string, string> }) =>
    api.post<{ agent: Agent; wallet: Wallet }>('/agents', data).then(r => r.data),
  getTrustScore: (id: string) => api.get<TrustScore>(`/agents/${id}/trust`).then(r => r.data),

  // Wallet
  getWallet: (agentId: string) => api.get<Wallet>(`/agents/${agentId}/wallet`).then(r => r.data),
  deposit: (agentId: string, amount: string) =>
    api.post<Wallet>(`/agents/${agentId}/wallet/deposit`, { amount }).then(r => r.data),

  // Actions
  getActions: () => api.get<Action[]>('/actions').then(r => r.data),
  getAction: (id: string) => api.get<Action>(`/actions/${id}`).then(r => r.data),
  getAgentActions: (agentId: string) => api.get<Action[]>(`/agents/${agentId}/actions`).then(r => r.data),
  submitAction: (data: {
    producer_id: string
    consumer_id: string
    action_type: string
    input_data: string
  }) => api.post<{ action: Action; estimated_price: string }>('/actions', data).then(r => r.data),
  verifyAction: (actionId: string, output_data: string) =>
    api.post<{ action: Action; proof: VerificationProof }>(`/actions/${actionId}/verify`, { output_data }).then(r => r.data),

  // ============= PRAXIS API =============

  // Get PRAXIS stats
  getPraxisStats: () => api.get<PraxisStats>('/praxis/stats').then(r => r.data),

  // Get memories for an agent
  getAgentMemories: (agentId: string, limit?: number) =>
    api.get<PraxisMemory[]>(`/praxis/agents/${agentId}/memories`, { params: { limit } }).then(r => r.data),

  // Get a specific memory
  getMemory: (memoryId: string) =>
    api.get<PraxisMemory>(`/praxis/memories/${memoryId}`).then(r => r.data),

  // Store a new memory
  storeMemory: (data: {
    agent_id: string
    env_state_pre: EnvironmentState
    internal_state: InternalState
    action: AgentAction
    env_state_post: EnvironmentState
    outcome: ActionOutcome
  }) => api.post<{ memory_id: string }>('/praxis/memories', data).then(r => r.data),

  // Retrieve relevant memories
  retrieveMemories: (data: {
    agent_id: string
    current_env: EnvironmentState
    current_internal: InternalState
    max_results?: number
    similarity_threshold?: number
  }) => api.post<{ memories: RetrievedMemory[]; total_searched: number; search_time_ms: number }>(
    '/praxis/retrieve', data
  ).then(r => r.data),

  // Get procedural competence
  getProceduralCompetence: (agentId: string) =>
    api.get<ProceduralCompetence>(`/praxis/agents/${agentId}/competence`).then(r => r.data),

  // Get learning metrics
  getLearningMetrics: (agentId: string) =>
    api.get<LearningMetrics>(`/praxis/agents/${agentId}/learning`).then(r => r.data),

  // Get action augmentation
  getActionAugmentation: (data: {
    agent_id: string
    current_env: EnvironmentState
    current_internal: InternalState
    max_memories?: number
  }) => api.post<ActionAugmentation>('/praxis/augment', data).then(r => r.data),

  // Import demonstrations
  importDemonstrations: (agentId: string, demonstrations: Array<{
    env_state_pre: EnvironmentState
    internal_state: InternalState
    action: AgentAction
    env_state_post: EnvironmentState
    outcome: ActionOutcome
  }>) => api.post<{ imported_count: number; memory_ids: string[] }>(
    `/praxis/agents/${agentId}/import`, { demonstrations }
  ).then(r => r.data),
}

// WebSocket connection
export function createWebSocket(onMessage: (event: any) => void) {
  const wsUrl = API_URL.replace('http', 'ws') + '/ws'
  const ws = new WebSocket(wsUrl)

  ws.onmessage = (event) => {
    const data = JSON.parse(event.data)
    onMessage(data)
  }

  ws.onerror = (error) => {
    console.error('WebSocket error:', error)
  }

  return ws
}
