'use client'

import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient, Agent } from '@/lib/api'
import { useDemoMode } from '@/lib/demo-context'
import { formatNumber, getTrustTier, getFitnessColor, truncateId } from '@/lib/utils'
import { Users, Bot, Building2, User, Plus, Search, Shield, Wallet } from 'lucide-react'

export default function IdentityCloudPage() {
  const [filter, setFilter] = useState<'all' | 'human' | 'agent' | 'organization'>('all')
  const [search, setSearch] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const queryClient = useQueryClient()
  const { isDemoMode, mockAgents } = useDemoMode()

  const { data: liveAgents, isLoading: liveLoading } = useQuery({
    queryKey: ['agents'],
    queryFn: apiClient.getAgents,
    enabled: !isDemoMode,
  })

  const agents = isDemoMode ? mockAgents : liveAgents
  const isLoading = !isDemoMode && liveLoading

  const createMutation = useMutation({
    mutationFn: (data: { name: string; type: string }) => apiClient.createAgent(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] })
      setShowCreate(false)
    },
  })

  const filteredAgents = agents?.filter(agent => {
    if (filter !== 'all' && agent.type !== filter) return false
    if (search && !agent.name.toLowerCase().includes(search.toLowerCase())) return false
    return true
  })

  const stats = {
    total: agents?.length || 0,
    humans: agents?.filter(a => a.type === 'human').length || 0,
    agents: agents?.filter(a => a.type === 'agent').length || 0,
    organizations: agents?.filter(a => a.type === 'organization').length || 0,
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex justify-between items-start">
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-3xl font-bold text-gray-900">IdentityCloud</h1>
            {isDemoMode && (
              <span className="px-2 py-1 text-xs font-medium bg-actoris-100 text-actoris-700 rounded-full">
                Demo Mode
              </span>
            )}
          </div>
          <p className="text-gray-500 mt-1">
            UnifiedID Registry — Every entity gets a DID, TrustScore (0-1000), and HC Wallet
          </p>
        </div>
        <button
          onClick={() => setShowCreate(true)}
          className="btn-primary flex items-center space-x-2"
          disabled={isDemoMode}
          title={isDemoMode ? 'Disable demo mode to create entities' : undefined}
        >
          <Plus className="w-4 h-4" />
          <span>Create Entity</span>
        </button>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-4 gap-6">
        <StatCard
          title="Total Entities"
          value={stats.total}
          icon={<Users className="w-5 h-5" />}
          color="blue"
        />
        <StatCard
          title="Humans"
          value={stats.humans}
          icon={<User className="w-5 h-5" />}
          color="green"
        />
        <StatCard
          title="AI Agents"
          value={stats.agents}
          icon={<Bot className="w-5 h-5" />}
          color="purple"
        />
        <StatCard
          title="Organizations"
          value={stats.organizations}
          icon={<Building2 className="w-5 h-5" />}
          color="orange"
        />
      </div>

      {/* Filters & Search */}
      <div className="card p-4 flex items-center justify-between">
        <div className="flex space-x-2">
          {(['all', 'human', 'agent', 'organization'] as const).map((f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                filter === f
                  ? 'bg-actoris-500 text-white'
                  : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
              }`}
            >
              {f.charAt(0).toUpperCase() + f.slice(1)}
            </button>
          ))}
        </div>
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
          <input
            type="text"
            placeholder="Search entities..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-10 pr-4 py-2 border rounded-lg focus:ring-2 focus:ring-actoris-500 focus:border-transparent"
          />
        </div>
      </div>

      {/* Entities Grid */}
      {isLoading ? (
        <div className="grid grid-cols-2 gap-6">
          {[1, 2, 3, 4].map(i => (
            <div key={i} className="card h-48 animate-pulse bg-gray-100" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-6">
          {filteredAgents?.map((agent) => (
            <EntityCard key={agent.id} agent={agent} />
          ))}
        </div>
      )}

      {/* Create Modal */}
      {showCreate && !isDemoMode && (
        <CreateEntityModal
          onClose={() => setShowCreate(false)}
          onSubmit={(data) => createMutation.mutate(data)}
          isLoading={createMutation.isPending}
        />
      )}
    </div>
  )
}

function StatCard({ title, value, icon, color }: {
  title: string
  value: number
  icon: React.ReactNode
  color: 'blue' | 'green' | 'purple' | 'orange'
}) {
  const colors = {
    blue: 'bg-blue-50 text-blue-600',
    green: 'bg-green-50 text-green-600',
    purple: 'bg-purple-50 text-purple-600',
    orange: 'bg-actoris-50 text-actoris-600',
  }

  return (
    <div className="stat-card">
      <div className={`inline-flex p-2 rounded-lg ${colors[color]}`}>{icon}</div>
      <p className="metric-value mt-3">{value}</p>
      <p className="metric-label">{title}</p>
    </div>
  )
}

function EntityCard({ agent }: { agent: Agent }) {
  const trustTier = getTrustTier(agent.trust_score.score)
  const typeIcons = {
    human: <User className="w-5 h-5" />,
    agent: <Bot className="w-5 h-5" />,
    organization: <Building2 className="w-5 h-5" />,
  }

  return (
    <div className="card hover:shadow-md transition-shadow">
      <div className="p-6">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-3">
            <div className={`p-2 rounded-lg ${
              agent.type === 'human' ? 'bg-green-50 text-green-600' :
              agent.type === 'agent' ? 'bg-purple-50 text-purple-600' :
              'bg-actoris-50 text-actoris-600'
            }`}>
              {typeIcons[agent.type]}
            </div>
            <div>
              <h3 className="font-semibold text-gray-900">{agent.name}</h3>
              <p className="text-xs text-gray-500 font-mono">{truncateId(agent.id, 12)}</p>
            </div>
          </div>
          <span className={`px-2 py-1 rounded-full text-xs font-medium ${
            agent.status === 'active' ? 'bg-green-100 text-green-700' :
            agent.status === 'warning' ? 'bg-yellow-100 text-yellow-700' :
            'bg-red-100 text-red-700'
          }`}>
            {agent.status}
          </span>
        </div>

        <div className="grid grid-cols-3 gap-4 mt-6">
          {/* TrustScore */}
          <div className="text-center p-3 bg-gray-50 rounded-lg">
            <div className="flex items-center justify-center space-x-1 mb-1">
              <Shield className="w-4 h-4 text-gray-400" />
              <span className="text-xs text-gray-500">TrustScore</span>
            </div>
            <p className={`text-xl font-bold ${trustTier.color}`}>
              {agent.trust_score.score}
            </p>
            <p className="text-xs text-gray-500">Tier {trustTier.tier} · {trustTier.label}</p>
          </div>

          {/* HC Wallet */}
          <div className="text-center p-3 bg-gray-50 rounded-lg">
            <div className="flex items-center justify-center space-x-1 mb-1">
              <Wallet className="w-4 h-4 text-gray-400" />
              <span className="text-xs text-gray-500">HC Balance</span>
            </div>
            <p className="text-xl font-bold text-gray-900">
              {formatNumber(agent.wallet.balance)}
            </p>
            <p className="text-xs text-gray-500">PFLOP-hours</p>
          </div>

          {/* Fitness */}
          <div className="text-center p-3 bg-gray-50 rounded-lg">
            <div className="flex items-center justify-center space-x-1 mb-1">
              <span className="text-xs text-gray-500">Fitness η</span>
            </div>
            <p className={`text-xl font-bold ${
              agent.fitness.eta >= 1.0 ? 'text-green-600' :
              agent.fitness.eta >= 0.7 ? 'text-yellow-600' : 'text-red-600'
            }`}>
              {agent.fitness.eta.toFixed(2)}
            </p>
            <p className={`text-xs px-2 py-0.5 rounded-full inline-block ${getFitnessColor(agent.fitness.classification)}`}>
              {agent.fitness.classification}
            </p>
          </div>
        </div>

        <div className="mt-4 pt-4 border-t border-gray-100 flex justify-between text-xs text-gray-500">
          <span>{agent.trust_score.verifications} verifications</span>
          <span>{agent.trust_score.disputes} disputes</span>
        </div>
      </div>
    </div>
  )
}

function CreateEntityModal({ onClose, onSubmit, isLoading }: {
  onClose: () => void
  onSubmit: (data: { name: string; type: string }) => void
  isLoading: boolean
}) {
  const [name, setName] = useState('')
  const [type, setType] = useState<'human' | 'agent' | 'organization'>('agent')

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-xl p-6 w-full max-w-md">
        <h2 className="text-xl font-bold mb-4">Create New Entity</h2>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-actoris-500"
              placeholder="Enter entity name"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Type</label>
            <div className="grid grid-cols-3 gap-2">
              {(['human', 'agent', 'organization'] as const).map((t) => (
                <button
                  key={t}
                  onClick={() => setType(t)}
                  className={`p-3 rounded-lg border text-sm font-medium transition-colors ${
                    type === t
                      ? 'border-actoris-500 bg-actoris-50 text-actoris-600'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  {t.charAt(0).toUpperCase() + t.slice(1)}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="flex space-x-3 mt-6">
          <button onClick={onClose} className="btn-secondary flex-1">Cancel</button>
          <button
            onClick={() => onSubmit({ name, type })}
            disabled={!name || isLoading}
            className="btn-primary flex-1"
          >
            {isLoading ? 'Creating...' : 'Create Entity'}
          </button>
        </div>
      </div>
    </div>
  )
}
