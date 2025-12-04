'use client'

import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  Brain, Database, Search, Activity, Sparkles,
  ChevronRight, RefreshCw, Users, Zap
} from 'lucide-react'
import { apiClient } from '@/lib/api'
import type { Agent, PraxisMemory, RetrievedMemory, ProceduralCompetence, LearningMetrics, ActionAugmentation } from '@/lib/api'
import {
  PraxisStatsCards,
  MemoryCard,
  MemoryCardSkeleton,
  ProceduralCompetenceCard,
  LearningMetricsCard,
  ActionAugmentationCard,
  RetrievalDemo,
} from '@/components/praxis'

type TabType = 'overview' | 'memories' | 'retrieval' | 'competence'

export default function PraxisPage() {
  const [activeTab, setActiveTab] = useState<TabType>('overview')
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null)
  const [retrievalResults, setRetrievalResults] = useState<{
    memories: RetrievedMemory[]
    total_searched: number
    search_time_ms: number
  } | undefined>()
  const [augmentation, setAugmentation] = useState<ActionAugmentation | undefined>()

  const queryClient = useQueryClient()

  // Queries
  const { data: agents, isLoading: agentsLoading } = useQuery({
    queryKey: ['agents'],
    queryFn: apiClient.getAgents,
  })

  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ['praxis-stats'],
    queryFn: apiClient.getPraxisStats,
    refetchInterval: 30000,
  })

  const { data: memories, isLoading: memoriesLoading } = useQuery({
    queryKey: ['agent-memories', selectedAgent],
    queryFn: () => selectedAgent ? apiClient.getAgentMemories(selectedAgent, 50) : Promise.resolve([]),
    enabled: !!selectedAgent,
  })

  const { data: competence, isLoading: competenceLoading } = useQuery({
    queryKey: ['agent-competence', selectedAgent],
    queryFn: () => selectedAgent ? apiClient.getProceduralCompetence(selectedAgent) : Promise.resolve(undefined),
    enabled: !!selectedAgent,
  })

  const { data: learningMetrics, isLoading: learningLoading } = useQuery({
    queryKey: ['agent-learning', selectedAgent],
    queryFn: () => selectedAgent ? apiClient.getLearningMetrics(selectedAgent) : Promise.resolve(undefined),
    enabled: !!selectedAgent,
  })

  // Mutations
  const retrieveMutation = useMutation({
    mutationFn: apiClient.retrieveMemories,
    onSuccess: async (data) => {
      setRetrievalResults(data)
      // Also get augmentation
      if (data.memories.length > 0) {
        const memory = data.memories[0].memory
        try {
          const aug = await apiClient.getActionAugmentation({
            agent_id: memory.agent_id,
            current_env: memory.env_state_pre,
            current_internal: memory.internal_state,
            max_memories: 5,
          })
          setAugmentation(aug)
        } catch (e) {
          console.error('Failed to get augmentation:', e)
        }
      }
    },
  })

  const handleRetrieve = async (data: Parameters<typeof apiClient.retrieveMemories>[0]) => {
    return retrieveMutation.mutateAsync(data)
  }

  const tabs = [
    { id: 'overview', label: 'Overview', icon: Brain },
    { id: 'memories', label: 'Memories', icon: Database },
    { id: 'retrieval', label: 'Retrieval Demo', icon: Search },
    { id: 'competence', label: 'Competence', icon: Activity },
  ]

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center space-x-3">
            <div className="p-3 bg-gradient-to-br from-purple-500 to-blue-600 rounded-xl shadow-lg">
              <Brain className="h-8 w-8 text-white" />
            </div>
            <div>
              <h1 className="text-3xl font-bold text-gray-900">PRAXIS</h1>
              <p className="text-gray-500 mt-1">
                Procedural Recall for Agents with eXperiences Indexed by State
              </p>
            </div>
          </div>
        </div>
        <button
          onClick={() => {
            queryClient.invalidateQueries({ queryKey: ['praxis-stats'] })
            if (selectedAgent) {
              queryClient.invalidateQueries({ queryKey: ['agent-memories', selectedAgent] })
              queryClient.invalidateQueries({ queryKey: ['agent-competence', selectedAgent] })
              queryClient.invalidateQueries({ queryKey: ['agent-learning', selectedAgent] })
            }
          }}
          className="flex items-center space-x-2 px-4 py-2 bg-white border border-gray-200 rounded-xl hover:bg-gray-50 transition-colors"
        >
          <RefreshCw className="h-4 w-4" />
          <span>Refresh</span>
        </button>
      </div>

      {/* Stats Cards */}
      <PraxisStatsCards stats={stats} isLoading={statsLoading} />

      {/* Tab Navigation */}
      <div className="flex space-x-2 border-b border-gray-200">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id as TabType)}
            className={`flex items-center space-x-2 px-4 py-3 border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-primary-600 text-primary-600'
                : 'border-transparent text-gray-500 hover:text-gray-700'
            }`}
          >
            <tab.icon className="h-5 w-5" />
            <span className="font-medium">{tab.label}</span>
          </button>
        ))}
      </div>

      {/* Agent Selector (shown on relevant tabs) */}
      {(activeTab === 'memories' || activeTab === 'competence') && (
        <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100">
          <div className="flex items-center space-x-4">
            <Users className="h-5 w-5 text-gray-400" />
            <select
              value={selectedAgent || ''}
              onChange={(e) => setSelectedAgent(e.target.value || null)}
              className="flex-1 px-4 py-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
            >
              <option value="">Select an agent to view details...</option>
              {agents?.map((agent) => (
                <option key={agent.id} value={agent.id}>
                  {agent.name} ({agent.agent_type}) - Trust: {(agent.trust_score.tau * 100).toFixed(0)}%
                </option>
              ))}
            </select>
          </div>
        </div>
      )}

      {/* Tab Content */}
      {activeTab === 'overview' && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Left column - Concept explanation */}
          <div className="lg:col-span-2 space-y-6">
            <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-100">
              <h2 className="text-xl font-semibold text-gray-900 mb-4">
                What is PRAXIS?
              </h2>
              <p className="text-gray-600 mb-4">
                PRAXIS (Procedural Recall for Agents with eXperiences Indexed by State) is a
                lightweight post-training learning mechanism that enables AI agents to learn
                from their experiences in real-time.
              </p>

              <div className="grid grid-cols-2 gap-4 mt-6">
                <div className="p-4 bg-purple-50 rounded-xl border border-purple-100">
                  <h3 className="font-semibold text-purple-900 mb-2 flex items-center space-x-2">
                    <Database className="h-5 w-5" />
                    <span>Memory Storage</span>
                  </h3>
                  <p className="text-sm text-purple-700">
                    Stores state-action-outcome triplets indexed by environmental and internal state.
                  </p>
                </div>

                <div className="p-4 bg-blue-50 rounded-xl border border-blue-100">
                  <h3 className="font-semibold text-blue-900 mb-2 flex items-center space-x-2">
                    <Search className="h-5 w-5" />
                    <span>Smart Retrieval</span>
                  </h3>
                  <p className="text-sm text-blue-700">
                    Uses IoU + embedding similarity to find relevant past experiences.
                  </p>
                </div>

                <div className="p-4 bg-green-50 rounded-xl border border-green-100">
                  <h3 className="font-semibold text-green-900 mb-2 flex items-center space-x-2">
                    <Sparkles className="h-5 w-5" />
                    <span>Action Augmentation</span>
                  </h3>
                  <p className="text-sm text-green-700">
                    Augments agent decisions with retrieved procedural knowledge.
                  </p>
                </div>

                <div className="p-4 bg-amber-50 rounded-xl border border-amber-100">
                  <h3 className="font-semibold text-amber-900 mb-2 flex items-center space-x-2">
                    <Activity className="h-5 w-5" />
                    <span>Fitness Integration</span>
                  </h3>
                  <p className="text-sm text-amber-700">
                    Learning ability factors into Darwinian fitness calculations.
                  </p>
                </div>
              </div>
            </div>

            {/* Formula explanation */}
            <div className="bg-gradient-to-br from-gray-900 to-gray-800 rounded-xl p-6 text-white">
              <h3 className="font-semibold mb-4 flex items-center space-x-2">
                <Zap className="h-5 w-5 text-yellow-400" />
                <span>Enhanced Fitness Formula</span>
              </h3>
              <div className="font-mono text-lg text-center py-4 bg-black/30 rounded-lg mb-4">
                η = τ × (revenue / cost) × <span className="text-green-400">procedural_multiplier</span>
              </div>
              <p className="text-gray-300 text-sm">
                The procedural_multiplier (0.5x - 2.0x) is derived from PRAXIS competence metrics:
                success rate, diversity, generalization, and learning velocity. Agents that learn
                effectively get boosted fitness; those that don't learn get penalized.
              </p>
            </div>
          </div>

          {/* Right column - Quick stats */}
          <div className="space-y-6">
            <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-100">
              <h3 className="font-semibold text-gray-900 mb-4">Quick Facts</h3>
              <div className="space-y-4">
                <div className="flex items-center justify-between py-2 border-b border-gray-100">
                  <span className="text-gray-600">Retrieval Algorithm</span>
                  <span className="font-mono text-sm bg-gray-100 px-2 py-1 rounded">IoU + Embedding</span>
                </div>
                <div className="flex items-center justify-between py-2 border-b border-gray-100">
                  <span className="text-gray-600">Default Threshold</span>
                  <span className="font-mono text-sm bg-gray-100 px-2 py-1 rounded">τ = 0.3</span>
                </div>
                <div className="flex items-center justify-between py-2 border-b border-gray-100">
                  <span className="text-gray-600">Search Breadth</span>
                  <span className="font-mono text-sm bg-gray-100 px-2 py-1 rounded">k = 10</span>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-600">Max Memories/Agent</span>
                  <span className="font-mono text-sm bg-gray-100 px-2 py-1 rounded">10,000</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-br from-purple-600 to-blue-600 rounded-xl p-6 text-white">
              <h3 className="font-semibold mb-2">Paper Reference</h3>
              <p className="text-purple-100 text-sm mb-4">
                Based on "Real-Time Procedural Learning From Experience for AI Agents"
              </p>
              <p className="text-xs text-purple-200">
                Bi, Hu, Nasir - Altrina, 2025
              </p>
            </div>
          </div>
        </div>
      )}

      {activeTab === 'memories' && (
        <div className="space-y-6">
          {!selectedAgent ? (
            <div className="bg-white rounded-xl p-12 shadow-sm border border-gray-100 text-center">
              <Database className="h-16 w-16 text-gray-300 mx-auto mb-4" />
              <p className="text-gray-500 text-lg">Select an agent above to view their procedural memories</p>
            </div>
          ) : memoriesLoading ? (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {[...Array(4)].map((_, i) => (
                <MemoryCardSkeleton key={i} />
              ))}
            </div>
          ) : memories && memories.length > 0 ? (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {memories.map((memory) => (
                <MemoryCard key={memory.id} memory={memory} />
              ))}
            </div>
          ) : (
            <div className="bg-white rounded-xl p-12 shadow-sm border border-gray-100 text-center">
              <Brain className="h-16 w-16 text-gray-300 mx-auto mb-4" />
              <p className="text-gray-500 text-lg">No memories found for this agent</p>
              <p className="text-gray-400 text-sm mt-2">
                Memories are created as the agent performs actions
              </p>
            </div>
          )}
        </div>
      )}

      {activeTab === 'retrieval' && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <RetrievalDemo
            agents={agents || []}
            onRetrieve={handleRetrieve}
            isLoading={retrieveMutation.isPending}
            results={retrievalResults}
          />

          <div className="space-y-6">
            {/* Retrieved Memories */}
            {retrievalResults && retrievalResults.memories.length > 0 && (
              <div className="space-y-4">
                <h3 className="font-semibold text-gray-900">Retrieved Memories</h3>
                {retrievalResults.memories.slice(0, 5).map((retrieved) => (
                  <MemoryCard
                    key={retrieved.memory.id}
                    memory={retrieved.memory}
                    relevanceScore={retrieved.relevance_score}
                    rank={retrieved.rank}
                  />
                ))}
              </div>
            )}

            {/* Action Augmentation */}
            <ActionAugmentationCard
              augmentation={augmentation}
              isLoading={retrieveMutation.isPending}
            />
          </div>
        </div>
      )}

      {activeTab === 'competence' && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <ProceduralCompetenceCard
            competence={competence}
            isLoading={competenceLoading}
          />
          <LearningMetricsCard
            metrics={learningMetrics}
            isLoading={learningLoading}
          />
        </div>
      )}
    </div>
  )
}
