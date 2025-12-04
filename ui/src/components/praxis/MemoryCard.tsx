'use client'

import { useState } from 'react'
import {
  CheckCircle, XCircle, AlertCircle, Clock, Eye,
  ChevronDown, ChevronUp, Zap, Target, Brain,
  MousePointer, Type, Navigation
} from 'lucide-react'
import type { PraxisMemory, RetrievedMemory } from '@/lib/api'
import { formatDistanceToNow } from 'date-fns'

interface MemoryCardProps {
  memory: PraxisMemory
  relevanceScore?: number
  rank?: number
  showDetails?: boolean
}

const outcomeConfig = {
  success: {
    icon: CheckCircle,
    color: 'text-green-600',
    bgColor: 'bg-green-50',
    borderColor: 'border-green-200',
    label: 'Success',
  },
  failure: {
    icon: XCircle,
    color: 'text-red-600',
    bgColor: 'bg-red-50',
    borderColor: 'border-red-200',
    label: 'Failed',
  },
  partial: {
    icon: AlertCircle,
    color: 'text-amber-600',
    bgColor: 'bg-amber-50',
    borderColor: 'border-amber-200',
    label: 'Partial',
  },
}

const sourceConfig = {
  agent_experience: { label: 'Agent Experience', color: 'bg-blue-100 text-blue-700' },
  human_demonstration: { label: 'Human Demo', color: 'bg-purple-100 text-purple-700' },
  agent_transfer: { label: 'Transferred', color: 'bg-amber-100 text-amber-700' },
  synthetic: { label: 'Synthetic', color: 'bg-gray-100 text-gray-700' },
}

const actionIcons: Record<string, typeof MousePointer> = {
  click: MousePointer,
  type: Type,
  navigate: Navigation,
}

export function MemoryCard({ memory, relevanceScore, rank, showDetails = false }: MemoryCardProps) {
  const [expanded, setExpanded] = useState(showDetails)
  const outcome = outcomeConfig[memory.outcome.type]
  const OutcomeIcon = outcome.icon
  const source = sourceConfig[memory.source]
  const ActionIcon = actionIcons[memory.action.action_type] || Zap

  return (
    <div className={`bg-white rounded-xl border ${outcome.borderColor} shadow-sm overflow-hidden`}>
      {/* Header */}
      <div className="p-4">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-3">
            {rank !== undefined && (
              <div className="flex items-center justify-center w-8 h-8 rounded-full bg-primary-100 text-primary-700 font-bold text-sm">
                #{rank + 1}
              </div>
            )}
            <div className={`p-2 rounded-lg ${outcome.bgColor}`}>
              <OutcomeIcon className={`h-5 w-5 ${outcome.color}`} />
            </div>
            <div>
              <div className="flex items-center space-x-2">
                <span className={`font-semibold ${outcome.color}`}>{outcome.label}</span>
                <span className={`text-xs px-2 py-0.5 rounded-full ${source.color}`}>
                  {source.label}
                </span>
              </div>
              <p className="text-sm text-gray-500 mt-0.5">
                {formatDistanceToNow(new Date(memory.created_at), { addSuffix: true })}
              </p>
            </div>
          </div>

          {relevanceScore !== undefined && (
            <div className="text-right">
              <div className="text-lg font-bold text-primary-600">
                {(relevanceScore * 100).toFixed(0)}%
              </div>
              <div className="text-xs text-gray-500">relevance</div>
            </div>
          )}
        </div>

        {/* Directive */}
        <div className="mt-4 p-3 bg-gray-50 rounded-lg">
          <div className="flex items-center space-x-2 text-xs text-gray-500 mb-1">
            <Target className="h-3 w-3" />
            <span>Directive</span>
          </div>
          <p className="text-sm font-medium text-gray-900">{memory.internal_state.directive}</p>
          {memory.internal_state.sub_task && (
            <p className="text-xs text-gray-500 mt-1">Sub-task: {memory.internal_state.sub_task}</p>
          )}
        </div>

        {/* Action */}
        <div className="mt-3 flex items-center space-x-3">
          <div className="p-2 bg-gray-100 rounded-lg">
            <ActionIcon className="h-4 w-4 text-gray-600" />
          </div>
          <div className="flex-1">
            <code className="text-sm bg-gray-100 px-2 py-1 rounded font-mono">
              {memory.action.raw_action}
            </code>
          </div>
        </div>

        {/* Outcome description */}
        <p className="mt-3 text-sm text-gray-600">
          {memory.outcome.description}
        </p>

        {/* Stats row */}
        <div className="mt-4 flex items-center space-x-4 text-xs text-gray-500">
          <div className="flex items-center space-x-1">
            <Eye className="h-3 w-3" />
            <span>{memory.retrieval_count} retrievals</span>
          </div>
          <div className="flex items-center space-x-1">
            <Zap className="h-3 w-3" />
            <span>Score: {memory.reinforcement_score.toFixed(2)}</span>
          </div>
          {memory.internal_state.task_tags.length > 0 && (
            <div className="flex items-center space-x-1">
              {memory.internal_state.task_tags.slice(0, 2).map(tag => (
                <span key={tag} className="bg-gray-100 px-1.5 py-0.5 rounded">
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Expand button */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-4 py-2 bg-gray-50 border-t border-gray-100 flex items-center justify-center space-x-2 text-sm text-gray-600 hover:bg-gray-100 transition-colors"
      >
        {expanded ? (
          <>
            <ChevronUp className="h-4 w-4" />
            <span>Hide Details</span>
          </>
        ) : (
          <>
            <ChevronDown className="h-4 w-4" />
            <span>Show Details</span>
          </>
        )}
      </button>

      {/* Expanded details */}
      {expanded && (
        <div className="px-4 pb-4 border-t border-gray-100 bg-gray-50/50">
          <div className="grid grid-cols-2 gap-4 mt-4">
            {/* Pre-state */}
            <div>
              <h4 className="text-xs font-semibold text-gray-500 uppercase mb-2">
                Environment Before
              </h4>
              <div className="bg-white p-3 rounded-lg border border-gray-200 text-xs">
                <p className="text-gray-600 line-clamp-3">{memory.env_state_pre.textual_repr || 'No description'}</p>
                {Object.keys(memory.env_state_pre.state_features).length > 0 && (
                  <div className="mt-2 space-y-1">
                    {Object.entries(memory.env_state_pre.state_features).slice(0, 3).map(([k, v]) => (
                      <div key={k} className="flex justify-between">
                        <span className="text-gray-400">{k}:</span>
                        <span className="text-gray-600 truncate ml-2">{v}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Post-state */}
            <div>
              <h4 className="text-xs font-semibold text-gray-500 uppercase mb-2">
                Environment After
              </h4>
              <div className="bg-white p-3 rounded-lg border border-gray-200 text-xs">
                <p className="text-gray-600 line-clamp-3">{memory.env_state_post.textual_repr || 'No description'}</p>
                {Object.keys(memory.env_state_post.state_features).length > 0 && (
                  <div className="mt-2 space-y-1">
                    {Object.entries(memory.env_state_post.state_features).slice(0, 3).map(([k, v]) => (
                      <div key={k} className="flex justify-between">
                        <span className="text-gray-400">{k}:</span>
                        <span className="text-gray-600 truncate ml-2">{v}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Memory ID */}
          <div className="mt-4 text-xs text-gray-400">
            Memory ID: <code className="bg-gray-100 px-1 rounded">{memory.id}</code>
          </div>
        </div>
      )}
    </div>
  )
}

export function MemoryCardSkeleton() {
  return (
    <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 animate-pulse">
      <div className="flex items-center space-x-3">
        <div className="w-10 h-10 bg-gray-200 rounded-lg" />
        <div>
          <div className="h-4 w-24 bg-gray-200 rounded" />
          <div className="h-3 w-16 bg-gray-200 rounded mt-1" />
        </div>
      </div>
      <div className="mt-4 h-16 bg-gray-100 rounded-lg" />
      <div className="mt-3 h-8 bg-gray-100 rounded" />
    </div>
  )
}
