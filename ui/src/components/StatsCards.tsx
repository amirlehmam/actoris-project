'use client'

import { Users, Activity, CheckCircle2, Coins } from 'lucide-react'
import type { Stats } from '@/lib/api'

interface StatsCardsProps {
  stats?: Stats
  isLoading: boolean
}

export function StatsCards({ stats, isLoading }: StatsCardsProps) {
  const cards = [
    {
      name: 'Total Agents',
      value: stats?.total_agents || 0,
      icon: Users,
      color: 'bg-blue-500',
      change: '+12%',
    },
    {
      name: 'Total Actions',
      value: stats?.total_actions || 0,
      icon: Activity,
      color: 'bg-purple-500',
      change: '+23%',
    },
    {
      name: 'Verified',
      value: stats?.total_verified || 0,
      icon: CheckCircle2,
      color: 'bg-green-500',
      change: '+8%',
    },
    {
      name: 'HC Volume',
      value: stats?.total_hc_volume || '0',
      icon: Coins,
      color: 'bg-amber-500',
      prefix: 'HC ',
    },
  ]

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="bg-white rounded-xl p-6 shadow-sm animate-pulse">
            <div className="h-12 w-12 bg-gray-200 rounded-lg mb-4" />
            <div className="h-4 w-24 bg-gray-200 rounded mb-2" />
            <div className="h-8 w-16 bg-gray-200 rounded" />
          </div>
        ))}
      </div>
    )
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      {cards.map((card) => (
        <div
          key={card.name}
          className="bg-white rounded-xl p-6 shadow-sm hover:shadow-md transition-shadow"
        >
          <div className="flex items-center justify-between">
            <div className={`${card.color} p-3 rounded-lg`}>
              <card.icon className="h-6 w-6 text-white" />
            </div>
            {card.change && (
              <span className="text-green-500 text-sm font-medium">{card.change}</span>
            )}
          </div>
          <div className="mt-4">
            <p className="text-gray-500 text-sm">{card.name}</p>
            <p className="text-2xl font-bold text-gray-900 mt-1">
              {card.prefix}
              {typeof card.value === 'number'
                ? card.value.toLocaleString()
                : parseFloat(card.value).toLocaleString()}
            </p>
          </div>
        </div>
      ))}
    </div>
  )
}
