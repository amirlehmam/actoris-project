'use client'

import { useQuery } from '@tanstack/react-query'
import { apiClient } from '@/lib/api'
import { StatsCards } from '@/components/StatsCards'
import { AgentsList } from '@/components/AgentsList'
import { RecentActions } from '@/components/RecentActions'
import { QuickActions } from '@/components/QuickActions'
import { ServiceStatus } from '@/components/ServiceStatus'

export default function Dashboard() {
  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ['stats'],
    queryFn: apiClient.getStats,
  })

  const { data: health, isLoading: healthLoading } = useQuery({
    queryKey: ['health'],
    queryFn: apiClient.getHealth,
  })

  const { data: agents, isLoading: agentsLoading } = useQuery({
    queryKey: ['agents'],
    queryFn: apiClient.getAgents,
  })

  const { data: actions, isLoading: actionsLoading } = useQuery({
    queryKey: ['actions'],
    queryFn: apiClient.getActions,
  })

  return (
    <div className="space-y-8">
      {/* Page Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
          <p className="mt-1 text-gray-500">
            Autonomous Contract-based Trust Operating & Resource Interoperability System
          </p>
        </div>
        <QuickActions />
      </div>

      {/* Stats Cards */}
      <StatsCards stats={stats} isLoading={statsLoading} />

      {/* Main Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Left Column - Agents */}
        <div className="lg:col-span-2">
          <AgentsList agents={agents || []} isLoading={agentsLoading} />
        </div>

        {/* Right Column - Service Status */}
        <div>
          <ServiceStatus health={health} isLoading={healthLoading} />
        </div>
      </div>

      {/* Recent Actions */}
      <RecentActions actions={actions || []} isLoading={actionsLoading} />
    </div>
  )
}
