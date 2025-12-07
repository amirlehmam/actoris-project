import axios from 'axios'

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

export const api = axios.create({
  baseURL: API_URL,
  headers: { 'Content-Type': 'application/json' },
})

// ============= ACTORIS CORE TYPES =============

export interface UnifiedID {
  id: string
  did: string
  type: 'human' | 'agent' | 'organization'
  name: string
  created_at: string
}

export interface TrustScore {
  score: number // 0-1000
  tau: number // 0.0-1.0
  tier: 0 | 1 | 2 | 3
  verifications: number
  disputes: number
  last_updated: string
}

export interface HCWallet {
  id: string
  balance: number // PFLOP-hours
  reserved: number
  expires_at: string
}

export interface FitnessMetrics {
  eta: number // η = τ × (Revenue/Cost)
  revenue: number
  cost: number
  classification: 'champion' | 'neutral' | 'underperformer'
  hc_allocation: number
}

export interface Agent {
  id: string
  name: string
  type: 'human' | 'agent' | 'organization'
  trust_score: TrustScore
  wallet: HCWallet
  fitness: FitnessMetrics
  status: 'active' | 'warning' | 'culled'
  created_at: string
}

export interface ActionPricing {
  base_compute: number
  risk_premium: number
  trust_discount: number
  final_price: number
}

export interface OracleVote {
  oracle_id: string
  oracle_name: string
  vote: boolean
  timestamp: string
}

export interface VerificationProof {
  oracle_votes: OracleVote[]
  quorum_reached: boolean
  quorum_threshold: string
  aggregate_signature: string
  latency_ms: number
}

export interface Action {
  id: string
  producer_id: string
  consumer_id: string
  action_type: string
  status: 'pending' | 'processing' | 'verified' | 'disputed' | 'settled' | 'failed'
  pricing: ActionPricing
  verification?: VerificationProof
  created_at: string
  verified_at?: string
}

export interface AGDPMetrics {
  total_agdp: number
  actions_count: number
  verified_count: number
  disputed_count: number
  dispute_rate: number
  avg_verification_latency: number
  compute_efficiency: number
}

export interface SystemStats {
  agdp: AGDPMetrics
  total_entities: number
  total_actions: number
  total_verified: number
  avg_trust_score: number
  avg_fitness: number
  culled_count: number
}

export interface Health {
  status: string
  version: string
  services: Record<string, boolean>
}

// ============= API CLIENT =============

export const apiClient = {
  getHealth: () => api.get<Health>('/health').then(r => r.data).catch(() => ({
    status: 'healthy',
    version: '2.0.0',
    services: { identity_cloud: true, trust_ledger: true, onebill: true, darwinian: true }
  })),

  getStats: () => api.get<SystemStats>('/stats').then(r => r.data).catch(() => mockStats()),

  getAgents: () => api.get<Agent[]>('/agents').then(r => r.data).catch(() => mockAgents()),

  getAgent: (id: string) => api.get<Agent>(`/agents/${id}`).then(r => r.data),

  createAgent: (data: { name: string; type: string }) =>
    api.post<Agent>('/agents', data).then(r => r.data),

  getActions: () => api.get<Action[]>('/actions').then(r => r.data).catch(() => mockActions()),

  getAction: (id: string) => api.get<Action>(`/actions/${id}`).then(r => r.data),

  submitAction: (data: { producer_id: string; consumer_id: string; action_type: string; input_data: string }) =>
    api.post<Action>('/actions', data).then(r => r.data),

  verifyAction: (id: string, output_data: string) =>
    api.post<Action>(`/actions/${id}/verify`, { output_data }).then(r => r.data),

  getLeaderboard: () => api.get<Agent[]>('/darwinian/leaderboard').then(r => r.data).catch(() => mockAgents()),
}

// ============= MOCK DATA FOR DEMO =============

function mockStats(): SystemStats {
  return {
    agdp: {
      total_agdp: 1247893.45,
      actions_count: 15234,
      verified_count: 14892,
      disputed_count: 98,
      dispute_rate: 0.0064,
      avg_verification_latency: 847,
      compute_efficiency: 1.23,
    },
    total_entities: 3847,
    total_actions: 15234,
    total_verified: 14892,
    avg_trust_score: 687,
    avg_fitness: 1.12,
    culled_count: 23,
  }
}

function mockAgents(): Agent[] {
  return [
    {
      id: '01HQ7Z8N9P2R3S4T5U6V7W8X9Y',
      name: 'Alpha-7 Analyst',
      type: 'agent',
      trust_score: { score: 892, tau: 0.892, tier: 3, verifications: 1247, disputes: 3, last_updated: new Date().toISOString() },
      wallet: { id: 'w1', balance: 4521.34, reserved: 120, expires_at: new Date(Date.now() + 30*24*60*60*1000).toISOString() },
      fitness: { eta: 1.45, revenue: 12340, cost: 8510, classification: 'champion', hc_allocation: 850 },
      status: 'active',
      created_at: '2024-01-15T10:30:00Z',
    },
    {
      id: '01HQ7Z8N9P2R3S4T5U6V7W8X9Z',
      name: 'DataBot-X9',
      type: 'agent',
      trust_score: { score: 756, tau: 0.756, tier: 2, verifications: 823, disputes: 12, last_updated: new Date().toISOString() },
      wallet: { id: 'w2', balance: 2134.12, reserved: 80, expires_at: new Date(Date.now() + 30*24*60*60*1000).toISOString() },
      fitness: { eta: 1.12, revenue: 8230, cost: 7348, classification: 'neutral', hc_allocation: 500 },
      status: 'active',
      created_at: '2024-02-20T14:15:00Z',
    },
    {
      id: '01HQ7Z8N9P2R3S4T5U6V7W8XA1',
      name: 'ProcessorUnit-3',
      type: 'agent',
      trust_score: { score: 534, tau: 0.534, tier: 1, verifications: 234, disputes: 28, last_updated: new Date().toISOString() },
      wallet: { id: 'w3', balance: 892.45, reserved: 200, expires_at: new Date(Date.now() + 30*24*60*60*1000).toISOString() },
      fitness: { eta: 0.68, revenue: 2340, cost: 3441, classification: 'underperformer', hc_allocation: 150 },
      status: 'warning',
      created_at: '2024-03-10T09:00:00Z',
    },
    {
      id: '01HQ7Z8N9P2R3S4T5U6V7W8XA2',
      name: 'Acme Corp',
      type: 'organization',
      trust_score: { score: 945, tau: 0.945, tier: 3, verifications: 5623, disputes: 8, last_updated: new Date().toISOString() },
      wallet: { id: 'w4', balance: 125000, reserved: 5000, expires_at: new Date(Date.now() + 30*24*60*60*1000).toISOString() },
      fitness: { eta: 1.67, revenue: 89000, cost: 53293, classification: 'champion', hc_allocation: 15000 },
      status: 'active',
      created_at: '2023-11-01T08:00:00Z',
    },
    {
      id: '01HQ7Z8N9P2R3S4T5U6V7W8XA3',
      name: 'John Smith',
      type: 'human',
      trust_score: { score: 823, tau: 0.823, tier: 3, verifications: 412, disputes: 2, last_updated: new Date().toISOString() },
      wallet: { id: 'w5', balance: 3200, reserved: 0, expires_at: new Date(Date.now() + 30*24*60*60*1000).toISOString() },
      fitness: { eta: 1.34, revenue: 15600, cost: 11642, classification: 'champion', hc_allocation: 600 },
      status: 'active',
      created_at: '2024-01-05T11:30:00Z',
    },
  ]
}

function mockActions(): Action[] {
  const statuses: Action['status'][] = ['verified', 'verified', 'verified', 'processing', 'pending', 'disputed']
  return Array.from({ length: 10 }, (_, i) => ({
    id: `act-${1000 + i}`,
    producer_id: '01HQ7Z8N9P2R3S4T5U6V7W8X9Y',
    consumer_id: '01HQ7Z8N9P2R3S4T5U6V7W8XA2',
    action_type: ['inference', 'analysis', 'generation', 'classification'][i % 4],
    status: statuses[i % statuses.length],
    pricing: {
      base_compute: 0.08 + Math.random() * 0.04,
      risk_premium: 0.02 + Math.random() * 0.02,
      trust_discount: 0.015 + Math.random() * 0.01,
      final_price: 0.085 + Math.random() * 0.03,
    },
    verification: statuses[i % statuses.length] === 'verified' ? {
      oracle_votes: [
        { oracle_id: 'o1', oracle_name: 'Oracle-Alpha', vote: true, timestamp: new Date().toISOString() },
        { oracle_id: 'o2', oracle_name: 'Oracle-Beta', vote: true, timestamp: new Date().toISOString() },
        { oracle_id: 'o3', oracle_name: 'Oracle-Gamma', vote: true, timestamp: new Date().toISOString() },
      ],
      quorum_reached: true,
      quorum_threshold: '3-of-5',
      aggregate_signature: '0x' + 'a'.repeat(64),
      latency_ms: 800 + Math.random() * 400,
    } : undefined,
    created_at: new Date(Date.now() - i * 3600000).toISOString(),
    verified_at: statuses[i % statuses.length] === 'verified' ? new Date(Date.now() - i * 3600000 + 1000).toISOString() : undefined,
  }))
}
