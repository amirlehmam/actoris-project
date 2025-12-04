'use client'

import { CheckCircle2, XCircle, Server, Database, Cloud, Radio } from 'lucide-react'
import type { Health } from '@/lib/api'

interface ServiceStatusProps {
  health?: Health
  isLoading: boolean
}

const serviceInfo = {
  identity_cloud: {
    name: 'IdentityCloud',
    description: 'Agent identity management',
    icon: Cloud,
  },
  trustledger: {
    name: 'TrustLedger',
    description: 'Consensus & verification',
    icon: Server,
  },
  onebill: {
    name: 'OneBill',
    description: 'Metering & billing',
    icon: Database,
  },
  darwinian: {
    name: 'Darwinian',
    description: 'PID-based pricing',
    icon: Radio,
  },
  redis: {
    name: 'Redis',
    description: 'Caching layer',
    icon: Database,
  },
  nats: {
    name: 'NATS',
    description: 'Message bus',
    icon: Radio,
  },
}

export function ServiceStatus({ health, isLoading }: ServiceStatusProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-xl shadow-sm p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Service Status</h2>
        <div className="space-y-3">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <div key={i} className="animate-pulse flex items-center space-x-3">
              <div className="h-8 w-8 bg-gray-200 rounded-lg" />
              <div className="flex-1">
                <div className="h-4 w-24 bg-gray-200 rounded mb-1" />
                <div className="h-3 w-32 bg-gray-200 rounded" />
              </div>
            </div>
          ))}
        </div>
      </div>
    )
  }

  const services = health?.services || {}
  const allHealthy = Object.values(services).every(Boolean)

  return (
    <div className="bg-white rounded-xl shadow-sm p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-900">Service Status</h2>
        <span
          className={`px-2 py-1 rounded-full text-xs font-medium ${
            allHealthy ? 'bg-green-100 text-green-700' : 'bg-red-100 text-red-700'
          }`}
        >
          {allHealthy ? 'All Systems Operational' : 'Degraded'}
        </span>
      </div>

      {health?.version && (
        <p className="text-sm text-gray-500 mb-4">Version: {health.version}</p>
      )}

      <div className="space-y-3">
        {Object.entries(serviceInfo).map(([key, info]) => {
          const isHealthy = services[key] ?? false
          const Icon = info.icon

          return (
            <div
              key={key}
              className={`flex items-center justify-between p-3 rounded-lg border ${
                isHealthy ? 'border-green-100 bg-green-50/50' : 'border-red-100 bg-red-50/50'
              }`}
            >
              <div className="flex items-center space-x-3">
                <div className={`p-2 rounded-lg ${isHealthy ? 'bg-green-100' : 'bg-red-100'}`}>
                  <Icon className={`h-4 w-4 ${isHealthy ? 'text-green-600' : 'text-red-600'}`} />
                </div>
                <div>
                  <p className="font-medium text-gray-900 text-sm">{info.name}</p>
                  <p className="text-xs text-gray-500">{info.description}</p>
                </div>
              </div>
              {isHealthy ? (
                <CheckCircle2 className="h-5 w-5 text-green-500" />
              ) : (
                <XCircle className="h-5 w-5 text-red-500" />
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
