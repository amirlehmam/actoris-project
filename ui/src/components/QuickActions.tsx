'use client'

import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Plus, Send, Wallet, X } from 'lucide-react'
import { apiClient } from '@/lib/api'

export function QuickActions() {
  const [showCreateAgent, setShowCreateAgent] = useState(false)
  const [agentName, setAgentName] = useState('')
  const [agentType, setAgentType] = useState<'human' | 'ai' | 'hybrid' | 'contract'>('ai')

  const queryClient = useQueryClient()

  const createAgentMutation = useMutation({
    mutationFn: apiClient.createAgent,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] })
      queryClient.invalidateQueries({ queryKey: ['stats'] })
      setShowCreateAgent(false)
      setAgentName('')
    },
  })

  const handleCreateAgent = (e: React.FormEvent) => {
    e.preventDefault()
    if (!agentName.trim()) return
    createAgentMutation.mutate({
      name: agentName,
      agent_type: agentType,
    })
  }

  return (
    <>
      <div className="flex space-x-3">
        <button
          onClick={() => setShowCreateAgent(true)}
          className="inline-flex items-center px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
        >
          <Plus className="h-4 w-4 mr-2" />
          New Agent
        </button>
        <a
          href="/actions?new=true"
          className="inline-flex items-center px-4 py-2 bg-white border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 transition-colors"
        >
          <Send className="h-4 w-4 mr-2" />
          Submit Action
        </a>
      </div>

      {/* Create Agent Modal */}
      {showCreateAgent && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl shadow-xl w-full max-w-md mx-4">
            <div className="flex items-center justify-between p-4 border-b">
              <h3 className="text-lg font-semibold text-gray-900">Create New Agent</h3>
              <button
                onClick={() => setShowCreateAgent(false)}
                className="text-gray-400 hover:text-gray-600"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <form onSubmit={handleCreateAgent} className="p-4 space-y-4">
              <div>
                <label htmlFor="name" className="block text-sm font-medium text-gray-700 mb-1">
                  Agent Name
                </label>
                <input
                  type="text"
                  id="name"
                  value={agentName}
                  onChange={(e) => setAgentName(e.target.value)}
                  placeholder="Enter agent name"
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                  required
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Agent Type
                </label>
                <div className="grid grid-cols-2 gap-2">
                  {(['human', 'ai', 'hybrid', 'contract'] as const).map((type) => (
                    <button
                      key={type}
                      type="button"
                      onClick={() => setAgentType(type)}
                      className={`px-4 py-2 rounded-lg border text-sm font-medium transition-colors ${
                        agentType === type
                          ? 'bg-primary-100 border-primary-500 text-primary-700'
                          : 'bg-white border-gray-300 text-gray-700 hover:bg-gray-50'
                      }`}
                    >
                      {type.charAt(0).toUpperCase() + type.slice(1)}
                    </button>
                  ))}
                </div>
              </div>

              <div className="bg-gray-50 p-3 rounded-lg text-sm text-gray-600">
                <p className="font-medium text-gray-700 mb-1">Initial Trust Score</p>
                <p>New agents start with a neutral trust score (τ = 0.5, Tier 1). Complete verifications to increase your score!</p>
              </div>

              <div className="flex space-x-3 pt-2">
                <button
                  type="button"
                  onClick={() => setShowCreateAgent(false)}
                  className="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={createAgentMutation.isPending}
                  className="flex-1 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50"
                >
                  {createAgentMutation.isPending ? 'Creating...' : 'Create Agent'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </>
  )
}
