'use client'

import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Bot, User, Cpu, FileCode, Plus, Search, Wallet, Send } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'
import { apiClient } from '@/lib/api'
import { TrustBadge } from '@/components/TrustBadge'

const agentTypeIcons = {
  human: User,
  ai: Bot,
  hybrid: Cpu,
  contract: FileCode,
}

export default function AgentsPage() {
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [newAgentName, setNewAgentName] = useState('')
  const [newAgentType, setNewAgentType] = useState<'human' | 'ai' | 'hybrid' | 'contract'>('ai')
  const [depositAmount, setDepositAmount] = useState('')
  const [searchQuery, setSearchQuery] = useState('')

  const queryClient = useQueryClient()

  const { data: agents, isLoading } = useQuery({
    queryKey: ['agents'],
    queryFn: apiClient.getAgents,
  })

  const { data: selectedWallet } = useQuery({
    queryKey: ['wallet', selectedAgent],
    queryFn: () => selectedAgent ? apiClient.getWallet(selectedAgent) : null,
    enabled: !!selectedAgent,
  })

  const { data: selectedActions } = useQuery({
    queryKey: ['agent-actions', selectedAgent],
    queryFn: () => selectedAgent ? apiClient.getAgentActions(selectedAgent) : [],
    enabled: !!selectedAgent,
  })

  const createAgentMutation = useMutation({
    mutationFn: apiClient.createAgent,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] })
      setShowCreateModal(false)
      setNewAgentName('')
    },
  })

  const depositMutation = useMutation({
    mutationFn: ({ agentId, amount }: { agentId: string; amount: string }) =>
      apiClient.deposit(agentId, amount),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['wallet', selectedAgent] })
      setDepositAmount('')
    },
  })

  const filteredAgents = agents?.filter((agent) =>
    agent.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    agent.id.toLowerCase().includes(searchQuery.toLowerCase())
  ) || []

  const selectedAgentData = agents?.find((a) => a.id === selectedAgent)

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Agents</h1>
          <p className="mt-1 text-gray-500">Manage your agent identities and trust scores</p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="inline-flex items-center px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
        >
          <Plus className="h-4 w-4 mr-2" />
          Create Agent
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Agent List */}
        <div className="lg:col-span-2 bg-white rounded-xl shadow-sm p-6">
          {/* Search */}
          <div className="relative mb-6">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
            <input
              type="text"
              placeholder="Search agents..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
            />
          </div>

          {isLoading ? (
            <div className="space-y-4">
              {[1, 2, 3].map((i) => (
                <div key={i} className="animate-pulse flex items-center space-x-4 p-4 border rounded-lg">
                  <div className="h-12 w-12 bg-gray-200 rounded-full" />
                  <div className="flex-1">
                    <div className="h-4 w-32 bg-gray-200 rounded mb-2" />
                    <div className="h-3 w-24 bg-gray-200 rounded" />
                  </div>
                </div>
              ))}
            </div>
          ) : filteredAgents.length === 0 ? (
            <div className="text-center py-12">
              <Bot className="h-12 w-12 text-gray-400 mx-auto mb-4" />
              <p className="text-gray-500">
                {searchQuery ? 'No agents found matching your search' : 'No agents yet. Create one to get started!'}
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {filteredAgents.map((agent) => {
                const Icon = agentTypeIcons[agent.agent_type] || Bot
                const isSelected = selectedAgent === agent.id

                return (
                  <div
                    key={agent.id}
                    onClick={() => setSelectedAgent(agent.id)}
                    className={`flex items-center justify-between p-4 rounded-lg border cursor-pointer transition-all ${
                      isSelected
                        ? 'border-primary-500 bg-primary-50 ring-2 ring-primary-200'
                        : 'border-gray-200 hover:border-primary-300 hover:bg-gray-50'
                    }`}
                  >
                    <div className="flex items-center space-x-4">
                      <div className={`p-3 rounded-full ${
                        isSelected ? 'bg-primary-200' : 'bg-gray-100'
                      }`}>
                        <Icon className={`h-5 w-5 ${isSelected ? 'text-primary-700' : 'text-gray-600'}`} />
                      </div>
                      <div>
                        <h3 className="font-semibold text-gray-900">{agent.name}</h3>
                        <p className="text-sm text-gray-500">
                          <span className="font-mono">{agent.id.slice(0, 8)}...</span>
                          <span className="mx-2">·</span>
                          {agent.agent_type}
                          <span className="mx-2">·</span>
                          {formatDistanceToNow(new Date(agent.created_at), { addSuffix: true })}
                        </p>
                      </div>
                    </div>
                    <TrustBadge trustScore={agent.trust_score} />
                  </div>
                )
              })}
            </div>
          )}
        </div>

        {/* Agent Details */}
        <div className="space-y-6">
          {selectedAgentData ? (
            <>
              {/* Trust Score */}
              <TrustBadge trustScore={selectedAgentData.trust_score} showDetails />

              {/* Wallet */}
              <div className="bg-white rounded-xl shadow-sm p-6">
                <h3 className="font-semibold text-gray-900 mb-4 flex items-center">
                  <Wallet className="h-5 w-5 mr-2 text-primary-600" />
                  Wallet
                </h3>
                {selectedWallet ? (
                  <>
                    <div className="space-y-3">
                      <div className="flex justify-between">
                        <span className="text-gray-500">Balance</span>
                        <span className="font-bold text-gray-900">
                          {parseFloat(selectedWallet.balance).toFixed(2)} HC
                        </span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-500">Locked</span>
                        <span className="font-medium text-amber-600">
                          {parseFloat(selectedWallet.locked).toFixed(2)} HC
                        </span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-500">Pending</span>
                        <span className="font-medium text-blue-600">
                          {parseFloat(selectedWallet.pending).toFixed(2)} HC
                        </span>
                      </div>
                    </div>

                    {/* Deposit */}
                    <div className="mt-4 pt-4 border-t">
                      <form
                        onSubmit={(e) => {
                          e.preventDefault()
                          if (depositAmount && selectedAgent) {
                            depositMutation.mutate({
                              agentId: selectedAgent,
                              amount: depositAmount,
                            })
                          }
                        }}
                        className="flex space-x-2"
                      >
                        <input
                          type="number"
                          step="0.01"
                          placeholder="Amount"
                          value={depositAmount}
                          onChange={(e) => setDepositAmount(e.target.value)}
                          className="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm"
                        />
                        <button
                          type="submit"
                          disabled={depositMutation.isPending}
                          className="px-4 py-2 bg-green-600 text-white rounded-lg text-sm hover:bg-green-700 disabled:opacity-50"
                        >
                          {depositMutation.isPending ? '...' : 'Deposit'}
                        </button>
                      </form>
                    </div>
                  </>
                ) : (
                  <p className="text-gray-500 text-sm">Loading wallet...</p>
                )}
              </div>

              {/* Recent Actions */}
              <div className="bg-white rounded-xl shadow-sm p-6">
                <h3 className="font-semibold text-gray-900 mb-4 flex items-center">
                  <Send className="h-5 w-5 mr-2 text-primary-600" />
                  Recent Actions
                </h3>
                {selectedActions && selectedActions.length > 0 ? (
                  <div className="space-y-2">
                    {selectedActions.slice(0, 5).map((action) => (
                      <div key={action.id} className="p-2 bg-gray-50 rounded text-sm">
                        <div className="flex justify-between">
                          <span className="font-mono text-gray-600">{action.id.slice(0, 8)}...</span>
                          <span className={`px-2 py-0.5 rounded-full text-xs ${
                            action.status === 'verified' ? 'bg-green-100 text-green-700' :
                            action.status === 'pending' ? 'bg-gray-100 text-gray-700' :
                            'bg-amber-100 text-amber-700'
                          }`}>
                            {action.status}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-gray-500 text-sm">No actions yet</p>
                )}
              </div>
            </>
          ) : (
            <div className="bg-white rounded-xl shadow-sm p-6 text-center">
              <Bot className="h-12 w-12 text-gray-400 mx-auto mb-4" />
              <p className="text-gray-500">Select an agent to view details</p>
            </div>
          )}
        </div>
      </div>

      {/* Create Agent Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl shadow-xl w-full max-w-md mx-4">
            <div className="p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">Create New Agent</h3>
              <form
                onSubmit={(e) => {
                  e.preventDefault()
                  createAgentMutation.mutate({
                    name: newAgentName,
                    agent_type: newAgentType,
                  })
                }}
                className="space-y-4"
              >
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Name</label>
                  <input
                    type="text"
                    value={newAgentName}
                    onChange={(e) => setNewAgentName(e.target.value)}
                    placeholder="Enter agent name"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg"
                    required
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">Type</label>
                  <div className="grid grid-cols-2 gap-2">
                    {(['human', 'ai', 'hybrid', 'contract'] as const).map((type) => (
                      <button
                        key={type}
                        type="button"
                        onClick={() => setNewAgentType(type)}
                        className={`px-4 py-2 rounded-lg border text-sm ${
                          newAgentType === type
                            ? 'bg-primary-100 border-primary-500 text-primary-700'
                            : 'border-gray-300 hover:bg-gray-50'
                        }`}
                      >
                        {type.charAt(0).toUpperCase() + type.slice(1)}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="flex space-x-3 pt-4">
                  <button
                    type="button"
                    onClick={() => setShowCreateModal(false)}
                    className="flex-1 px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={createAgentMutation.isPending}
                    className="flex-1 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50"
                  >
                    {createAgentMutation.isPending ? 'Creating...' : 'Create'}
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
