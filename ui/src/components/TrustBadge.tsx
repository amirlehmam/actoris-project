'use client'

import { Shield, ShieldCheck, ShieldAlert, ShieldX } from 'lucide-react'
import type { TrustScore } from '@/lib/api'

interface TrustBadgeProps {
  trustScore: TrustScore
  showDetails?: boolean
}

const tierConfig = {
  0: {
    icon: ShieldX,
    label: 'Untrusted',
    color: 'text-red-500',
    bgColor: 'bg-red-50',
    borderColor: 'border-red-200',
    glowClass: 'trust-glow-low',
  },
  1: {
    icon: ShieldAlert,
    label: 'Basic',
    color: 'text-amber-500',
    bgColor: 'bg-amber-50',
    borderColor: 'border-amber-200',
    glowClass: 'trust-glow-medium',
  },
  2: {
    icon: Shield,
    label: 'Verified',
    color: 'text-green-500',
    bgColor: 'bg-green-50',
    borderColor: 'border-green-200',
    glowClass: 'trust-glow-high',
  },
  3: {
    icon: ShieldCheck,
    label: 'Trusted',
    color: 'text-blue-500',
    bgColor: 'bg-blue-50',
    borderColor: 'border-blue-200',
    glowClass: 'trust-glow-max',
  },
}

export function TrustBadge({ trustScore, showDetails = false }: TrustBadgeProps) {
  const tier = tierConfig[trustScore.tier as keyof typeof tierConfig] || tierConfig[0]
  const Icon = tier.icon
  const percentage = Math.round(trustScore.tau * 100)

  if (showDetails) {
    return (
      <div className={`p-4 rounded-xl border ${tier.bgColor} ${tier.borderColor} ${tier.glowClass}`}>
        <div className="flex items-center space-x-3">
          <div className={`p-2 rounded-lg ${tier.bgColor}`}>
            <Icon className={`h-6 w-6 ${tier.color}`} />
          </div>
          <div>
            <div className="flex items-center space-x-2">
              <span className={`text-lg font-bold ${tier.color}`}>{tier.label}</span>
              <span className="text-sm text-gray-500">Tier {trustScore.tier}</span>
            </div>
            <div className="flex items-center space-x-4 mt-1 text-sm text-gray-600">
              <span>Score: {trustScore.raw_score}/1000</span>
              <span>·</span>
              <span>τ = {trustScore.tau.toFixed(3)}</span>
            </div>
          </div>
        </div>

        {/* Trust bar */}
        <div className="mt-4">
          <div className="flex justify-between text-xs text-gray-500 mb-1">
            <span>Trust Level</span>
            <span>{percentage}%</span>
          </div>
          <div className="h-2 bg-gray-200 rounded-full overflow-hidden">
            <div
              className={`h-full rounded-full transition-all duration-500 ${
                percentage > 75
                  ? 'bg-blue-500'
                  : percentage > 50
                  ? 'bg-green-500'
                  : percentage > 25
                  ? 'bg-amber-500'
                  : 'bg-red-500'
              }`}
              style={{ width: `${percentage}%` }}
            />
          </div>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-2 gap-4 mt-4 text-sm">
          <div className="text-center p-2 bg-white/50 rounded-lg">
            <div className="font-semibold text-green-600">{trustScore.verifications}</div>
            <div className="text-gray-500 text-xs">Verifications</div>
          </div>
          <div className="text-center p-2 bg-white/50 rounded-lg">
            <div className="font-semibold text-red-600">{trustScore.disputes}</div>
            <div className="text-gray-500 text-xs">Disputes</div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className={`flex items-center space-x-2 px-3 py-1.5 rounded-full ${tier.bgColor} ${tier.borderColor} border`}>
      <Icon className={`h-4 w-4 ${tier.color}`} />
      <span className={`text-sm font-medium ${tier.color}`}>{percentage}%</span>
    </div>
  )
}
