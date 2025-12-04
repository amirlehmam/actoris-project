'use client'

import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Send, CheckCircle2, Clock, XCircle, AlertTriangle, Loader2, Eye } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'
import { apiClient, Action } from '@/lib/api'

const statusConfig = {
  pending: { icon: Clock, color: 'text-gray-500', bgColor: 'bg-gray-100', label: 'Pending' },
  processing: { icon: Loader2, color: 'text-blue-500', bgColor: 'bg-blue-100', label: 'Processing' },
  verified: { icon: CheckCircle2, color: 'text-green-500', bgColor: 'bg-green-100', label: 'Verified' },
  disputed: { icon: AlertTriangle, color: 'text-amber-500', bgColor: 'bg-amber-100', label: 'Disputed' },
  settled: { icon: CheckCircle2, color: 'text-blue-500', bgColor: 'bg-blue-100', label: 'Settled' },
  failed: { icon: XCircle, color: 'text-red-500', bgColor: 'bg-red-100', label: 'Failed' },
}

export default function ActionsPage() {
  const [showSubmitModal, setShowSubmitModal] = useState(false)
  const [showDetailModal, setShowDetailModal] = useState<Action | null>(null)
  const [producerId, setProducerId] = useState('')
  const [consumerId, setConsumerId] = useState('')
  const [actionType, setActionType] = useState('inference')
  const [inputData, setInputData] = useState('')
  const [outputData, setOutputData] = useState('')
  const [filterStatus, setFilterStatus] = useState<string>('all')

  const queryClient = useQueryClient()

  const { data: actions, isLoading } = useQuery({
    queryKey: ['actions'],
    queryFn: apiClient.getActions,
  })

  const { data: agents } = useQuery({
    queryKey: ['agents'],
    queryFn: apiClient.getAgents,
  })

  const submitMutation = useMutation({
    mutationFn: apiClient.submitAction,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['actions'] })
      setShowSubmitModal(false)
      setInputData('')
    },
  })

  const verifyMutation = useMutation({
    mutationFn: ({ actionId, outputData }: { actionId: string; outputData: string }) =>
      apiClient.verifyAction(actionId, outputData),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['actions'] })
      queryClient.invalidateQueries({ queryKey: ['agents'] })
      setShowDetailModal(null)
      setOutputData('')
    },
  })

  const filteredActions = actions?.filter((action) =>
    filterStatus === 'all' || action.status === filterStatus
  ) || []

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Actions</h1>
          <p className="mt-1 text-gray-500">Submit and verify agent actions with FROST consensus</p>
        </div>
        <button
          onClick={() => setShowSubmitModal(true)}
          className="inline-flex items-center px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
        >
          <Send className="h-4 w-4 mr-2" />
          Submit Action
        </button>
      </div>

      {/* Filters */}
      <div className="flex space-x-2">
        {['all', 'pending', 'processing', 'verified', 'disputed', 'settled'].map((status) => (
          <button
            key={status}
            onClick={() => setFilterStatus(status)}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              filterStatus === status
                ? 'bg-primary-100 text-primary-700'
                : 'bg-white text-gray-600 hover:bg-gray-100'
            }`}
          >
            {status.charAt(0).toUpperCase() + status.slice(1)}
          </button>
        ))}
      </div>

      {/* Actions List */}
      <div className="bg-white rounded-xl shadow-sm overflow-hidden">
        {isLoading ? (
          <div className="p-6 space-y-4">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="animate-pulse flex items-center space-x-4">
                <div className="h-10 w-10 bg-gray-200 rounded-lg" />
                <div className="flex-1">
                  <div className="h-4 w-48 bg-gray-200 rounded mb-2" />
                  <div className="h-3 w-32 bg-gray-200 rounded" />
                </div>
              </div>
            ))}
          </div>
        ) : filteredActions.length === 0 ? (
          <div className="text-center py-12">
            <Send className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-500">No actions found. Submit your first action!</p>
          </div>
        ) : (
          <table className="w-full">
            <thead className="bg-gray-50 border-b">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Action ID</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Type</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Producer</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Consumer</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Price</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Time</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {filteredActions.map((action) => {
                const status = statusConfig[action.status] || statusConfig.pending
                const Icon = status.icon

                return (
                  <tr key={action.id} className="hover:bg-gray-50">
                    <td className="px-6 py-4">
                      <span className="font-mono text-sm text-gray-600">{action.id.slice(0, 12)}...</span>
                    </td>
                    <td className="px-6 py-4">
                      <span className="px-2 py-1 bg-gray-100 text-gray-700 text-sm rounded">
                        {action.action_type}
                      </span>
                    </td>
                    <td className="px-6 py-4">
                      <span className="font-mono text-sm text-gray-600">{action.producer_id.slice(0, 8)}...</span>
                    </td>
                    <td className="px-6 py-4">
                      <span className="font-mono text-sm text-gray-600">{action.consumer_id.slice(0, 8)}...</span>
                    </td>
                    <td className="px-6 py-4">
                      <span className="font-medium">{action.price ? `${parseFloat(action.price).toFixed(2)} HC` : '-'}</span>
                    </td>
                    <td className="px-6 py-4">
                      <span className={`inline-flex items-center space-x-1 px-2 py-1 rounded-full text-sm ${status.bgColor} ${status.color}`}>
                        <Icon className={`h-3.5 w-3.5 ${action.status === 'processing' ? 'animate-spin' : ''}`} />
                        <span>{status.label}</span>
                      </span>
                    </td>
                    <td className="px-6 py-4 text-sm text-gray-500">
                      {formatDistanceToNow(new Date(action.created_at), { addSuffix: true })}
                    </td>
                    <td className="px-6 py-4">
                      <button
                        onClick={() => setShowDetailModal(action)}
                        className="text-primary-600 hover:text-primary-700"
                      >
                        <Eye className="h-5 w-5" />
                      </button>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        )}
      </div>

      {/* Submit Action Modal */}
      {showSubmitModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl shadow-xl w-full max-w-lg mx-4">
            <div className="p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">Submit New Action</h3>
              <form
                onSubmit={(e) => {
                  e.preventDefault()
                  submitMutation.mutate({
                    producer_id: producerId,
                    consumer_id: consumerId,
                    action_type: actionType,
                    input_data: inputData,
                  })
                }}
                className="space-y-4"
              >
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Producer</label>
                    <select
                      value={producerId}
                      onChange={(e) => setProducerId(e.target.value)}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg"
                      required
                    >
                      <option value="">Select agent</option>
                      {agents?.map((agent) => (
                        <option key={agent.id} value={agent.id}>{agent.name}</option>
                      ))}
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Consumer</label>
                    <select
                      value={consumerId}
                      onChange={(e) => setConsumerId(e.target.value)}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg"
                      required
                    >
                      <option value="">Select agent</option>
                      {agents?.map((agent) => (
                        <option key={agent.id} value={agent.id}>{agent.name}</option>
                      ))}
                    </select>
                  </div>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Action Type</label>
                  <select
                    value={actionType}
                    onChange={(e) => setActionType(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg"
                  >
                    <option value="inference">Inference</option>
                    <option value="training">Training</option>
                    <option value="data_processing">Data Processing</option>
                    <option value="api_call">API Call</option>
                    <option value="custom">Custom</option>
                  </select>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Input Data</label>
                  <textarea
                    value={inputData}
                    onChange={(e) => setInputData(e.target.value)}
                    placeholder="Enter the input data for this action..."
                    rows={4}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg"
                    required
                  />
                </div>

                <div className="flex space-x-3 pt-4">
                  <button
                    type="button"
                    onClick={() => setShowSubmitModal(false)}
                    className="flex-1 px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={submitMutation.isPending}
                    className="flex-1 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50"
                  >
                    {submitMutation.isPending ? 'Submitting...' : 'Submit Action'}
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      )}

      {/* Action Detail Modal */}
      {showDetailModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl shadow-xl w-full max-w-2xl mx-4 max-h-[90vh] overflow-y-auto">
            <div className="p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">Action Details</h3>

              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="text-sm text-gray-500">Action ID</label>
                    <p className="font-mono text-sm">{showDetailModal.id}</p>
                  </div>
                  <div>
                    <label className="text-sm text-gray-500">Status</label>
                    <p className="font-medium">{showDetailModal.status}</p>
                  </div>
                  <div>
                    <label className="text-sm text-gray-500">Input Hash</label>
                    <p className="font-mono text-xs break-all">{showDetailModal.input_hash}</p>
                  </div>
                  <div>
                    <label className="text-sm text-gray-500">Output Hash</label>
                    <p className="font-mono text-xs break-all">{showDetailModal.output_hash || 'Not verified'}</p>
                  </div>
                </div>

                {showDetailModal.verification_proof && (
                  <div className="p-4 bg-green-50 rounded-lg">
                    <h4 className="font-semibold text-green-800 mb-2">Verification Proof</h4>
                    <p className="text-sm text-green-700 mb-2">
                      Quorum: {showDetailModal.verification_proof.quorum_reached ? 'Reached' : 'Not reached'}
                    </p>
                    <p className="text-xs font-mono text-green-600 break-all">
                      Signature: {showDetailModal.verification_proof.aggregate_signature.slice(0, 32)}...
                    </p>
                    <div className="mt-3">
                      <p className="text-sm text-green-700 mb-1">Oracle Votes:</p>
                      <div className="grid grid-cols-5 gap-2">
                        {showDetailModal.verification_proof.oracle_votes.map((vote, i) => (
                          <div
                            key={i}
                            className={`p-2 rounded text-center text-xs ${
                              vote.vote ? 'bg-green-200 text-green-800' : 'bg-red-200 text-red-800'
                            }`}
                          >
                            {vote.oracle_id.slice(-1)}
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                )}

                {(showDetailModal.status === 'pending' || showDetailModal.status === 'processing') && (
                  <div className="p-4 bg-blue-50 rounded-lg">
                    <h4 className="font-semibold text-blue-800 mb-2">Verify This Action</h4>
                    <form
                      onSubmit={(e) => {
                        e.preventDefault()
                        verifyMutation.mutate({
                          actionId: showDetailModal.id,
                          outputData,
                        })
                      }}
                    >
                      <textarea
                        value={outputData}
                        onChange={(e) => setOutputData(e.target.value)}
                        placeholder="Enter the output data for verification..."
                        rows={3}
                        className="w-full px-3 py-2 border border-blue-200 rounded-lg mb-3"
                        required
                      />
                      <button
                        type="submit"
                        disabled={verifyMutation.isPending}
                        className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                      >
                        {verifyMutation.isPending ? 'Verifying...' : 'Submit Verification'}
                      </button>
                    </form>
                  </div>
                )}
              </div>

              <button
                onClick={() => setShowDetailModal(null)}
                className="mt-6 w-full px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
