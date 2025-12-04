'use client'

import { useState } from 'react'
import {
  Sparkles, AlertTriangle, CheckCircle, XCircle,
  Lightbulb, Copy, Check, Code, ChevronDown, ChevronUp
} from 'lucide-react'
import type { ActionAugmentation, AugmentedMemory } from '@/lib/api'

interface ActionAugmentationProps {
  augmentation: ActionAugmentation | undefined
  isLoading: boolean
}

export function ActionAugmentationCard({ augmentation, isLoading }: ActionAugmentationProps) {
  const [showContext, setShowContext] = useState(false)
  const [copied, setCopied] = useState(false)

  if (isLoading) {
    return (
      <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-100 animate-pulse">
        <div className="h-6 w-48 bg-gray-200 rounded mb-4" />
        <div className="h-24 bg-gray-100 rounded-lg mb-4" />
        <div className="space-y-3">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="h-16 bg-gray-100 rounded" />
          ))}
        </div>
      </div>
    )
  }

  if (!augmentation) {
    return (
      <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-100">
        <div className="text-center py-8">
          <Sparkles className="h-12 w-12 text-gray-300 mx-auto mb-3" />
          <p className="text-gray-500">No augmentation available</p>
          <p className="text-sm text-gray-400 mt-1">
            Run a retrieval query to see action suggestions
          </p>
        </div>
      </div>
    )
  }

  const handleCopyContext = () => {
    navigator.clipboard.writeText(augmentation.context_string)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 bg-gradient-to-r from-amber-50 to-orange-50 border-b border-gray-100">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="p-2 bg-white rounded-lg shadow-sm">
              <Sparkles className="h-6 w-6 text-amber-600" />
            </div>
            <div>
              <h3 className="font-semibold text-gray-900">Action Augmentation</h3>
              <p className="text-xs text-gray-500">
                {augmentation.memories.length} relevant memories found
              </p>
            </div>
          </div>
          <div className="text-right">
            <div className="text-2xl font-bold text-amber-600">
              {(augmentation.confidence * 100).toFixed(0)}%
            </div>
            <div className="text-xs text-gray-500">Confidence</div>
          </div>
        </div>
      </div>

      {/* Warning if present */}
      {augmentation.warning && (
        <div className="mx-6 mt-4 p-3 bg-amber-50 border border-amber-200 rounded-lg flex items-start space-x-3">
          <AlertTriangle className="h-5 w-5 text-amber-600 flex-shrink-0 mt-0.5" />
          <p className="text-sm text-amber-800">{augmentation.warning}</p>
        </div>
      )}

      {/* Suggested Action */}
      {augmentation.suggested_action && (
        <div className="p-6 border-b border-gray-100">
          <div className="flex items-center space-x-2 mb-3">
            <Lightbulb className="h-5 w-5 text-amber-500" />
            <h4 className="font-semibold text-gray-900">Suggested Action</h4>
          </div>
          <div className="bg-gray-900 rounded-lg p-4 text-green-400 font-mono text-sm">
            {augmentation.suggested_action.action}
          </div>
          <p className="mt-3 text-sm text-gray-600">
            {augmentation.suggested_action.reasoning}
          </p>
          <div className="mt-2 flex items-center space-x-2">
            <span className="text-xs text-gray-500">
              Based on memories: {augmentation.suggested_action.supporting_memories.map(m => `#${m}`).join(', ')}
            </span>
            <span className="text-xs px-2 py-0.5 bg-green-100 text-green-700 rounded-full">
              {(augmentation.suggested_action.confidence * 100).toFixed(0)}% confident
            </span>
          </div>
        </div>
      )}

      {/* Relevant Memories */}
      <div className="p-6">
        <h4 className="font-semibold text-gray-900 mb-4">Relevant Memories</h4>
        <div className="space-y-3">
          {augmentation.memories.map((mem) => (
            <div
              key={mem.rank}
              className={`p-4 rounded-lg border ${
                mem.was_successful
                  ? 'bg-green-50 border-green-200'
                  : 'bg-red-50 border-red-200'
              }`}
            >
              <div className="flex items-start justify-between">
                <div className="flex items-center space-x-3">
                  <div className={`w-8 h-8 rounded-full flex items-center justify-center font-bold text-sm ${
                    mem.was_successful ? 'bg-green-200 text-green-700' : 'bg-red-200 text-red-700'
                  }`}>
                    #{mem.rank}
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      {mem.was_successful ? (
                        <CheckCircle className="h-4 w-4 text-green-600" />
                      ) : (
                        <XCircle className="h-4 w-4 text-red-600" />
                      )}
                      <code className="text-sm font-mono bg-white/50 px-2 py-0.5 rounded">
                        {mem.action}
                      </code>
                    </div>
                    <p className="text-xs text-gray-600 mt-1">{mem.directive}</p>
                  </div>
                </div>
                <div className="text-right">
                  <div className={`font-semibold ${
                    mem.was_successful ? 'text-green-600' : 'text-red-600'
                  }`}>
                    {(mem.relevance * 100).toFixed(0)}%
                  </div>
                  <div className="text-xs text-gray-500">{mem.usage_count} uses</div>
                </div>
              </div>
              <p className="mt-2 text-xs text-gray-600">{mem.outcome_summary}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Context String Toggle */}
      <div className="border-t border-gray-100">
        <button
          onClick={() => setShowContext(!showContext)}
          className="w-full px-6 py-3 flex items-center justify-between text-sm text-gray-600 hover:bg-gray-50 transition-colors"
        >
          <div className="flex items-center space-x-2">
            <Code className="h-4 w-4" />
            <span>View Context String for Prompt Injection</span>
          </div>
          {showContext ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
        </button>

        {showContext && (
          <div className="px-6 pb-6">
            <div className="relative">
              <pre className="bg-gray-900 text-gray-100 p-4 rounded-lg text-xs overflow-x-auto max-h-64 overflow-y-auto">
                {augmentation.context_string}
              </pre>
              <button
                onClick={handleCopyContext}
                className="absolute top-2 right-2 p-2 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
              >
                {copied ? (
                  <Check className="h-4 w-4 text-green-400" />
                ) : (
                  <Copy className="h-4 w-4 text-gray-300" />
                )}
              </button>
            </div>
            <p className="mt-2 text-xs text-gray-500">
              Inject this context into your agent's prompt to provide procedural guidance.
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
