'use client'

import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Wallet, ArrowUpRight, ArrowDownLeft, Lock, Clock, TrendingUp, TrendingDown } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'
import { apiClient, Transaction } from '@/lib/api'

const txTypeConfig: Record<string, { icon: any; color: string; label: string; sign: '+' | '-' }> = {
  deposit: { icon: ArrowDownLeft, color: 'text-green-500', label: 'Deposit', sign: '+' },
  withdrawal: { icon: ArrowUpRight, color: 'text-red-500', label: 'Withdrawal', sign: '-' },
  action_payment: { icon: ArrowUpRight, color: 'text-red-500', label: 'Action Payment', sign: '-' },
  verification_reward: { icon: ArrowDownLeft, color: 'text-green-500', label: 'Verification Reward', sign: '+' },
  dispute_penalty: { icon: ArrowUpRight, color: 'text-red-500', label: 'Dispute Penalty', sign: '-' },
  stake: { icon: Lock, color: 'text-amber-500', label: 'Stake', sign: '-' },
  unstake: { icon: Lock, color: 'text-blue-500', label: 'Unstake', sign: '+' },
}

export default function WalletPage() {
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null)
  const [depositAmount, setDepositAmount] = useState('')

  const queryClient = useQueryClient()

  const { data: agents, isLoading: agentsLoading } = useQuery({
    queryKey: ['agents'],
    queryFn: apiClient.getAgents,
  })

  const { data: wallet, isLoading: walletLoading } = useQuery({
    queryKey: ['wallet', selectedAgentId],
    queryFn: () => selectedAgentId ? apiClient.getWallet(selectedAgentId) : null,
    enabled: !!selectedAgentId,
  })

  const depositMutation = useMutation({
    mutationFn: ({ agentId, amount }: { agentId: string; amount: string }) =>
      apiClient.deposit(agentId, amount),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['wallet', selectedAgentId] })
      setDepositAmount('')
    },
  })

  const selectedAgent = agents?.find((a) => a.id === selectedAgentId)

  // Calculate totals across all agents
  const totalBalance = wallet ? parseFloat(wallet.balance) : 0
  const totalLocked = wallet ? parseFloat(wallet.locked) : 0
  const totalPending = wallet ? parseFloat(wallet.pending) : 0

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Wallet</h1>
        <p className="mt-1 text-gray-500">Manage your HC (Harness Credits) balance and transactions</p>
      </div>

      {/* Agent Selector */}
      <div className="bg-white rounded-xl shadow-sm p-6">
        <h3 className="text-sm font-medium text-gray-700 mb-3">Select Agent</h3>
        {agentsLoading ? (
          <div className="animate-pulse h-10 bg-gray-200 rounded-lg" />
        ) : agents && agents.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {agents.map((agent) => (
              <button
                key={agent.id}
                onClick={() => setSelectedAgentId(agent.id)}
                className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                  selectedAgentId === agent.id
                    ? 'bg-primary-600 text-white'
                    : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                }`}
              >
                {agent.name}
              </button>
            ))}
          </div>
        ) : (
          <p className="text-gray-500">No agents found. Create an agent first!</p>
        )}
      </div>

      {selectedAgentId && wallet && (
        <>
          {/* Balance Cards */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="bg-white rounded-xl shadow-sm p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="p-3 bg-green-100 rounded-lg">
                  <Wallet className="h-6 w-6 text-green-600" />
                </div>
                <TrendingUp className="h-5 w-5 text-green-500" />
              </div>
              <p className="text-gray-500 text-sm">Available Balance</p>
              <p className="text-3xl font-bold text-gray-900 mt-1">
                {totalBalance.toFixed(2)} <span className="text-lg text-gray-500">HC</span>
              </p>
            </div>

            <div className="bg-white rounded-xl shadow-sm p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="p-3 bg-amber-100 rounded-lg">
                  <Lock className="h-6 w-6 text-amber-600" />
                </div>
              </div>
              <p className="text-gray-500 text-sm">Locked (Staked)</p>
              <p className="text-3xl font-bold text-gray-900 mt-1">
                {totalLocked.toFixed(2)} <span className="text-lg text-gray-500">HC</span>
              </p>
            </div>

            <div className="bg-white rounded-xl shadow-sm p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="p-3 bg-blue-100 rounded-lg">
                  <Clock className="h-6 w-6 text-blue-600" />
                </div>
              </div>
              <p className="text-gray-500 text-sm">Pending</p>
              <p className="text-3xl font-bold text-gray-900 mt-1">
                {totalPending.toFixed(2)} <span className="text-lg text-gray-500">HC</span>
              </p>
            </div>
          </div>

          {/* Deposit Section */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <h3 className="font-semibold text-gray-900 mb-4">Deposit HC</h3>
            <form
              onSubmit={(e) => {
                e.preventDefault()
                if (depositAmount && selectedAgentId) {
                  depositMutation.mutate({
                    agentId: selectedAgentId,
                    amount: depositAmount,
                  })
                }
              }}
              className="flex space-x-4"
            >
              <div className="flex-1 relative">
                <input
                  type="number"
                  step="0.01"
                  min="0"
                  value={depositAmount}
                  onChange={(e) => setDepositAmount(e.target.value)}
                  placeholder="Enter amount"
                  className="w-full px-4 py-3 border border-gray-300 rounded-lg pr-16"
                  required
                />
                <span className="absolute right-4 top-1/2 transform -translate-y-1/2 text-gray-500 font-medium">
                  HC
                </span>
              </div>
              <button
                type="submit"
                disabled={depositMutation.isPending}
                className="px-8 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 font-medium"
              >
                {depositMutation.isPending ? 'Processing...' : 'Deposit'}
              </button>
            </form>
            <p className="text-sm text-gray-500 mt-2">
              Note: In production, deposits would require on-chain verification
            </p>
          </div>

          {/* Transaction History */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <h3 className="font-semibold text-gray-900 mb-4">Transaction History</h3>
            {wallet.transactions.length === 0 ? (
              <div className="text-center py-12">
                <Clock className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-500">No transactions yet</p>
              </div>
            ) : (
              <div className="space-y-3">
                {wallet.transactions.slice().reverse().map((tx) => {
                  const config = txTypeConfig[tx.tx_type] || {
                    icon: Clock,
                    color: 'text-gray-500',
                    label: tx.tx_type,
                    sign: '+'
                  }
                  const Icon = config.icon

                  return (
                    <div
                      key={tx.id}
                      className="flex items-center justify-between p-4 border border-gray-100 rounded-lg hover:bg-gray-50"
                    >
                      <div className="flex items-center space-x-4">
                        <div className={`p-2 rounded-lg ${
                          config.sign === '+' ? 'bg-green-100' : 'bg-red-100'
                        }`}>
                          <Icon className={`h-5 w-5 ${config.color}`} />
                        </div>
                        <div>
                          <p className="font-medium text-gray-900">{config.label}</p>
                          <p className="text-sm text-gray-500">
                            {tx.from && <span>From: {tx.from.slice(0, 8)}...</span>}
                            {tx.to && <span> To: {tx.to.slice(0, 8)}...</span>}
                            {tx.action_id && <span> Action: {tx.action_id.slice(0, 8)}...</span>}
                          </p>
                        </div>
                      </div>
                      <div className="text-right">
                        <p className={`font-bold ${
                          config.sign === '+' ? 'text-green-600' : 'text-red-600'
                        }`}>
                          {config.sign}{parseFloat(tx.amount).toFixed(2)} HC
                        </p>
                        <p className="text-sm text-gray-500">
                          {formatDistanceToNow(new Date(tx.timestamp), { addSuffix: true })}
                        </p>
                      </div>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        </>
      )}

      {/* HC Info */}
      <div className="bg-gradient-to-r from-primary-600 to-primary-700 rounded-xl shadow-sm p-6 text-white">
        <h3 className="font-semibold text-lg mb-2">What are Harness Credits (HC)?</h3>
        <p className="text-primary-100 mb-4">
          HC is the compute-native currency in ACTORIS, backed by actual computational resources (PFLOP-hours).
        </p>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
          <div className="bg-white/10 rounded-lg p-3">
            <p className="font-medium">1 HC = 1 PFLOP-hour</p>
            <p className="text-primary-200">Base compute unit</p>
          </div>
          <div className="bg-white/10 rounded-lg p-3">
            <p className="font-medium">Trust Discount: up to 20%</p>
            <p className="text-primary-200">Higher trust = lower costs</p>
          </div>
          <div className="bg-white/10 rounded-lg p-3">
            <p className="font-medium">Instant Settlement</p>
            <p className="text-primary-200">Via FROST signatures</p>
          </div>
        </div>
      </div>
    </div>
  )
}
