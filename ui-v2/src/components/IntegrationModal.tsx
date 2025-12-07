'use client'

import { useState } from 'react'
import { X, ExternalLink, Check, Loader2, AlertCircle, Zap, Bot, Brain, Server } from 'lucide-react'

interface IntegrationModalProps {
  isOpen: boolean
  onClose: () => void
  onConnect: (config: { provider: string; endpoint: string; apiKey: string }) => Promise<void>
  connectionStatus: 'disconnected' | 'connecting' | 'connected'
}

const INTEGRATIONS = [
  {
    id: 'humain-one',
    name: 'HUMAIN ONE',
    description: 'Multi-agent orchestration platform',
    icon: Brain,
    color: 'from-purple-500 to-indigo-600',
    defaultEndpoint: 'https://api.humainone.com',
    docs: 'https://docs.humainone.com/actoris'
  },
  {
    id: 'grok',
    name: 'Grok (xAI)',
    description: 'xAI reasoning model integration',
    icon: Zap,
    color: 'from-blue-500 to-cyan-600',
    defaultEndpoint: 'https://api.x.ai/v1',
    docs: 'https://docs.x.ai/api'
  },
  {
    id: 'custom',
    name: 'Custom Agent Stack',
    description: 'Connect any agent framework via webhook',
    icon: Server,
    color: 'from-gray-600 to-gray-800',
    defaultEndpoint: '',
    docs: '#'
  }
]

export function IntegrationModal({ isOpen, onClose, onConnect, connectionStatus }: IntegrationModalProps) {
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null)
  const [endpoint, setEndpoint] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [webhookSecret, setWebhookSecret] = useState('')

  if (!isOpen) return null

  const selectedIntegration = INTEGRATIONS.find(i => i.id === selectedProvider)

  const handleProviderSelect = (providerId: string) => {
    setSelectedProvider(providerId)
    const integration = INTEGRATIONS.find(i => i.id === providerId)
    if (integration?.defaultEndpoint) {
      setEndpoint(integration.defaultEndpoint)
    }
  }

  const handleConnect = () => {
    if (selectedProvider && endpoint) {
      onConnect({
        provider: selectedProvider,
        endpoint,
        apiKey
      })
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="bg-gradient-to-r from-gray-900 to-gray-800 text-white p-6">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-xl font-bold">Connect to Agent Stack</h2>
              <p className="text-gray-300 text-sm mt-1">
                Actoris runs as a sidecar to your AI agent infrastructure
              </p>
            </div>
            <button
              onClick={onClose}
              className="p-2 hover:bg-white/10 rounded-lg transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-6">
          {!selectedProvider ? (
            /* Provider Selection */
            <div className="space-y-4">
              <p className="text-sm text-gray-600 mb-4">
                Select an agent platform to integrate with Actoris Economic OS:
              </p>

              <div className="grid gap-4">
                {INTEGRATIONS.map((integration) => {
                  const Icon = integration.icon
                  return (
                    <button
                      key={integration.id}
                      onClick={() => handleProviderSelect(integration.id)}
                      className="flex items-center gap-4 p-4 border-2 border-gray-200 rounded-xl hover:border-actoris-500 hover:bg-actoris-50 transition-all text-left group"
                    >
                      <div className={`w-12 h-12 rounded-xl bg-gradient-to-br ${integration.color} flex items-center justify-center`}>
                        <Icon className="w-6 h-6 text-white" />
                      </div>
                      <div className="flex-1">
                        <h3 className="font-semibold text-gray-900 group-hover:text-actoris-700">
                          {integration.name}
                        </h3>
                        <p className="text-sm text-gray-500">{integration.description}</p>
                      </div>
                      <ExternalLink className="w-4 h-4 text-gray-400 group-hover:text-actoris-500" />
                    </button>
                  )
                })}
              </div>

              {/* Sidecar Architecture Explanation */}
              <div className="mt-6 p-4 bg-gray-50 rounded-xl">
                <h4 className="font-medium text-gray-900 mb-2">How Actoris Sidecar Works</h4>
                <div className="text-sm text-gray-600 space-y-2">
                  <p>
                    Actoris acts as an economic sidecar that wraps your agent stack:
                  </p>
                  <ul className="list-disc list-inside space-y-1 ml-2">
                    <li><strong>Identity:</strong> Every agent gets a UnifiedID + TrustScore</li>
                    <li><strong>Verification:</strong> Actions are verified via 3-of-N oracle consensus</li>
                    <li><strong>Pricing:</strong> Pay-per-action with trust-based discounts</li>
                    <li><strong>Allocation:</strong> Resources flow to high-performing agents</li>
                  </ul>
                </div>
              </div>
            </div>
          ) : (
            /* Configuration Form */
            <div className="space-y-6">
              <button
                onClick={() => setSelectedProvider(null)}
                className="text-sm text-actoris-600 hover:text-actoris-700 flex items-center gap-1"
              >
                &larr; Back to integrations
              </button>

              <div className="flex items-center gap-4 p-4 bg-gray-50 rounded-xl">
                {selectedIntegration && (
                  <>
                    <div className={`w-12 h-12 rounded-xl bg-gradient-to-br ${selectedIntegration.color} flex items-center justify-center`}>
                      <selectedIntegration.icon className="w-6 h-6 text-white" />
                    </div>
                    <div>
                      <h3 className="font-semibold text-gray-900">{selectedIntegration.name}</h3>
                      <p className="text-sm text-gray-500">{selectedIntegration.description}</p>
                    </div>
                  </>
                )}
              </div>

              {/* Connection Form */}
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    API Endpoint
                  </label>
                  <input
                    type="url"
                    value={endpoint}
                    onChange={(e) => setEndpoint(e.target.value)}
                    placeholder="https://api.example.com"
                    className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-actoris-500 focus:border-actoris-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">
                    The base URL of your agent stack API
                  </p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    API Key
                  </label>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder="sk-..."
                    className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-actoris-500 focus:border-actoris-500"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Webhook Secret (optional)
                  </label>
                  <input
                    type="password"
                    value={webhookSecret}
                    onChange={(e) => setWebhookSecret(e.target.value)}
                    placeholder="whsec_..."
                    className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-actoris-500 focus:border-actoris-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">
                    For receiving real-time agent events
                  </p>
                </div>

                {/* Webhook Info */}
                <div className="p-4 bg-blue-50 border border-blue-200 rounded-xl">
                  <h4 className="font-medium text-blue-900 mb-2 flex items-center gap-2">
                    <AlertCircle className="w-4 h-4" />
                    Actoris Ingest Endpoints
                  </h4>
                  <div className="text-sm text-blue-800 font-mono space-y-1">
                    <p>POST /ingest/agent - Register new agents</p>
                    <p>POST /ingest/action - Submit actions for verification</p>
                    <p>POST /ingest/batch - Batch import</p>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="border-t border-gray-200 p-6 bg-gray-50 flex justify-between items-center">
          <a
            href={selectedIntegration?.docs || '#'}
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-gray-600 hover:text-actoris-600 flex items-center gap-1"
          >
            <ExternalLink className="w-4 h-4" />
            View documentation
          </a>

          <div className="flex gap-3">
            <button
              onClick={onClose}
              className="px-4 py-2 text-gray-700 hover:bg-gray-200 rounded-lg transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleConnect}
              disabled={!selectedProvider || !endpoint || connectionStatus === 'connecting'}
              className="px-6 py-2 bg-actoris-600 text-white rounded-lg hover:bg-actoris-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2 transition-colors"
            >
              {connectionStatus === 'connecting' ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Connecting...
                </>
              ) : (
                <>
                  <Check className="w-4 h-4" />
                  Connect
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
