'use client'

import { Bot, User, Cpu, FileCode } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'
import type { Agent } from '@/lib/api'
import { TrustBadge } from './TrustBadge'

interface AgentsListProps {
  agents: Agent[]
  isLoading: boolean
}

const agentTypeIcons = {
  human: User,
  ai: Bot,
  hybrid: Cpu,
  contract: FileCode,
}

const agentTypeColors = {
  human: 'bg-blue-100 text-blue-700',
  ai: 'bg-purple-100 text-purple-700',
  hybrid: 'bg-green-100 text-green-700',
  contract: 'bg-amber-100 text-amber-700',
}

export function AgentsList({ agents, isLoading }: AgentsListProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-xl shadow-sm p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Active Agents</h2>
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
      </div>
    )
  }

  return (
    <div className="bg-white rounded-xl shadow-sm p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-900">Active Agents</h2>
        <a href="/agents" className="text-primary-600 hover:text-primary-700 text-sm font-medium">
          View all
        </a>
      </div>

      {agents.length === 0 ? (
        <div className="text-center py-12">
          <Bot className="h-12 w-12 text-gray-400 mx-auto mb-4" />
          <p className="text-gray-500">No agents yet. Create your first agent to get started!</p>
        </div>
      ) : (
        <div className="space-y-3">
          {agents.slice(0, 5).map((agent) => {
            const Icon = agentTypeIcons[agent.agent_type] || Bot
            const colorClass = agentTypeColors[agent.agent_type] || 'bg-gray-100 text-gray-700'

            return (
              <div
                key={agent.id}
                className="flex items-center justify-between p-4 border border-gray-100 rounded-lg hover:border-primary-200 hover:bg-primary-50/30 transition-colors cursor-pointer"
              >
                <div className="flex items-center space-x-4">
                  <div className={`p-2.5 rounded-full ${colorClass}`}>
                    <Icon className="h-5 w-5" />
                  </div>
                  <div>
                    <h3 className="font-medium text-gray-900">{agent.name}</h3>
                    <p className="text-sm text-gray-500">
                      {agent.agent_type.charAt(0).toUpperCase() + agent.agent_type.slice(1)} Agent
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
  )
}
