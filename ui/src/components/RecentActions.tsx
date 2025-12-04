'use client'

import { Clock, CheckCircle2, XCircle, Loader2, AlertTriangle } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'
import type { Action } from '@/lib/api'

interface RecentActionsProps {
  actions: Action[]
  isLoading: boolean
}

const statusConfig = {
  pending: {
    icon: Clock,
    color: 'text-gray-500',
    bgColor: 'bg-gray-100',
    label: 'Pending',
  },
  processing: {
    icon: Loader2,
    color: 'text-blue-500',
    bgColor: 'bg-blue-100',
    label: 'Processing',
    animate: true,
  },
  verified: {
    icon: CheckCircle2,
    color: 'text-green-500',
    bgColor: 'bg-green-100',
    label: 'Verified',
  },
  disputed: {
    icon: AlertTriangle,
    color: 'text-amber-500',
    bgColor: 'bg-amber-100',
    label: 'Disputed',
  },
  settled: {
    icon: CheckCircle2,
    color: 'text-blue-500',
    bgColor: 'bg-blue-100',
    label: 'Settled',
  },
  failed: {
    icon: XCircle,
    color: 'text-red-500',
    bgColor: 'bg-red-100',
    label: 'Failed',
  },
}

export function RecentActions({ actions, isLoading }: RecentActionsProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-xl shadow-sm p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Recent Actions</h2>
        <div className="space-y-4">
          {[1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="animate-pulse flex items-center space-x-4">
              <div className="h-10 w-10 bg-gray-200 rounded-lg" />
              <div className="flex-1">
                <div className="h-4 w-48 bg-gray-200 rounded mb-2" />
                <div className="h-3 w-32 bg-gray-200 rounded" />
              </div>
              <div className="h-6 w-20 bg-gray-200 rounded-full" />
            </div>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="bg-white rounded-xl shadow-sm p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-900">Recent Actions</h2>
        <a href="/actions" className="text-primary-600 hover:text-primary-700 text-sm font-medium">
          View all
        </a>
      </div>

      {actions.length === 0 ? (
        <div className="text-center py-12">
          <Clock className="h-12 w-12 text-gray-400 mx-auto mb-4" />
          <p className="text-gray-500">No actions yet. Submit your first action to see it here!</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="text-left text-sm text-gray-500 border-b">
                <th className="pb-3 font-medium">Action</th>
                <th className="pb-3 font-medium">Type</th>
                <th className="pb-3 font-medium">Producer</th>
                <th className="pb-3 font-medium">Consumer</th>
                <th className="pb-3 font-medium">Price</th>
                <th className="pb-3 font-medium">Status</th>
                <th className="pb-3 font-medium">Time</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {actions.slice(0, 10).map((action) => {
                const status = statusConfig[action.status] || statusConfig.pending
                const Icon = status.icon

                return (
                  <tr key={action.id} className="hover:bg-gray-50">
                    <td className="py-3">
                      <span className="font-mono text-sm text-gray-600">
                        {action.id.slice(0, 8)}...
                      </span>
                    </td>
                    <td className="py-3">
                      <span className="px-2 py-1 bg-gray-100 text-gray-700 text-sm rounded">
                        {action.action_type}
                      </span>
                    </td>
                    <td className="py-3">
                      <span className="font-mono text-sm text-gray-600">
                        {action.producer_id.slice(0, 8)}...
                      </span>
                    </td>
                    <td className="py-3">
                      <span className="font-mono text-sm text-gray-600">
                        {action.consumer_id.slice(0, 8)}...
                      </span>
                    </td>
                    <td className="py-3">
                      <span className="font-medium text-gray-900">
                        {action.price ? `${parseFloat(action.price).toFixed(2)} HC` : '-'}
                      </span>
                    </td>
                    <td className="py-3">
                      <span className={`inline-flex items-center space-x-1 px-2 py-1 rounded-full text-sm ${status.bgColor} ${status.color}`}>
                        <Icon className={`h-3.5 w-3.5 ${status.animate ? 'animate-spin' : ''}`} />
                        <span>{status.label}</span>
                      </span>
                    </td>
                    <td className="py-3 text-sm text-gray-500">
                      {formatDistanceToNow(new Date(action.created_at), { addSuffix: true })}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
