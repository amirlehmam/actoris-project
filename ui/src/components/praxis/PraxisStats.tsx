'use client'

import { Brain, Database, TrendingUp, CheckCircle, Users, Sparkles } from 'lucide-react'
import type { PraxisStats } from '@/lib/api'

interface PraxisStatsProps {
  stats: PraxisStats | undefined
  isLoading: boolean
}

export function PraxisStatsCards({ stats, isLoading }: PraxisStatsProps) {
  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="bg-white rounded-xl p-6 shadow-sm animate-pulse">
            <div className="h-4 w-24 bg-gray-200 rounded mb-4" />
            <div className="h-8 w-16 bg-gray-200 rounded" />
          </div>
        ))}
      </div>
    )
  }

  const statCards = [
    {
      label: 'Total Memories',
      value: stats?.total_memories ?? 0,
      icon: Database,
      color: 'text-purple-600',
      bgColor: 'bg-purple-50',
    },
    {
      label: 'Unique Agents',
      value: stats?.unique_agents ?? 0,
      icon: Users,
      color: 'text-blue-600',
      bgColor: 'bg-blue-50',
    },
    {
      label: 'Success Rate',
      value: `${((stats?.success_rate ?? 0) * 100).toFixed(1)}%`,
      icon: CheckCircle,
      color: 'text-green-600',
      bgColor: 'bg-green-50',
    },
    {
      label: 'Avg Memories/Agent',
      value: (stats?.avg_memories_per_agent ?? 0).toFixed(1),
      icon: Brain,
      color: 'text-amber-600',
      bgColor: 'bg-amber-50',
    },
  ]

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      {statCards.map((stat) => (
        <div key={stat.label} className="bg-white rounded-xl p-6 shadow-sm border border-gray-100">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-500">{stat.label}</p>
              <p className="text-2xl font-bold mt-1">{stat.value}</p>
            </div>
            <div className={`p-3 rounded-xl ${stat.bgColor}`}>
              <stat.icon className={`h-6 w-6 ${stat.color}`} />
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}
