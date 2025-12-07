'use client'

import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react'
import type { Agent, Action, SystemStats, Loan, InsurancePolicy, Delegation } from './api'

interface DemoContextType {
  isDemoMode: boolean
  toggleDemoMode: () => void
  setDemoMode: (value: boolean) => void
  // Dynamic mock data
  mockStats: SystemStats
  mockAgents: Agent[]
  mockActions: Action[]
  mockLoans: Loan[]
  mockPolicies: InsurancePolicy[]
  mockDelegations: Delegation[]
  // Refresh mock data (for animations)
  refreshMockData: () => void
}

const DemoContext = createContext<DemoContextType | null>(null)

export function useDemoMode() {
  const ctx = useContext(DemoContext)
  if (!ctx) throw new Error('useDemoMode must be used within DemoProvider')
  return ctx
}

// Dynamic mock data generators
function generateMockStats(): SystemStats {
  const baseAGDP = 1247893.45
  const variation = Math.random() * 50000 - 25000
  const actionsCount = 15234 + Math.floor(Math.random() * 100)
  const verifiedCount = Math.floor(actionsCount * 0.977)
  const disputedCount = Math.floor(actionsCount * 0.0064)

  return {
    agdp: {
      total_agdp: baseAGDP + variation,
      actions_count: actionsCount,
      verified_count: verifiedCount,
      disputed_count: disputedCount,
      dispute_rate: disputedCount / actionsCount,
      avg_verification_latency: 800 + Math.random() * 200,
      compute_efficiency: 1.15 + Math.random() * 0.2,
    },
    total_entities: 3847 + Math.floor(Math.random() * 50),
    total_actions: actionsCount,
    total_verified: verifiedCount,
    avg_trust_score: 680 + Math.random() * 20,
    avg_fitness: 1.08 + Math.random() * 0.1,
    culled_count: 23 + Math.floor(Math.random() * 5),
  }
}

function generateMockAgents(): Agent[] {
  const agentTemplates = [
    { name: 'Alpha-7 Analyst', type: 'agent' as const, baseScore: 892, baseEta: 1.45 },
    { name: 'DataBot-X9', type: 'agent' as const, baseScore: 756, baseEta: 1.12 },
    { name: 'ProcessorUnit-3', type: 'agent' as const, baseScore: 534, baseEta: 0.68 },
    { name: 'Acme Corp', type: 'organization' as const, baseScore: 945, baseEta: 1.67 },
    { name: 'John Smith', type: 'human' as const, baseScore: 823, baseEta: 1.34 },
    { name: 'NeuroNet-Prime', type: 'agent' as const, baseScore: 867, baseEta: 1.38 },
    { name: 'Quantum-7B', type: 'agent' as const, baseScore: 712, baseEta: 0.95 },
    { name: 'GlobalTech Inc', type: 'organization' as const, baseScore: 901, baseEta: 1.52 },
  ]

  return agentTemplates.map((template, i) => {
    const scoreVariation = Math.floor(Math.random() * 20 - 10)
    const score = Math.min(1000, Math.max(0, template.baseScore + scoreVariation))
    const tau = score / 1000
    const etaVariation = Math.random() * 0.1 - 0.05
    const eta = Math.max(0, template.baseEta + etaVariation)

    const revenue = 5000 + Math.random() * 95000
    const cost = revenue / (eta / tau || 1)

    return {
      id: `agent-demo-${i.toString().padStart(4, '0')}`,
      name: template.name,
      type: template.type,
      trust_score: {
        score,
        tau,
        tier: score <= 250 ? 0 : score <= 500 ? 1 : score <= 750 ? 2 : 3,
        verifications: 100 + Math.floor(Math.random() * 2000),
        disputes: Math.floor(Math.random() * 30),
        last_updated: new Date().toISOString(),
      },
      wallet: {
        id: `wallet-demo-${i}`,
        balance: 500 + Math.random() * 50000,
        reserved: Math.random() * 500,
        expires_at: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(),
      },
      fitness: {
        eta,
        revenue,
        cost,
        classification: eta >= 1.0 ? 'champion' : eta >= 0.7 ? 'neutral' : 'underperformer',
        hc_allocation: 100 + Math.random() * 2000,
      },
      status: eta < 0.7 ? 'warning' : 'active',
      created_at: new Date(Date.now() - Math.random() * 90 * 24 * 60 * 60 * 1000).toISOString(),
    } as Agent
  })
}

function generateMockActions(): Action[] {
  const actionTypes = ['inference', 'analysis', 'generation', 'classification', 'embedding']
  const statuses = ['verified', 'verified', 'verified', 'verified', 'processing', 'pending', 'disputed'] as const

  return Array.from({ length: 15 }, (_, i) => {
    const status = statuses[Math.floor(Math.random() * statuses.length)]
    const baseCompute = 0.05 + Math.random() * 0.1
    const riskPremium = 0.01 + Math.random() * 0.04
    const trustDiscount = 0.005 + Math.random() * 0.025

    return {
      id: `act-demo-${(1000 + i).toString()}`,
      producer_id: `agent-demo-${Math.floor(Math.random() * 8).toString().padStart(4, '0')}`,
      consumer_id: `agent-demo-${Math.floor(Math.random() * 8).toString().padStart(4, '0')}`,
      action_type: actionTypes[Math.floor(Math.random() * actionTypes.length)],
      status,
      pricing: {
        base_compute: baseCompute,
        risk_premium: riskPremium,
        trust_discount: trustDiscount,
        final_price: baseCompute + riskPremium - trustDiscount,
      },
      verification: status === 'verified' || status === 'disputed' ? {
        oracle_votes: ['Oracle-Alpha', 'Oracle-Beta', 'Oracle-Gamma', 'Oracle-Delta', 'Oracle-Epsilon'].map((name, j) => ({
          oracle_id: `oracle-${j}`,
          oracle_name: name,
          vote: status === 'verified' ? true : Math.random() > 0.4,
          timestamp: new Date().toISOString(),
        })),
        quorum_reached: status === 'verified',
        quorum_threshold: '3-of-5',
        aggregate_signature: '0x' + Array.from({ length: 64 }, () => 'abcdef0123456789'[Math.floor(Math.random() * 16)]).join(''),
        latency_ms: 600 + Math.random() * 800,
      } : undefined,
      created_at: new Date(Date.now() - Math.random() * 48 * 60 * 60 * 1000).toISOString(),
      verified_at: status === 'verified' ? new Date(Date.now() - Math.random() * 24 * 60 * 60 * 1000).toISOString() : undefined,
    } as Action
  })
}

function generateMockLoans(): Loan[] {
  const statuses = ['active', 'active', 'repaid', 'defaulted'] as const
  return Array.from({ length: 5 }, (_, i) => {
    const status = statuses[Math.floor(Math.random() * statuses.length)]
    const principal = 500 + Math.random() * 10000
    const tau = 0.5 + Math.random() * 0.5
    const interestRate = 0.032 * (2.0 - tau)

    return {
      id: `loan-demo-${i}`,
      lender_id: `agent-demo-${Math.floor(Math.random() * 8).toString().padStart(4, '0')}`,
      borrower_id: `agent-demo-${Math.floor(Math.random() * 8).toString().padStart(4, '0')}`,
      principal,
      interest_rate: interestRate,
      term_days: [30, 60, 90][Math.floor(Math.random() * 3)],
      status,
      created_at: new Date(Date.now() - Math.random() * 30 * 24 * 60 * 60 * 1000).toISOString(),
      due_at: new Date(Date.now() + Math.random() * 60 * 24 * 60 * 60 * 1000).toISOString(),
      repaid_amount: status === 'repaid' ? principal * (1 + interestRate) : status === 'active' ? principal * Math.random() * 0.5 : 0,
    }
  })
}

function generateMockPolicies(): InsurancePolicy[] {
  const statuses = ['active', 'active', 'claimed', 'expired'] as const
  const actionTypes = ['inference', 'analysis', 'generation']
  return Array.from({ length: 4 }, (_, i) => {
    const status = statuses[Math.floor(Math.random() * statuses.length)]
    const coverage = 1000 + Math.random() * 20000
    const tau = 0.5 + Math.random() * 0.5
    const premiumRate = 0.05 * (1 + (1 - tau))
    const premium = coverage * premiumRate

    return {
      id: `policy-demo-${i}`,
      insurer_id: `agent-demo-${Math.floor(Math.random() * 8).toString().padStart(4, '0')}`,
      insured_id: `agent-demo-${Math.floor(Math.random() * 8).toString().padStart(4, '0')}`,
      coverage,
      premium,
      premium_rate: premiumRate,
      action_type: actionTypes[Math.floor(Math.random() * actionTypes.length)],
      status,
      created_at: new Date(Date.now() - Math.random() * 30 * 24 * 60 * 60 * 1000).toISOString(),
      expires_at: new Date(Date.now() + Math.random() * 90 * 24 * 60 * 60 * 1000).toISOString(),
    }
  })
}

function generateMockDelegations(): Delegation[] {
  const statuses = ['pending', 'active', 'completed', 'disputed'] as const
  const tasks = [
    'Analyze market trends',
    'Generate quarterly report',
    'Process customer data',
    'Optimize ML model',
    'Audit smart contracts',
  ]
  return Array.from({ length: 4 }, (_, i) => {
    const status = statuses[Math.floor(Math.random() * statuses.length)]
    const escrowAmount = 200 + Math.random() * 5000

    return {
      id: `delegation-demo-${i}`,
      client_id: `agent-demo-${Math.floor(Math.random() * 8).toString().padStart(4, '0')}`,
      agent_id: `agent-demo-${Math.floor(Math.random() * 8).toString().padStart(4, '0')}`,
      task_description: tasks[Math.floor(Math.random() * tasks.length)],
      escrow_amount: escrowAmount,
      status,
      created_at: new Date(Date.now() - Math.random() * 14 * 24 * 60 * 60 * 1000).toISOString(),
      deadline: new Date(Date.now() + Math.random() * 30 * 24 * 60 * 60 * 1000).toISOString(),
      completed_at: status === 'completed' ? new Date(Date.now() - Math.random() * 7 * 24 * 60 * 60 * 1000).toISOString() : undefined,
    }
  })
}

export function DemoProvider({ children }: { children: ReactNode }) {
  const [isDemoMode, setIsDemoMode] = useState(true) // Default to demo mode
  const [mockStats, setMockStats] = useState<SystemStats>(generateMockStats)
  const [mockAgents, setMockAgents] = useState<Agent[]>(generateMockAgents)
  const [mockActions, setMockActions] = useState<Action[]>(generateMockActions)
  const [mockLoans, setMockLoans] = useState<Loan[]>(generateMockLoans)
  const [mockPolicies, setMockPolicies] = useState<InsurancePolicy[]>(generateMockPolicies)
  const [mockDelegations, setMockDelegations] = useState<Delegation[]>(generateMockDelegations)

  const toggleDemoMode = useCallback(() => {
    setIsDemoMode(prev => !prev)
  }, [])

  const setDemoMode = useCallback((value: boolean) => {
    setIsDemoMode(value)
  }, [])

  const refreshMockData = useCallback(() => {
    setMockStats(generateMockStats())
    setMockAgents(generateMockAgents())
    setMockActions(generateMockActions())
    setMockLoans(generateMockLoans())
    setMockPolicies(generateMockPolicies())
    setMockDelegations(generateMockDelegations())
  }, [])

  // Auto-refresh mock data periodically in demo mode for live feel
  useEffect(() => {
    if (!isDemoMode) return

    const interval = setInterval(() => {
      // Subtle updates to stats to show "live" data
      setMockStats(prev => ({
        ...prev,
        agdp: {
          ...prev.agdp,
          total_agdp: prev.agdp.total_agdp + (Math.random() * 100 - 20),
          actions_count: prev.agdp.actions_count + Math.floor(Math.random() * 3),
          avg_verification_latency: 800 + Math.random() * 200,
        }
      }))
    }, 5000) // Update every 5 seconds

    return () => clearInterval(interval)
  }, [isDemoMode])

  return (
    <DemoContext.Provider value={{
      isDemoMode,
      toggleDemoMode,
      setDemoMode,
      mockStats,
      mockAgents,
      mockActions,
      mockLoans,
      mockPolicies,
      mockDelegations,
      refreshMockData,
    }}>
      {children}
    </DemoContext.Provider>
  )
}
