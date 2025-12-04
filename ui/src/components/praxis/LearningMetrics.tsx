'use client'

import {
  TrendingUp, TrendingDown, Minus, Shield,
  Brain, Activity, Calendar, Sparkles
} from 'lucide-react'
import type { LearningMetrics } from '@/lib/api'

interface LearningMetricsProps {
  metrics: LearningMetrics | undefined
  isLoading: boolean
}

const trendConfig = {
  1: { icon: TrendingUp, color: 'text-green-600', bgColor: 'bg-green-50', label: 'Improving' },
  0: { icon: Minus, color: 'text-gray-600', bgColor: 'bg-gray-50', label: 'Stable' },
  '-1': { icon: TrendingDown, color: 'text-red-600', bgColor: 'bg-red-50', label: 'Declining' },
}

export function LearningMetricsCard({ metrics, isLoading }: LearningMetricsProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-100 animate-pulse">
        <div className="h-6 w-40 bg-gray-200 rounded mb-6" />
        <div className="h-32 bg-gray-100 rounded-lg mb-4" />
        <div className="grid grid-cols-3 gap-4">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="h-16 bg-gray-100 rounded" />
          ))}
        </div>
      </div>
    )
  }

  if (!metrics) {
    return (
      <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-100">
        <div className="text-center py-8">
          <Activity className="h-12 w-12 text-gray-300 mx-auto mb-3" />
          <p className="text-gray-500">No learning data available</p>
          <p className="text-sm text-gray-400 mt-1">Select an agent to view their learning metrics</p>
        </div>
      </div>
    )
  }

  const trend = trendConfig[metrics.trend.toString() as keyof typeof trendConfig]
  const TrendIcon = trend.icon

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-100">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="p-2 bg-blue-50 rounded-lg">
              <Activity className="h-6 w-6 text-blue-600" />
            </div>
            <div>
              <h3 className="font-semibold text-gray-900">Learning Metrics</h3>
              <p className="text-xs text-gray-500">Tracking improvement over time</p>
            </div>
          </div>
          <div className={`flex items-center space-x-2 px-3 py-1.5 rounded-full ${trend.bgColor}`}>
            <TrendIcon className={`h-4 w-4 ${trend.color}`} />
            <span className={`text-sm font-medium ${trend.color}`}>{trend.label}</span>
          </div>
        </div>
      </div>

      {/* Mini chart */}
      {metrics.history.length > 0 && (
        <div className="px-6 py-4">
          <div className="h-24 flex items-end space-x-1">
            {metrics.history.slice(-20).map((snapshot, i) => {
              const height = Math.max(10, snapshot.fitness_multiplier * 40)
              const isRecent = i >= metrics.history.length - 3
              return (
                <div
                  key={i}
                  className={`flex-1 rounded-t transition-all ${
                    isRecent ? 'bg-primary-500' : 'bg-primary-200'
                  }`}
                  style={{ height: `${height}%` }}
                  title={`${new Date(snapshot.timestamp).toLocaleDateString()}: ${snapshot.fitness_multiplier.toFixed(2)}x`}
                />
              )
            })}
          </div>
          <div className="flex justify-between text-xs text-gray-400 mt-2">
            <span>Older</span>
            <span>Fitness Multiplier History</span>
            <span>Recent</span>
          </div>
        </div>
      )}

      {/* Status indicators */}
      <div className="px-6 py-4 grid grid-cols-2 gap-4">
        <div className={`p-4 rounded-xl ${metrics.is_actively_learning ? 'bg-green-50 border border-green-200' : 'bg-gray-50 border border-gray-200'}`}>
          <div className="flex items-center space-x-2">
            <Brain className={`h-5 w-5 ${metrics.is_actively_learning ? 'text-green-600' : 'text-gray-400'}`} />
            <span className={`text-sm font-medium ${metrics.is_actively_learning ? 'text-green-700' : 'text-gray-600'}`}>
              {metrics.is_actively_learning ? 'Actively Learning' : 'Not Learning'}
            </span>
          </div>
        </div>

        <div className={`p-4 rounded-xl ${metrics.should_protect_for_learning ? 'bg-blue-50 border border-blue-200' : 'bg-gray-50 border border-gray-200'}`}>
          <div className="flex items-center space-x-2">
            <Shield className={`h-5 w-5 ${metrics.should_protect_for_learning ? 'text-blue-600' : 'text-gray-400'}`} />
            <span className={`text-sm font-medium ${metrics.should_protect_for_learning ? 'text-blue-700' : 'text-gray-600'}`}>
              {metrics.should_protect_for_learning ? 'Protected' : 'Not Protected'}
            </span>
          </div>
        </div>
      </div>

      {/* Footer stats */}
      <div className="px-6 py-4 bg-gray-50 border-t border-gray-100">
        <div className="grid grid-cols-3 gap-4 text-center text-sm">
          <div>
            <div className="font-semibold text-gray-900">{metrics.days_since_improvement}</div>
            <div className="text-xs text-gray-500">Days Since Improvement</div>
          </div>
          <div>
            <div className="font-semibold text-gray-900">{metrics.history.length}</div>
            <div className="text-xs text-gray-500">Data Points</div>
          </div>
          <div>
            <div className="font-semibold text-gray-900">
              {(metrics.current.success_rate * 100).toFixed(0)}%
            </div>
            <div className="text-xs text-gray-500">Current Success</div>
          </div>
        </div>
      </div>
    </div>
  )
}
