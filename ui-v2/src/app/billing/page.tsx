'use client'

import { useQuery } from '@tanstack/react-query'
import { apiClient } from '@/lib/api'
import { formatCurrency, formatPercent } from '@/lib/utils'
import { Receipt, Calculator, TrendingDown, DollarSign, Cpu, AlertTriangle, Shield } from 'lucide-react'

export default function OneBillPage() {
  const { data: actions } = useQuery({
    queryKey: ['actions'],
    queryFn: apiClient.getActions,
  })

  // Calculate pricing stats
  const pricingStats = actions?.reduce((acc, action) => {
    acc.totalRevenue += action.pricing.final_price
    acc.totalCompute += action.pricing.base_compute
    acc.totalRisk += action.pricing.risk_premium
    acc.totalDiscount += action.pricing.trust_discount
    return acc
  }, { totalRevenue: 0, totalCompute: 0, totalRisk: 0, totalDiscount: 0 }) || {
    totalRevenue: 1234.56, totalCompute: 987.65, totalRisk: 321.45, totalDiscount: 74.54
  }

  const avgPrice = pricingStats.totalRevenue / (actions?.length || 1)

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold text-gray-900">OneBill</h1>
        <p className="text-gray-500 mt-1">
          Programmable Billing — Outcome-based pricing per verified action
        </p>
      </div>

      {/* Formula Hero */}
      <div className="card p-8 bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900 text-white">
        <div className="text-center">
          <p className="text-gray-400 text-sm mb-4">The OneBill Formula</p>
          <div className="text-4xl font-mono font-bold tracking-wide">
            Price = <span className="text-blue-400">Compute</span> + <span className="text-red-400">Risk</span> - <span className="text-green-400">Trust</span>
          </div>
          <p className="text-gray-400 mt-4 max-w-2xl mx-auto">
            No seat licenses. No API call counts. Pay for verified value delivered.
            High-trust agents get discounts. Low-trust agents pay premium or get culled.
          </p>
        </div>

        <div className="grid grid-cols-3 gap-8 mt-10 pt-8 border-t border-gray-700">
          <div className="text-center">
            <div className="w-12 h-12 mx-auto bg-blue-500/20 rounded-xl flex items-center justify-center mb-3">
              <Cpu className="w-6 h-6 text-blue-400" />
            </div>
            <p className="text-blue-400 font-semibold">Compute</p>
            <p className="text-gray-400 text-sm mt-1">CPU, GPU, Memory measured via eBPF sidecar</p>
          </div>
          <div className="text-center">
            <div className="w-12 h-12 mx-auto bg-red-500/20 rounded-xl flex items-center justify-center mb-3">
              <AlertTriangle className="w-6 h-6 text-red-400" />
            </div>
            <p className="text-red-400 font-semibold">Risk Premium</p>
            <p className="text-gray-400 text-sm mt-1">Complexity +0-50%, Data sensitivity +0-30%</p>
          </div>
          <div className="text-center">
            <div className="w-12 h-12 mx-auto bg-green-500/20 rounded-xl flex items-center justify-center mb-3">
              <Shield className="w-6 h-6 text-green-400" />
            </div>
            <p className="text-green-400 font-semibold">Trust Discount</p>
            <p className="text-gray-400 text-sm mt-1">Up to 20% off based on TrustScore history</p>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-6">
        <div className="stat-card">
          <div className="flex items-center space-x-2 mb-2">
            <DollarSign className="w-5 h-5 text-green-500" />
            <span className="text-sm text-gray-500">Total Billed</span>
          </div>
          <p className="metric-value">{formatCurrency(pricingStats.totalRevenue)}</p>
        </div>
        <div className="stat-card">
          <div className="flex items-center space-x-2 mb-2">
            <Cpu className="w-5 h-5 text-blue-500" />
            <span className="text-sm text-gray-500">Compute Cost</span>
          </div>
          <p className="metric-value">{formatCurrency(pricingStats.totalCompute)}</p>
        </div>
        <div className="stat-card">
          <div className="flex items-center space-x-2 mb-2">
            <AlertTriangle className="w-5 h-5 text-red-500" />
            <span className="text-sm text-gray-500">Risk Premium</span>
          </div>
          <p className="metric-value">{formatCurrency(pricingStats.totalRisk)}</p>
        </div>
        <div className="stat-card">
          <div className="flex items-center space-x-2 mb-2">
            <TrendingDown className="w-5 h-5 text-green-500" />
            <span className="text-sm text-gray-500">Trust Discounts</span>
          </div>
          <p className="metric-value text-green-600">-{formatCurrency(pricingStats.totalDiscount)}</p>
        </div>
      </div>

      {/* Pricing Calculator */}
      <div className="grid grid-cols-2 gap-8">
        <div className="card">
          <div className="card-header">
            <h2 className="font-semibold flex items-center space-x-2">
              <Calculator className="w-5 h-5" />
              <span>Pricing Calculator</span>
            </h2>
          </div>
          <div className="card-body">
            <PricingCalculator />
          </div>
        </div>

        <div className="card">
          <div className="card-header">
            <h2 className="font-semibold flex items-center space-x-2">
              <Receipt className="w-5 h-5" />
              <span>Recent Billing</span>
            </h2>
          </div>
          <div className="card-body p-0">
            <table className="w-full">
              <thead className="bg-gray-50">
                <tr>
                  <th className="text-left text-xs text-gray-500 px-4 py-3">Action</th>
                  <th className="text-right text-xs text-gray-500 px-4 py-3">Compute</th>
                  <th className="text-right text-xs text-gray-500 px-4 py-3">Risk</th>
                  <th className="text-right text-xs text-gray-500 px-4 py-3">Discount</th>
                  <th className="text-right text-xs text-gray-500 px-4 py-3">Final</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {actions?.slice(0, 6).map((action) => (
                  <tr key={action.id} className="hover:bg-gray-50">
                    <td className="px-4 py-3 font-mono text-sm">{action.id.slice(0, 10)}...</td>
                    <td className="px-4 py-3 text-right text-sm text-blue-600">
                      {formatCurrency(action.pricing.base_compute)}
                    </td>
                    <td className="px-4 py-3 text-right text-sm text-red-600">
                      +{formatCurrency(action.pricing.risk_premium)}
                    </td>
                    <td className="px-4 py-3 text-right text-sm text-green-600">
                      -{formatCurrency(action.pricing.trust_discount)}
                    </td>
                    <td className="px-4 py-3 text-right text-sm font-semibold">
                      {formatCurrency(action.pricing.final_price)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      {/* Risk Factors Table */}
      <div className="card">
        <div className="card-header">
          <h2 className="font-semibold">Risk Factor Reference</h2>
        </div>
        <div className="card-body">
          <div className="grid grid-cols-2 gap-8">
            <div>
              <h3 className="font-medium mb-3">Complexity Level</h3>
              <div className="space-y-2">
                {[
                  { level: 'Low', premium: '+0%', color: 'bg-green-100 text-green-700' },
                  { level: 'Medium', premium: '+15%', color: 'bg-yellow-100 text-yellow-700' },
                  { level: 'High', premium: '+30%', color: 'bg-orange-100 text-orange-700' },
                  { level: 'Critical', premium: '+50%', color: 'bg-red-100 text-red-700' },
                ].map((item) => (
                  <div key={item.level} className="flex justify-between items-center p-2 rounded bg-gray-50">
                    <span className="text-sm">{item.level}</span>
                    <span className={`px-2 py-1 rounded text-xs font-medium ${item.color}`}>
                      {item.premium}
                    </span>
                  </div>
                ))}
              </div>
            </div>
            <div>
              <h3 className="font-medium mb-3">Data Sensitivity</h3>
              <div className="space-y-2">
                {[
                  { level: 'Public', premium: '+0%', color: 'bg-green-100 text-green-700' },
                  { level: 'Internal', premium: '+10%', color: 'bg-blue-100 text-blue-700' },
                  { level: 'Confidential', premium: '+20%', color: 'bg-yellow-100 text-yellow-700' },
                  { level: 'Restricted', premium: '+30%', color: 'bg-red-100 text-red-700' },
                ].map((item) => (
                  <div key={item.level} className="flex justify-between items-center p-2 rounded bg-gray-50">
                    <span className="text-sm">{item.level}</span>
                    <span className={`px-2 py-1 rounded text-xs font-medium ${item.color}`}>
                      {item.premium}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

function PricingCalculator() {
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm text-gray-600 mb-1">Base Compute (CPU/GPU/Memory)</label>
        <input
          type="number"
          defaultValue={0.08}
          step={0.01}
          className="w-full px-3 py-2 border rounded-lg"
        />
      </div>
      <div>
        <label className="block text-sm text-gray-600 mb-1">Complexity Level</label>
        <select className="w-full px-3 py-2 border rounded-lg">
          <option>Low (+0%)</option>
          <option>Medium (+15%)</option>
          <option>High (+30%)</option>
          <option>Critical (+50%)</option>
        </select>
      </div>
      <div>
        <label className="block text-sm text-gray-600 mb-1">Data Sensitivity</label>
        <select className="w-full px-3 py-2 border rounded-lg">
          <option>Public (+0%)</option>
          <option>Internal (+10%)</option>
          <option>Confidential (+20%)</option>
          <option>Restricted (+30%)</option>
        </select>
      </div>
      <div>
        <label className="block text-sm text-gray-600 mb-1">Consumer TrustScore</label>
        <input
          type="number"
          defaultValue={750}
          min={0}
          max={1000}
          className="w-full px-3 py-2 border rounded-lg"
        />
      </div>

      <div className="pt-4 border-t">
        <div className="flex justify-between text-sm mb-2">
          <span className="text-gray-600">Compute:</span>
          <span className="text-blue-600">$0.080</span>
        </div>
        <div className="flex justify-between text-sm mb-2">
          <span className="text-gray-600">Risk Premium:</span>
          <span className="text-red-600">+$0.024</span>
        </div>
        <div className="flex justify-between text-sm mb-2">
          <span className="text-gray-600">Trust Discount:</span>
          <span className="text-green-600">-$0.016</span>
        </div>
        <div className="flex justify-between font-semibold pt-2 border-t">
          <span>Final Price:</span>
          <span className="text-lg">$0.088</span>
        </div>
      </div>
    </div>
  )
}
