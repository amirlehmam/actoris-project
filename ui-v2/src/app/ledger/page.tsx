'use client'

import { useQuery } from '@tanstack/react-query'
import { apiClient, Action } from '@/lib/api'
import { formatCurrency } from '@/lib/utils'
import { Shield, CheckCircle, Clock, AlertTriangle, XCircle, Zap } from 'lucide-react'

export default function TrustLedgerPage() {
  const { data: actions, isLoading } = useQuery({
    queryKey: ['actions'],
    queryFn: apiClient.getActions,
    refetchInterval: 5000,
  })

  const verifiedActions = actions?.filter(a => a.verification) || []
  const stats = {
    total: actions?.length || 0,
    verified: actions?.filter(a => a.status === 'verified').length || 0,
    processing: actions?.filter(a => a.status === 'processing').length || 0,
    disputed: actions?.filter(a => a.status === 'disputed').length || 0,
    avgLatency: verifiedActions.length > 0
      ? verifiedActions.reduce((acc, a) => acc + (a.verification?.latency_ms || 0), 0) / verifiedActions.length
      : 0,
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold text-gray-900">TrustLedger</h1>
        <p className="text-gray-500 mt-1">
          3-of-N Oracle Consensus — Immutable verification records with FROST threshold signatures
        </p>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-5 gap-4">
        <div className="stat-card text-center">
          <Shield className="w-6 h-6 mx-auto text-blue-500" />
          <p className="metric-value mt-2">{stats.total}</p>
          <p className="metric-label">Total Actions</p>
        </div>
        <div className="stat-card text-center">
          <CheckCircle className="w-6 h-6 mx-auto text-green-500" />
          <p className="metric-value mt-2">{stats.verified}</p>
          <p className="metric-label">Verified</p>
        </div>
        <div className="stat-card text-center">
          <Clock className="w-6 h-6 mx-auto text-blue-500 animate-spin" />
          <p className="metric-value mt-2">{stats.processing}</p>
          <p className="metric-label">Processing</p>
        </div>
        <div className="stat-card text-center">
          <AlertTriangle className="w-6 h-6 mx-auto text-red-500" />
          <p className="metric-value mt-2">{stats.disputed}</p>
          <p className="metric-label">Disputed</p>
        </div>
        <div className="stat-card text-center">
          <Zap className="w-6 h-6 mx-auto text-yellow-500" />
          <p className="metric-value mt-2">{stats.avgLatency.toFixed(0)}ms</p>
          <p className="metric-label">Avg Latency</p>
        </div>
      </div>

      {/* Consensus Explanation */}
      <div className="card p-6 bg-gradient-to-r from-gray-900 to-gray-800 text-white">
        <h3 className="font-semibold text-lg mb-4">How TrustLedger Works</h3>
        <div className="grid grid-cols-4 gap-6">
          <div className="text-center">
            <div className="w-12 h-12 mx-auto bg-blue-500/20 rounded-full flex items-center justify-center mb-2">
              <span className="text-2xl font-bold">1</span>
            </div>
            <p className="text-sm text-gray-300">Action submitted with input hash</p>
          </div>
          <div className="text-center">
            <div className="w-12 h-12 mx-auto bg-purple-500/20 rounded-full flex items-center justify-center mb-2">
              <span className="text-2xl font-bold">2</span>
            </div>
            <p className="text-sm text-gray-300">5 Oracles receive verification request</p>
          </div>
          <div className="text-center">
            <div className="w-12 h-12 mx-auto bg-green-500/20 rounded-full flex items-center justify-center mb-2">
              <span className="text-2xl font-bold">3</span>
            </div>
            <p className="text-sm text-gray-300">3-of-5 quorum reached via FROST</p>
          </div>
          <div className="text-center">
            <div className="w-12 h-12 mx-auto bg-actoris-500/20 rounded-full flex items-center justify-center mb-2">
              <span className="text-2xl font-bold">4</span>
            </div>
            <p className="text-sm text-gray-300">Aggregate signature recorded &lt;2s</p>
          </div>
        </div>
      </div>

      {/* Actions List */}
      <div className="card">
        <div className="card-header flex justify-between items-center">
          <h2 className="font-semibold">Verification Pipeline</h2>
          <span className="flex items-center space-x-2 text-sm text-green-600">
            <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
            <span>Live Feed</span>
          </span>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-gray-50">
              <tr>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Action ID</th>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Type</th>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Status</th>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Oracle Votes</th>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">Latency</th>
                <th className="text-left text-xs font-medium text-gray-500 px-6 py-3">FROST Sig</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {isLoading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <tr key={i}>
                    <td colSpan={6} className="px-6 py-4">
                      <div className="h-4 bg-gray-200 rounded animate-pulse" />
                    </td>
                  </tr>
                ))
              ) : (
                actions?.map((action) => (
                  <ActionRow key={action.id} action={action} />
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}

function ActionRow({ action }: { action: Action }) {
  const statusConfig = {
    verified: { icon: CheckCircle, color: 'text-green-600', bg: 'bg-green-50' },
    processing: { icon: Clock, color: 'text-blue-600', bg: 'bg-blue-50' },
    pending: { icon: Clock, color: 'text-gray-600', bg: 'bg-gray-50' },
    disputed: { icon: AlertTriangle, color: 'text-red-600', bg: 'bg-red-50' },
    failed: { icon: XCircle, color: 'text-red-600', bg: 'bg-red-50' },
    settled: { icon: CheckCircle, color: 'text-green-600', bg: 'bg-green-50' },
  }

  const config = statusConfig[action.status] || statusConfig.pending
  const Icon = config.icon

  return (
    <tr className="hover:bg-gray-50">
      <td className="px-6 py-4">
        <span className="font-mono text-sm">{action.id}</span>
      </td>
      <td className="px-6 py-4">
        <span className="px-2 py-1 bg-gray-100 rounded text-sm">{action.action_type}</span>
      </td>
      <td className="px-6 py-4">
        <span className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${config.bg} ${config.color}`}>
          <Icon className="w-3 h-3 mr-1" />
          {action.status}
        </span>
      </td>
      <td className="px-6 py-4">
        {action.verification ? (
          <div className="flex space-x-1">
            {action.verification.oracle_votes.map((vote, i) => (
              <span
                key={i}
                className={`w-6 h-6 rounded-full flex items-center justify-center text-xs ${
                  vote.vote ? 'bg-green-100 text-green-600' : 'bg-red-100 text-red-600'
                }`}
                title={vote.oracle_name}
              >
                {vote.vote ? '✓' : '✗'}
              </span>
            ))}
            <span className="text-xs text-gray-500 ml-2">
              {action.verification.quorum_threshold}
            </span>
          </div>
        ) : (
          <span className="text-gray-400 text-sm">Awaiting...</span>
        )}
      </td>
      <td className="px-6 py-4">
        {action.verification ? (
          <span className={`font-mono text-sm ${
            action.verification.latency_ms < 1000 ? 'text-green-600' :
            action.verification.latency_ms < 2000 ? 'text-yellow-600' : 'text-red-600'
          }`}>
            {action.verification.latency_ms.toFixed(0)}ms
          </span>
        ) : (
          <span className="text-gray-400">—</span>
        )}
      </td>
      <td className="px-6 py-4">
        {action.verification?.aggregate_signature ? (
          <span className="font-mono text-xs text-gray-500">
            {action.verification.aggregate_signature.slice(0, 16)}...
          </span>
        ) : (
          <span className="text-gray-400">—</span>
        )}
      </td>
    </tr>
  )
}
