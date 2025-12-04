'use client'

import {
  Brain, TrendingUp, Sparkles, Target,
  Gauge, BarChart3, Zap, Shield
} from 'lucide-react'
import type { ProceduralCompetence } from '@/lib/api'

interface ProceduralCompetenceProps {
  competence: ProceduralCompetence | undefined
  isLoading: boolean
}

function MetricBar({
  label,
  value,
  icon: Icon,
  color
}: {
  label: string
  value: number
  icon: typeof Brain
  color: string
}) {
  const percentage = Math.round(value * 100)

  return (
    <div>
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center space-x-2">
          <Icon className={`h-4 w-4 ${color}`} />
          <span className="text-sm text-gray-600">{label}</span>
        </div>
        <span className={`text-sm font-semibold ${color}`}>{percentage}%</span>
      </div>
      <div className="h-2 bg-gray-100 rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-500 ${
            percentage > 75 ? 'bg-green-500' :
            percentage > 50 ? 'bg-blue-500' :
            percentage > 25 ? 'bg-amber-500' : 'bg-red-500'
          }`}
          style={{ width: `${percentage}%` }}
        />
      </div>
    </div>
  )
}

export function ProceduralCompetenceCard({ competence, isLoading }: ProceduralCompetenceProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-100 animate-pulse">
        <div className="h-6 w-48 bg-gray-200 rounded mb-6" />
        <div className="space-y-4">
          {[...Array(5)].map((_, i) => (
            <div key={i}>
              <div className="h-4 w-32 bg-gray-200 rounded mb-2" />
              <div className="h-2 bg-gray-200 rounded-full" />
            </div>
          ))}
        </div>
      </div>
    )
  }

  if (!competence) {
    return (
      <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-100">
        <div className="text-center py-8">
          <Brain className="h-12 w-12 text-gray-300 mx-auto mb-3" />
          <p className="text-gray-500">No competence data available</p>
          <p className="text-sm text-gray-400 mt-1">Select an agent to view their procedural competence</p>
        </div>
      </div>
    )
  }

  const multiplier = competence.fitness_multiplier
  const multiplierColor = multiplier >= 1.5 ? 'text-green-600' :
                          multiplier >= 1.0 ? 'text-blue-600' :
                          multiplier >= 0.7 ? 'text-amber-600' : 'text-red-600'

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 bg-gradient-to-r from-purple-50 to-blue-50 border-b border-gray-100">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="p-2 bg-white rounded-lg shadow-sm">
              <Brain className="h-6 w-6 text-purple-600" />
            </div>
            <div>
              <h3 className="font-semibold text-gray-900">Procedural Competence</h3>
              <p className="text-xs text-gray-500">
                {competence.total_memories} memories · {competence.successful_memories} successful
              </p>
            </div>
          </div>
          <div className="text-right">
            <div className={`text-2xl font-bold ${multiplierColor}`}>
              {multiplier.toFixed(2)}x
            </div>
            <div className="text-xs text-gray-500">Fitness Multiplier</div>
          </div>
        </div>
      </div>

      {/* Metrics */}
      <div className="p-6 space-y-4">
        <MetricBar
          label="Success Rate"
          value={competence.success_rate}
          icon={Target}
          color="text-green-600"
        />
        <MetricBar
          label="Diversity"
          value={competence.diversity_score}
          icon={Sparkles}
          color="text-purple-600"
        />
        <MetricBar
          label="Generalization"
          value={competence.generalization_score}
          icon={BarChart3}
          color="text-blue-600"
        />
        <MetricBar
          label="Learning Velocity"
          value={competence.learning_velocity}
          icon={TrendingUp}
          color="text-amber-600"
        />
        <MetricBar
          label="Memory Utilization"
          value={competence.memory_utilization}
          icon={Gauge}
          color="text-cyan-600"
        />
      </div>

      {/* Footer stats */}
      <div className="px-6 py-4 bg-gray-50 border-t border-gray-100">
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div className="text-center">
            <div className="font-semibold text-gray-900">
              {(competence.retrieval_accuracy * 100).toFixed(0)}%
            </div>
            <div className="text-xs text-gray-500">Retrieval Accuracy</div>
          </div>
          <div className="text-center">
            <div className="font-semibold text-gray-900">
              {new Date(competence.calculated_at).toLocaleDateString()}
            </div>
            <div className="text-xs text-gray-500">Last Updated</div>
          </div>
        </div>
      </div>
    </div>
  )
}
