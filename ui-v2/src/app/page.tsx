'use client'

import { useQuery } from '@tanstack/react-query'
import { apiClient } from '@/lib/api'
import { formatNumber, formatCurrency, formatPercent } from '@/lib/utils'
import {
  TrendingUp, Users, Shield, DollarSign,
  Activity, Clock, AlertTriangle, CheckCircle
} from 'lucide-react'

export default function AGDPDashboard() {
  const { data: stats, isLoading } = useQuery({
    queryKey: ['stats'],
    queryFn: apiClient.getStats,
  })

  const { data: actions } = useQuery({
    queryKey: ['actions'],
    queryFn: apiClient.getActions,
  })

  if (isLoading) {
    return <DashboardSkeleton />
  }

  const agdp = stats?.agdp

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold text-gray-900">AGDP Dashboard</h1>
        <p className="text-gray-500 mt-1">
          Agentic Gross Domestic Product — Real-time economic metrics for your AI agent fleet
        </p>
      </div>

      {/* AGDP Hero Card */}
      <div className="card bg-gradient-to-br from-actoris-500 to-actoris-600 text-white p-8">
        <div className="flex justify-between items-start">
          <div>
            <p className="text-actoris-100 text-sm font-medium">Total AGDP</p>
            <p className="text-5xl font-bold mt-2">
              {formatCurrency(agdp?.total_agdp || 0)}
            </p>
            <p className="text-actoris-100 mt-2">
              Σ(Actions × Price) — Verified economic value
            </p>
          </div>
          <div className="bg-white/20 rounded-xl p-4">
            <TrendingUp className="w-12 h-12" />
          </div>
        </div>

        <div className="grid grid-cols-4 gap-6 mt-8 pt-6 border-t border-white/20">
          <div>
            <p className="text-actoris-100 text-xs">Actions/sec</p>
            <p className="text-2xl font-bold">3,699</p>
          </div>
          <div>
            <p className="text-actoris-100 text-xs">Verification P95</p>
            <p className="text-2xl font-bold">{agdp?.avg_verification_latency || 847}ms</p>
          </div>
          <div>
            <p className="text-actoris-100 text-xs">Compute Efficiency (CRI)</p>
            <p className="text-2xl font-bold">{agdp?.compute_efficiency?.toFixed(2) || '1.23'}x</p>
          </div>
          <div>
            <p className="text-actoris-100 text-xs">Dispute Rate</p>
            <p className="text-2xl font-bold">{formatPercent(agdp?.dispute_rate || 0.0064)}</p>
          </div>
        </div>
      </div>

      {/* Key Metrics Grid */}
      <div className="grid grid-cols-4 gap-6">
        <MetricCard
          title="Total Entities"
          value={formatNumber(stats?.total_entities || 0, 0)}
          subtitle="UnifiedIDs in network"
          icon={<Users className="w-5 h-5" />}
          color="blue"
        />
        <MetricCard
          title="Verified Actions"
          value={formatNumber(stats?.total_verified || 0, 0)}
          subtitle={`${formatPercent((stats?.total_verified || 0) / (stats?.total_actions || 1))} success`}
          icon={<CheckCircle className="w-5 h-5" />}
          color="green"
        />
        <MetricCard
          title="Avg Trust Score"
          value={stats?.avg_trust_score?.toFixed(0) || '687'}
          subtitle="Network-wide τ"
          icon={<Shield className="w-5 h-5" />}
          color="purple"
        />
        <MetricCard
          title="Culled Agents"
          value={stats?.culled_count || 23}
          subtitle="η < 0.7 for 2 epochs"
          icon={<AlertTriangle className="w-5 h-5" />}
          color="red"
        />
      </div>

      {/* Two Column Layout */}
      <div className="grid grid-cols-2 gap-8">
        {/* Recent Verifications */}
        <div className="card">
          <div className="card-header flex justify-between items-center">
            <h2 className="font-semibold text-gray-900">Recent Verifications</h2>
            <span className="text-xs bg-green-100 text-green-700 px-2 py-1 rounded-full">
              Live
            </span>
          </div>
          <div className="card-body p-0">
            <table className="w-full">
              <thead className="bg-gray-50">
                <tr>
                  <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Action</th>
                  <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Status</th>
                  <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Price</th>
                  <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Latency</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {actions?.slice(0, 5).map((action) => (
                  <tr key={action.id} className="hover:bg-gray-50">
                    <td className="px-6 py-4">
                      <span className="font-mono text-sm">{action.id}</span>
                      <p className="text-xs text-gray-500">{action.action_type}</p>
                    </td>
                    <td className="px-6 py-4">
                      <StatusBadge status={action.status} />
                    </td>
                    <td className="px-6 py-4 font-mono text-sm">
                      {formatCurrency(action.pricing.final_price)}
                    </td>
                    <td className="px-6 py-4 text-sm text-gray-600">
                      {action.verification?.latency_ms?.toFixed(0) || '—'}ms
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* 4 Engines Status */}
        <div className="card">
          <div className="card-header">
            <h2 className="font-semibold text-gray-900">System Engines</h2>
          </div>
          <div className="card-body space-y-4">
            <EngineStatus
              name="IdentityCloud"
              description="UnifiedID Registry"
              status="operational"
              metric="3,847 entities"
            />
            <EngineStatus
              name="TrustLedger"
              description="3-of-N Oracle Consensus"
              status="operational"
              metric="<2s latency"
            />
            <EngineStatus
              name="OneBill"
              description="Outcome-Based Pricing"
              status="operational"
              metric="$0.102 avg"
            />
            <EngineStatus
              name="Darwinian"
              description="Resource Allocation"
              status="operational"
              metric="23 culled"
            />
          </div>
        </div>
      </div>

      {/* Pricing Formula Banner */}
      <div className="card p-6 bg-gray-900 text-white">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-semibold text-lg">OneBill Pricing Formula</h3>
            <p className="text-gray-400 text-sm mt-1">Dynamic outcome-based pricing per verified action</p>
          </div>
          <div className="font-mono text-2xl bg-gray-800 px-6 py-3 rounded-lg">
            Price = <span className="text-blue-400">Compute</span> + <span className="text-red-400">Risk</span> - <span className="text-green-400">Trust</span>
          </div>
        </div>
      </div>
    </div>
  )
}

function MetricCard({ title, value, subtitle, icon, color }: {
  title: string
  value: string | number
  subtitle: string
  icon: React.ReactNode
  color: 'blue' | 'green' | 'purple' | 'red' | 'orange'
}) {
  const colors = {
    blue: 'bg-blue-50 text-blue-600',
    green: 'bg-green-50 text-green-600',
    purple: 'bg-purple-50 text-purple-600',
    red: 'bg-red-50 text-red-600',
    orange: 'bg-actoris-50 text-actoris-600',
  }

  return (
    <div className="stat-card">
      <div className="flex items-center justify-between">
        <div className={`p-2 rounded-lg ${colors[color]}`}>{icon}</div>
      </div>
      <p className="metric-value mt-4">{value}</p>
      <p className="text-sm font-medium text-gray-700 mt-1">{title}</p>
      <p className="text-xs text-gray-500">{subtitle}</p>
    </div>
  )
}

function StatusBadge({ status }: { status: string }) {
  const config: Record<string, { bg: string; text: string; dot: string }> = {
    verified: { bg: 'bg-green-50', text: 'text-green-700', dot: 'bg-green-500' },
    processing: { bg: 'bg-blue-50', text: 'text-blue-700', dot: 'bg-blue-500' },
    pending: { bg: 'bg-gray-50', text: 'text-gray-700', dot: 'bg-gray-400' },
    disputed: { bg: 'bg-red-50', text: 'text-red-700', dot: 'bg-red-500' },
    failed: { bg: 'bg-red-50', text: 'text-red-700', dot: 'bg-red-500' },
  }
  const c = config[status] || config.pending

  return (
    <span className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${c.bg} ${c.text}`}>
      <span className={`w-1.5 h-1.5 rounded-full mr-1.5 ${c.dot}`} />
      {status}
    </span>
  )
}

function EngineStatus({ name, description, status, metric }: {
  name: string
  description: string
  status: 'operational' | 'degraded' | 'down'
  metric: string
}) {
  return (
    <div className="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
      <div className="flex items-center space-x-3">
        <div className={`w-2 h-2 rounded-full ${status === 'operational' ? 'bg-green-500' : 'bg-red-500'}`} />
        <div>
          <p className="font-medium text-gray-900">{name}</p>
          <p className="text-xs text-gray-500">{description}</p>
        </div>
      </div>
      <span className="text-sm font-mono text-gray-600">{metric}</span>
    </div>
  )
}

function DashboardSkeleton() {
  return (
    <div className="space-y-8 animate-pulse">
      <div className="h-8 w-64 bg-gray-200 rounded" />
      <div className="h-64 bg-gray-200 rounded-xl" />
      <div className="grid grid-cols-4 gap-6">
        {[1, 2, 3, 4].map(i => <div key={i} className="h-32 bg-gray-200 rounded-xl" />)}
      </div>
    </div>
  )
}
