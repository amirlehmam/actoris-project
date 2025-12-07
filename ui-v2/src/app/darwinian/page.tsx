'use client'

import { useQuery } from '@tanstack/react-query'
import { apiClient, Agent } from '@/lib/api'
import { useDemoMode } from '@/lib/demo-context'
import { formatNumber, formatCurrency, getFitnessColor } from '@/lib/utils'
import { TrendingUp, Trophy, AlertTriangle, Skull, Zap, Target, Activity } from 'lucide-react'

export default function DarwinianPage() {
  const { isDemoMode, mockAgents } = useDemoMode()

  const { data: liveAgents } = useQuery({
    queryKey: ['agents'],
    queryFn: apiClient.getAgents,
    enabled: !isDemoMode,
  })

  const agents = isDemoMode ? mockAgents : liveAgents

  // Sort by fitness
  const sortedAgents = [...(agents || [])].sort((a, b) => b.fitness.eta - a.fitness.eta)

  const champions = sortedAgents.filter(a => a.fitness.classification === 'champion')
  const neutrals = sortedAgents.filter(a => a.fitness.classification === 'neutral')
  const underperformers = sortedAgents.filter(a => a.fitness.classification === 'underperformer')
  const culled = sortedAgents.filter(a => a.status === 'culled')

  const avgFitness = sortedAgents.reduce((acc, a) => acc + a.fitness.eta, 0) / (sortedAgents.length || 1)

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <div className="flex items-center gap-3">
          <h1 className="text-3xl font-bold text-gray-900">Darwinian Engine</h1>
          {isDemoMode && (
            <span className="px-2 py-1 text-xs font-medium bg-actoris-100 text-actoris-700 rounded-full">
              Demo Mode
            </span>
          )}
        </div>
        <p className="text-gray-500 mt-1">
          Automated resource allocation — The best agents survive, the rest get culled
        </p>
      </div>

      {/* Formula Hero */}
      <div className="card p-6 bg-gradient-to-r from-purple-900 to-indigo-900 text-white">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-purple-200 text-sm mb-2">Agent Fitness Formula</p>
            <div className="text-3xl font-mono font-bold">
              η = τ × (Revenue / Cost)
            </div>
            <p className="text-purple-200 text-sm mt-2">
              Where τ = TrustScore / 1000. Agents with η &lt; 0.7 for 2 epochs get culled.
            </p>
          </div>
          <div className="text-right">
            <p className="text-purple-200 text-sm">Target Efficiency</p>
            <p className="text-4xl font-bold">1.05</p>
            <p className="text-purple-200 text-xs">PID regulated</p>
          </div>
        </div>
      </div>

      {/* Classification Stats */}
      <div className="grid grid-cols-4 gap-6">
        <div className="stat-card border-l-4 border-green-500">
          <div className="flex items-center space-x-2 mb-2">
            <Trophy className="w-5 h-5 text-green-500" />
            <span className="text-sm text-gray-500">Champions</span>
          </div>
          <p className="metric-value text-green-600">{champions.length}</p>
          <p className="text-xs text-gray-500">η ≥ 1.0 — Get more HC</p>
        </div>
        <div className="stat-card border-l-4 border-yellow-500">
          <div className="flex items-center space-x-2 mb-2">
            <Activity className="w-5 h-5 text-yellow-500" />
            <span className="text-sm text-gray-500">Neutral</span>
          </div>
          <p className="metric-value text-yellow-600">{neutrals.length}</p>
          <p className="text-xs text-gray-500">0.7 ≤ η &lt; 1.0</p>
        </div>
        <div className="stat-card border-l-4 border-orange-500">
          <div className="flex items-center space-x-2 mb-2">
            <AlertTriangle className="w-5 h-5 text-orange-500" />
            <span className="text-sm text-gray-500">Underperformers</span>
          </div>
          <p className="metric-value text-orange-600">{underperformers.length}</p>
          <p className="text-xs text-gray-500">η &lt; 0.7 — At risk</p>
        </div>
        <div className="stat-card border-l-4 border-red-500">
          <div className="flex items-center space-x-2 mb-2">
            <Skull className="w-5 h-5 text-red-500" />
            <span className="text-sm text-gray-500">Culled</span>
          </div>
          <p className="metric-value text-red-600">{culled.length}</p>
          <p className="text-xs text-gray-500">η &lt; 0.7 for 2 epochs</p>
        </div>
      </div>

      {/* Leaderboard */}
      <div className="card">
        <div className="card-header flex justify-between items-center">
          <h2 className="font-semibold">Agent Fitness Leaderboard</h2>
          <div className="flex items-center gap-4">
            {isDemoMode && (
              <span className="text-xs text-actoris-600">Demo data</span>
            )}
            <span className="text-sm text-gray-500">
              Network Avg: <span className="font-mono font-semibold">{avgFitness.toFixed(2)}</span>
            </span>
          </div>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-gray-50">
              <tr>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Rank</th>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Agent</th>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">TrustScore (τ)</th>
                <th className="text-right text-xs font-medium text-gray-500 px-6 py-3">Revenue</th>
                <th className="text-right text-xs font-medium text-gray-500 px-6 py-3">Cost</th>
                <th className="text-center text-xs font-medium text-gray-500 px-6 py-3">Fitness (η)</th>
                <th className="text-center text-xs font-medium text-gray-500 px-6 py-3">Status</th>
                <th className="text-right text-xs font-medium text-gray-500 px-6 py-3">HC Allocation</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {sortedAgents.map((agent, index) => (
                <AgentRow key={agent.id} agent={agent} rank={index + 1} />
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Allocation Rules */}
      <div className="grid grid-cols-2 gap-8">
        <div className="card">
          <div className="card-header">
            <h2 className="font-semibold">PID Controller Settings</h2>
          </div>
          <div className="card-body space-y-4">
            <div className="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
              <span className="text-sm text-gray-600">Target Ratio</span>
              <span className="font-mono font-semibold">1.05</span>
            </div>
            <div className="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
              <span className="text-sm text-gray-600">Kp (Proportional)</span>
              <span className="font-mono">0.5</span>
            </div>
            <div className="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
              <span className="text-sm text-gray-600">Ki (Integral)</span>
              <span className="font-mono">0.1</span>
            </div>
            <div className="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
              <span className="text-sm text-gray-600">Kd (Derivative)</span>
              <span className="font-mono">0.05</span>
            </div>
            <div className="flex justify-between items-center p-3 bg-red-50 rounded-lg">
              <span className="text-sm text-red-700">Cull Threshold</span>
              <span className="font-mono text-red-700">η &lt; 0.7 × 2 epochs</span>
            </div>
          </div>
        </div>

        <div className="card">
          <div className="card-header">
            <h2 className="font-semibold">Anti-Abuse Mechanisms</h2>
          </div>
          <div className="card-body space-y-4">
            <div className="p-3 bg-gray-50 rounded-lg">
              <div className="flex items-center space-x-2 mb-1">
                <Zap className="w-4 h-4 text-yellow-500" />
                <span className="font-medium">HC Expiration</span>
              </div>
              <p className="text-sm text-gray-600">Credits expire after 30 days — use it or lose it</p>
            </div>
            <div className="p-3 bg-gray-50 rounded-lg">
              <div className="flex items-center space-x-2 mb-1">
                <Target className="w-4 h-4 text-red-500" />
                <span className="font-medium">Anti-Sybil (SyRA)</span>
              </div>
              <p className="text-sm text-gray-600">Stake requirements + behavioral heuristics prevent fake identities</p>
            </div>
            <div className="p-3 bg-gray-50 rounded-lg">
              <div className="flex items-center space-x-2 mb-1">
                <Activity className="w-4 h-4 text-purple-500" />
                <span className="font-medium">Collusion Detection</span>
              </div>
              <p className="text-sm text-gray-600">Graph analysis on ledger detects coordinated manipulation</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

function AgentRow({ agent, rank }: { agent: Agent; rank: number }) {
  const getRankBadge = (rank: number) => {
    if (rank === 1) return '🥇'
    if (rank === 2) return '🥈'
    if (rank === 3) return '🥉'
    return rank
  }

  return (
    <tr className={`hover:bg-gray-50 ${agent.status === 'culled' ? 'opacity-50' : ''}`}>
      <td className="px-6 py-4 text-center text-lg">
        {getRankBadge(rank)}
      </td>
      <td className="px-6 py-4">
        <div>
          <p className="font-medium">{agent.name}</p>
          <p className="text-xs text-gray-500 font-mono">{agent.id.slice(0, 12)}...</p>
        </div>
      </td>
      <td className="px-6 py-4">
        <div className="flex items-center space-x-2">
          <div className="w-16 bg-gray-200 rounded-full h-2">
            <div
              className="bg-blue-500 h-2 rounded-full"
              style={{ width: `${agent.trust_score.tau * 100}%` }}
            />
          </div>
          <span className="font-mono text-sm">{agent.trust_score.tau.toFixed(3)}</span>
        </div>
      </td>
      <td className="px-6 py-4 text-right font-mono text-sm text-green-600">
        {formatCurrency(agent.fitness.revenue)}
      </td>
      <td className="px-6 py-4 text-right font-mono text-sm text-red-600">
        {formatCurrency(agent.fitness.cost)}
      </td>
      <td className="px-6 py-4 text-center">
        <span className={`font-mono font-bold text-lg ${
          agent.fitness.eta >= 1.0 ? 'text-green-600' :
          agent.fitness.eta >= 0.7 ? 'text-yellow-600' : 'text-red-600'
        }`}>
          {agent.fitness.eta.toFixed(2)}
        </span>
      </td>
      <td className="px-6 py-4 text-center">
        <span className={`px-2 py-1 rounded-full text-xs font-medium ${getFitnessColor(agent.fitness.classification)}`}>
          {agent.fitness.classification}
        </span>
      </td>
      <td className="px-6 py-4 text-right">
        <span className="font-mono text-sm">{formatNumber(agent.fitness.hc_allocation)} HC</span>
      </td>
    </tr>
  )
}
