'use client'

import { useState } from 'react'
import {
  Search, Play, Loader2, Settings, Target, Brain,
  Plus, X, Tag
} from 'lucide-react'
import type { EnvironmentState, InternalState, RetrievedMemory, Agent } from '@/lib/api'

interface RetrievalDemoProps {
  agents: Agent[]
  onRetrieve: (data: {
    agent_id: string
    current_env: EnvironmentState
    current_internal: InternalState
    max_results?: number
    similarity_threshold?: number
  }) => Promise<{ memories: RetrievedMemory[]; total_searched: number; search_time_ms: number }>
  isLoading: boolean
  results?: { memories: RetrievedMemory[]; total_searched: number; search_time_ms: number }
}

export function RetrievalDemo({ agents, onRetrieve, isLoading, results }: RetrievalDemoProps) {
  const [selectedAgent, setSelectedAgent] = useState<string>('')
  const [directive, setDirective] = useState('')
  const [subTask, setSubTask] = useState('')
  const [envText, setEnvText] = useState('')
  const [features, setFeatures] = useState<Record<string, string>>({})
  const [newFeatureKey, setNewFeatureKey] = useState('')
  const [newFeatureValue, setNewFeatureValue] = useState('')
  const [tags, setTags] = useState<string[]>([])
  const [newTag, setNewTag] = useState('')
  const [maxResults, setMaxResults] = useState(10)
  const [threshold, setThreshold] = useState(0.3)
  const [showAdvanced, setShowAdvanced] = useState(false)

  const addFeature = () => {
    if (newFeatureKey && newFeatureValue) {
      setFeatures({ ...features, [newFeatureKey]: newFeatureValue })
      setNewFeatureKey('')
      setNewFeatureValue('')
    }
  }

  const removeFeature = (key: string) => {
    const { [key]: _, ...rest } = features
    setFeatures(rest)
  }

  const addTag = () => {
    if (newTag && !tags.includes(newTag)) {
      setTags([...tags, newTag])
      setNewTag('')
    }
  }

  const removeTag = (tag: string) => {
    setTags(tags.filter(t => t !== tag))
  }

  const handleRetrieve = async () => {
    if (!selectedAgent || !directive) return

    const currentEnv: EnvironmentState = {
      textual_repr: envText,
      state_features: features,
      element_ids: [],
      captured_at: new Date().toISOString(),
    }

    const currentInternal: InternalState = {
      directive,
      sub_task: subTask || undefined,
      progress: 0,
      task_tags: tags,
    }

    await onRetrieve({
      agent_id: selectedAgent,
      current_env: currentEnv,
      current_internal: currentInternal,
      max_results: maxResults,
      similarity_threshold: threshold,
    })
  }

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 bg-gradient-to-r from-blue-50 to-indigo-50 border-b border-gray-100">
        <div className="flex items-center space-x-3">
          <div className="p-2 bg-white rounded-lg shadow-sm">
            <Search className="h-6 w-6 text-blue-600" />
          </div>
          <div>
            <h3 className="font-semibold text-gray-900">Memory Retrieval Demo</h3>
            <p className="text-xs text-gray-500">
              Test the PRAXIS retrieval algorithm with custom queries
            </p>
          </div>
        </div>
      </div>

      <div className="p-6 space-y-6">
        {/* Agent Selection */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Select Agent
          </label>
          <select
            value={selectedAgent}
            onChange={(e) => setSelectedAgent(e.target.value)}
            className="w-full px-4 py-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
          >
            <option value="">Choose an agent...</option>
            {agents.map((agent) => (
              <option key={agent.id} value={agent.id}>
                {agent.name} ({agent.agent_type})
              </option>
            ))}
          </select>
        </div>

        {/* Internal State */}
        <div className="p-4 bg-purple-50 rounded-xl border border-purple-100">
          <div className="flex items-center space-x-2 mb-3">
            <Brain className="h-5 w-5 text-purple-600" />
            <h4 className="font-medium text-purple-900">Internal State</h4>
          </div>

          <div className="space-y-3">
            <div>
              <label className="block text-xs text-purple-700 mb-1">Directive *</label>
              <input
                type="text"
                value={directive}
                onChange={(e) => setDirective(e.target.value)}
                placeholder="e.g., Complete user registration"
                className="w-full px-3 py-2 border border-purple-200 rounded-lg text-sm focus:ring-2 focus:ring-purple-500 focus:border-purple-500 bg-white"
              />
            </div>

            <div>
              <label className="block text-xs text-purple-700 mb-1">Sub-task (optional)</label>
              <input
                type="text"
                value={subTask}
                onChange={(e) => setSubTask(e.target.value)}
                placeholder="e.g., Fill in email field"
                className="w-full px-3 py-2 border border-purple-200 rounded-lg text-sm focus:ring-2 focus:ring-purple-500 focus:border-purple-500 bg-white"
              />
            </div>

            {/* Tags */}
            <div>
              <label className="block text-xs text-purple-700 mb-1">Task Tags</label>
              <div className="flex flex-wrap gap-2 mb-2">
                {tags.map((tag) => (
                  <span key={tag} className="inline-flex items-center space-x-1 px-2 py-1 bg-purple-100 text-purple-700 rounded-full text-xs">
                    <span>{tag}</span>
                    <button onClick={() => removeTag(tag)} className="hover:text-purple-900">
                      <X className="h-3 w-3" />
                    </button>
                  </span>
                ))}
              </div>
              <div className="flex space-x-2">
                <input
                  type="text"
                  value={newTag}
                  onChange={(e) => setNewTag(e.target.value)}
                  placeholder="Add tag..."
                  className="flex-1 px-3 py-1.5 border border-purple-200 rounded-lg text-xs bg-white"
                  onKeyPress={(e) => e.key === 'Enter' && addTag()}
                />
                <button
                  onClick={addTag}
                  className="px-3 py-1.5 bg-purple-600 text-white rounded-lg text-xs hover:bg-purple-700"
                >
                  <Plus className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Environment State */}
        <div className="p-4 bg-blue-50 rounded-xl border border-blue-100">
          <div className="flex items-center space-x-2 mb-3">
            <Target className="h-5 w-5 text-blue-600" />
            <h4 className="font-medium text-blue-900">Environment State</h4>
          </div>

          <div className="space-y-3">
            <div>
              <label className="block text-xs text-blue-700 mb-1">State Description</label>
              <textarea
                value={envText}
                onChange={(e) => setEnvText(e.target.value)}
                placeholder="Describe the current environment state..."
                rows={3}
                className="w-full px-3 py-2 border border-blue-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-white"
              />
            </div>

            {/* Features */}
            <div>
              <label className="block text-xs text-blue-700 mb-1">State Features</label>
              <div className="space-y-2 mb-2">
                {Object.entries(features).map(([key, value]) => (
                  <div key={key} className="flex items-center space-x-2 text-xs">
                    <span className="px-2 py-1 bg-blue-100 rounded text-blue-700">{key}</span>
                    <span className="text-gray-500">=</span>
                    <span className="px-2 py-1 bg-white border border-blue-200 rounded flex-1 truncate">{value}</span>
                    <button onClick={() => removeFeature(key)} className="text-red-500 hover:text-red-700">
                      <X className="h-4 w-4" />
                    </button>
                  </div>
                ))}
              </div>
              <div className="flex space-x-2">
                <input
                  type="text"
                  value={newFeatureKey}
                  onChange={(e) => setNewFeatureKey(e.target.value)}
                  placeholder="Key"
                  className="w-1/3 px-3 py-1.5 border border-blue-200 rounded-lg text-xs bg-white"
                />
                <input
                  type="text"
                  value={newFeatureValue}
                  onChange={(e) => setNewFeatureValue(e.target.value)}
                  placeholder="Value"
                  className="flex-1 px-3 py-1.5 border border-blue-200 rounded-lg text-xs bg-white"
                />
                <button
                  onClick={addFeature}
                  className="px-3 py-1.5 bg-blue-600 text-white rounded-lg text-xs hover:bg-blue-700"
                >
                  <Plus className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Advanced Settings */}
        <div>
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center space-x-2 text-sm text-gray-600 hover:text-gray-900"
          >
            <Settings className="h-4 w-4" />
            <span>Advanced Settings</span>
          </button>

          {showAdvanced && (
            <div className="mt-3 p-4 bg-gray-50 rounded-lg grid grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-gray-600 mb-1">Max Results</label>
                <input
                  type="number"
                  value={maxResults}
                  onChange={(e) => setMaxResults(parseInt(e.target.value))}
                  min={1}
                  max={50}
                  className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm"
                />
              </div>
              <div>
                <label className="block text-xs text-gray-600 mb-1">Similarity Threshold</label>
                <input
                  type="number"
                  value={threshold}
                  onChange={(e) => setThreshold(parseFloat(e.target.value))}
                  min={0}
                  max={1}
                  step={0.1}
                  className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm"
                />
              </div>
            </div>
          )}
        </div>

        {/* Retrieve Button */}
        <button
          onClick={handleRetrieve}
          disabled={!selectedAgent || !directive || isLoading}
          className="w-full py-3 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 disabled:bg-gray-300 disabled:cursor-not-allowed flex items-center justify-center space-x-2 transition-colors"
        >
          {isLoading ? (
            <>
              <Loader2 className="h-5 w-5 animate-spin" />
              <span>Searching...</span>
            </>
          ) : (
            <>
              <Play className="h-5 w-5" />
              <span>Retrieve Memories</span>
            </>
          )}
        </button>

        {/* Results Summary */}
        {results && (
          <div className="p-4 bg-green-50 border border-green-200 rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <span className="text-lg font-bold text-green-700">
                  {results.memories.length} memories found
                </span>
                <span className="text-sm text-green-600 ml-2">
                  ({results.total_searched} searched)
                </span>
              </div>
              <span className="text-sm text-green-600">
                {results.search_time_ms.toFixed(1)}ms
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
