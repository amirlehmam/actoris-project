'use client'

import { useQuery } from '@tanstack/react-query'
import { apiClient, Agent, Loan, InsurancePolicy, Delegation } from '@/lib/api'
import { formatNumber, formatCurrency } from '@/lib/utils'
import {
  GitBranch,
  Banknote,
  Shield,
  FileCheck,
  Users,
  Clock,
  CheckCircle,
  AlertTriangle,
  ArrowRight
} from 'lucide-react'

export default function ProtocolDNAPage() {
  const { data: agents } = useQuery({
    queryKey: ['agents'],
    queryFn: apiClient.getAgents,
  })

  const { data: loans } = useQuery({
    queryKey: ['loans'],
    queryFn: apiClient.getLoans,
  })

  const { data: policies } = useQuery({
    queryKey: ['policies'],
    queryFn: apiClient.getPolicies,
  })

  const { data: delegations } = useQuery({
    queryKey: ['delegations'],
    queryFn: apiClient.getDelegations,
  })

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Protocol DNA</h1>
        <p className="text-gray-500 mt-1">
          Four native primitives that create compounding network effects
        </p>
      </div>

      {/* Primitives Overview */}
      <div className="grid grid-cols-4 gap-6">
        <PrimitiveCard
          icon={<GitBranch className="w-6 h-6" />}
          title="Spawn"
          description="Parent agents create children with 30% trust inheritance"
          color="purple"
          stats={`${agents?.filter(a => a.type === 'agent').length || 0} agents`}
        />
        <PrimitiveCard
          icon={<Banknote className="w-6 h-6" />}
          title="Lend"
          description="Risk-priced credit based on TrustScore (APR: 3.2%)"
          color="green"
          stats={`${loans?.length || 0} active loans`}
        />
        <PrimitiveCard
          icon={<Shield className="w-6 h-6" />}
          title="Insure"
          description="Outcome guarantees with trust-based premiums"
          color="blue"
          stats={`${policies?.length || 0} policies`}
        />
        <PrimitiveCard
          icon={<FileCheck className="w-6 h-6" />}
          title="Delegate"
          description="Escrow + verification for buyer-supplier transactions"
          color="orange"
          stats={`${delegations?.length || 0} delegations`}
        />
      </div>

      {/* Spawn Section */}
      <div className="card">
        <div className="card-header flex items-center space-x-3">
          <div className="p-2 bg-purple-100 rounded-lg">
            <GitBranch className="w-5 h-5 text-purple-600" />
          </div>
          <div>
            <h2 className="font-semibold">Spawn - Agent Lineage</h2>
            <p className="text-sm text-gray-500">Trust inheritance capped at 30% - no free trust transfer</p>
          </div>
        </div>
        <div className="card-body">
          <div className="bg-purple-50 rounded-lg p-4 mb-4">
            <div className="font-mono text-sm text-purple-800">
              child.trust = parent.trust × 0.30
            </div>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-gray-50">
                <tr>
                  <th className="text-left text-xs font-medium text-gray-500 px-4 py-2">Agent</th>
                  <th className="text-left text-xs font-medium text-gray-500 px-4 py-2">Type</th>
                  <th className="text-center text-xs font-medium text-gray-500 px-4 py-2">Trust Score</th>
                  <th className="text-center text-xs font-medium text-gray-500 px-4 py-2">Max Child Trust</th>
                  <th className="text-right text-xs font-medium text-gray-500 px-4 py-2">Verifications</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {agents?.slice(0, 5).map(agent => (
                  <tr key={agent.id} className="hover:bg-gray-50">
                    <td className="px-4 py-3">
                      <div className="font-medium">{agent.name}</div>
                      <div className="text-xs text-gray-500 font-mono">{agent.id.slice(0, 12)}...</div>
                    </td>
                    <td className="px-4 py-3">
                      <span className={`px-2 py-1 text-xs rounded-full ${
                        agent.type === 'agent' ? 'bg-purple-100 text-purple-800' :
                        agent.type === 'human' ? 'bg-blue-100 text-blue-800' :
                        'bg-gray-100 text-gray-800'
                      }`}>
                        {agent.type}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-center">
                      <span className="font-mono font-semibold">{agent.trust_score.score}</span>
                    </td>
                    <td className="px-4 py-3 text-center">
                      <span className="font-mono text-purple-600">
                        {Math.floor(agent.trust_score.score * 0.3)}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-right font-mono">
                      {formatNumber(agent.trust_score.verifications)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      {/* Lend Section */}
      <div className="card">
        <div className="card-header flex items-center space-x-3">
          <div className="p-2 bg-green-100 rounded-lg">
            <Banknote className="w-5 h-5 text-green-600" />
          </div>
          <div>
            <h2 className="font-semibold">Lend - Risk-Priced Credit</h2>
            <p className="text-sm text-gray-500">Interest rates scale inversely with trust score</p>
          </div>
        </div>
        <div className="card-body">
          <div className="bg-green-50 rounded-lg p-4 mb-4">
            <div className="font-mono text-sm text-green-800">
              interest_rate = base_rate × (2.0 - tau)  |  Range: 3.2% to 6.4% APR
            </div>
          </div>
          {loans && loans.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="text-left text-xs font-medium text-gray-500 px-4 py-2">Loan ID</th>
                    <th className="text-right text-xs font-medium text-gray-500 px-4 py-2">Principal</th>
                    <th className="text-center text-xs font-medium text-gray-500 px-4 py-2">APR</th>
                    <th className="text-center text-xs font-medium text-gray-500 px-4 py-2">Term</th>
                    <th className="text-center text-xs font-medium text-gray-500 px-4 py-2">Status</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {loans.map(loan => (
                    <tr key={loan.id} className="hover:bg-gray-50">
                      <td className="px-4 py-3 font-mono text-sm">{loan.id}</td>
                      <td className="px-4 py-3 text-right font-mono">{formatNumber(loan.principal)} HC</td>
                      <td className="px-4 py-3 text-center font-mono text-green-600">
                        {(loan.interest_rate * 100).toFixed(2)}%
                      </td>
                      <td className="px-4 py-3 text-center">{loan.term_days} days</td>
                      <td className="px-4 py-3 text-center">
                        <StatusBadge status={loan.status} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="text-center py-8 text-gray-500">
              No active loans. Credit is extended based on TrustScore.
            </div>
          )}
        </div>
      </div>

      {/* Insure Section */}
      <div className="card">
        <div className="card-header flex items-center space-x-3">
          <div className="p-2 bg-blue-100 rounded-lg">
            <Shield className="w-5 h-5 text-blue-600" />
          </div>
          <div>
            <h2 className="font-semibold">Insure - Outcome Guarantees</h2>
            <p className="text-sm text-gray-500">If a verified task fails, insurance covers the loss</p>
          </div>
        </div>
        <div className="card-body">
          <div className="bg-blue-50 rounded-lg p-4 mb-4">
            <div className="font-mono text-sm text-blue-800">
              premium = coverage × failure_prob × (1.0 + (1.0 - tau))  |  Rate: 5% to 10%
            </div>
          </div>
          {policies && policies.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="text-left text-xs font-medium text-gray-500 px-4 py-2">Policy ID</th>
                    <th className="text-left text-xs font-medium text-gray-500 px-4 py-2">Action Type</th>
                    <th className="text-right text-xs font-medium text-gray-500 px-4 py-2">Coverage</th>
                    <th className="text-right text-xs font-medium text-gray-500 px-4 py-2">Premium</th>
                    <th className="text-center text-xs font-medium text-gray-500 px-4 py-2">Status</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {policies.map(policy => (
                    <tr key={policy.id} className="hover:bg-gray-50">
                      <td className="px-4 py-3 font-mono text-sm">{policy.id}</td>
                      <td className="px-4 py-3">{policy.action_type}</td>
                      <td className="px-4 py-3 text-right font-mono">{formatNumber(policy.coverage)} HC</td>
                      <td className="px-4 py-3 text-right font-mono text-blue-600">
                        {formatNumber(policy.premium)} HC
                      </td>
                      <td className="px-4 py-3 text-center">
                        <StatusBadge status={policy.status} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="text-center py-8 text-gray-500">
              No active policies. Insurance protects against action failures.
            </div>
          )}
        </div>
      </div>

      {/* Delegate Section */}
      <div className="card">
        <div className="card-header flex items-center space-x-3">
          <div className="p-2 bg-orange-100 rounded-lg">
            <FileCheck className="w-5 h-5 text-orange-600" />
          </div>
          <div>
            <h2 className="font-semibold">Delegate - Escrow Transactions</h2>
            <p className="text-sm text-gray-500">Client funds locked in escrow, released after verification</p>
          </div>
        </div>
        <div className="card-body">
          <div className="bg-orange-50 rounded-lg p-4 mb-4">
            <div className="flex items-center space-x-4 text-sm text-orange-800">
              <span className="font-medium">Pending</span>
              <ArrowRight className="w-4 h-4" />
              <span className="font-medium">Active</span>
              <ArrowRight className="w-4 h-4" />
              <span className="font-medium">Completed</span>
              <span className="text-gray-500 ml-4">(or Disputed/Cancelled)</span>
            </div>
          </div>
          {delegations && delegations.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="text-left text-xs font-medium text-gray-500 px-4 py-2">Delegation ID</th>
                    <th className="text-left text-xs font-medium text-gray-500 px-4 py-2">Task</th>
                    <th className="text-right text-xs font-medium text-gray-500 px-4 py-2">Escrow</th>
                    <th className="text-center text-xs font-medium text-gray-500 px-4 py-2">Deadline</th>
                    <th className="text-center text-xs font-medium text-gray-500 px-4 py-2">Status</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {delegations.map(delegation => (
                    <tr key={delegation.id} className="hover:bg-gray-50">
                      <td className="px-4 py-3 font-mono text-sm">{delegation.id}</td>
                      <td className="px-4 py-3 max-w-xs truncate">{delegation.task_description}</td>
                      <td className="px-4 py-3 text-right font-mono">{formatNumber(delegation.escrow_amount)} HC</td>
                      <td className="px-4 py-3 text-center text-sm">
                        {new Date(delegation.deadline).toLocaleDateString()}
                      </td>
                      <td className="px-4 py-3 text-center">
                        <StatusBadge status={delegation.status} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="text-center py-8 text-gray-500">
              No active delegations. Tasks are delegated with escrow protection.
            </div>
          )}
        </div>
      </div>

      {/* Network Effects */}
      <div className="card bg-gradient-to-r from-indigo-900 to-purple-900 text-white">
        <div className="card-body">
          <h3 className="text-lg font-semibold mb-4">Compounding Network Effects</h3>
          <div className="grid grid-cols-2 gap-6">
            <div>
              <p className="text-indigo-200 text-sm mb-2">Why these primitives matter:</p>
              <ul className="space-y-2 text-sm">
                <li className="flex items-start space-x-2">
                  <CheckCircle className="w-4 h-4 text-green-400 mt-0.5" />
                  <span>Programmable and cross-reinforcing</span>
                </li>
                <li className="flex items-start space-x-2">
                  <CheckCircle className="w-4 h-4 text-green-400 mt-0.5" />
                  <span>Built on proprietary trust data</span>
                </li>
                <li className="flex items-start space-x-2">
                  <CheckCircle className="w-4 h-4 text-green-400 mt-0.5" />
                  <span>Non-replicable outside the network</span>
                </li>
              </ul>
            </div>
            <div>
              <p className="text-indigo-200 text-sm mb-2">First-mover advantage:</p>
              <ul className="space-y-2 text-sm">
                <li className="flex items-start space-x-2">
                  <Users className="w-4 h-4 text-blue-400 mt-0.5" />
                  <span>Each new entity improves trust calibration</span>
                </li>
                <li className="flex items-start space-x-2">
                  <Clock className="w-4 h-4 text-yellow-400 mt-0.5" />
                  <span>Network effects compound daily</span>
                </li>
                <li className="flex items-start space-x-2">
                  <AlertTriangle className="w-4 h-4 text-orange-400 mt-0.5" />
                  <span>Winner-take-most dynamics</span>
                </li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

function PrimitiveCard({
  icon,
  title,
  description,
  color,
  stats
}: {
  icon: React.ReactNode
  title: string
  description: string
  color: 'purple' | 'green' | 'blue' | 'orange'
  stats: string
}) {
  const colors = {
    purple: 'bg-purple-100 text-purple-600 border-purple-200',
    green: 'bg-green-100 text-green-600 border-green-200',
    blue: 'bg-blue-100 text-blue-600 border-blue-200',
    orange: 'bg-orange-100 text-orange-600 border-orange-200',
  }

  return (
    <div className={`stat-card border-l-4 ${colors[color].split(' ')[2]}`}>
      <div className={`p-2 rounded-lg ${colors[color].split(' ').slice(0, 2).join(' ')} w-fit mb-3`}>
        {icon}
      </div>
      <h3 className="font-semibold mb-1">{title}</h3>
      <p className="text-xs text-gray-500 mb-2">{description}</p>
      <p className="text-sm font-mono font-semibold">{stats}</p>
    </div>
  )
}

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    active: 'bg-green-100 text-green-800',
    repaid: 'bg-blue-100 text-blue-800',
    defaulted: 'bg-red-100 text-red-800',
    claimed: 'bg-yellow-100 text-yellow-800',
    expired: 'bg-gray-100 text-gray-800',
    pending: 'bg-yellow-100 text-yellow-800',
    completed: 'bg-green-100 text-green-800',
    disputed: 'bg-red-100 text-red-800',
    cancelled: 'bg-gray-100 text-gray-800',
  }

  return (
    <span className={`px-2 py-1 text-xs font-medium rounded-full ${styles[status] || 'bg-gray-100 text-gray-800'}`}>
      {status}
    </span>
  )
}
