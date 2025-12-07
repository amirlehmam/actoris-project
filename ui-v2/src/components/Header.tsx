'use client'

import { useState } from 'react'
import { useDemoMode } from '@/lib/demo-context'
import { RefreshCw, Wifi, WifiOff, Plug, Settings, ExternalLink, Check, X, Loader2 } from 'lucide-react'
import { IntegrationModal } from './IntegrationModal'

export function Header() {
  const { isDemoMode, toggleDemoMode, refreshMockData } = useDemoMode()
  const [showIntegration, setShowIntegration] = useState(false)
  const [connectionStatus, setConnectionStatus] = useState<'disconnected' | 'connecting' | 'connected'>('disconnected')

  const handleConnect = async (config: { provider: string; endpoint: string; apiKey: string }) => {
    setConnectionStatus('connecting')

    // Simulate connection attempt
    await new Promise(resolve => setTimeout(resolve, 1500))

    // In real implementation, this would call the backend
    // For now, just set connected if we have an endpoint
    if (config.endpoint) {
      setConnectionStatus('connected')
      // Auto-switch to live mode when connected
      if (isDemoMode) {
        toggleDemoMode()
      }
    } else {
      setConnectionStatus('disconnected')
    }

    setShowIntegration(false)
  }

  return (
    <>
      <header className="fixed top-0 right-0 left-64 h-16 bg-white border-b border-gray-200 flex items-center justify-between px-8 z-10">
        {/* Left side - Connection Status */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            {connectionStatus === 'connected' ? (
              <span className="flex items-center gap-2 text-sm text-green-600">
                <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
                Connected to HUMAIN ONE
              </span>
            ) : connectionStatus === 'connecting' ? (
              <span className="flex items-center gap-2 text-sm text-yellow-600">
                <Loader2 className="w-4 h-4 animate-spin" />
                Connecting...
              </span>
            ) : (
              <span className="flex items-center gap-2 text-sm text-gray-500">
                <span className="w-2 h-2 bg-gray-400 rounded-full" />
                {isDemoMode ? 'Demo Mode' : 'Disconnected'}
              </span>
            )}
          </div>
        </div>

        {/* Right side - Controls */}
        <div className="flex items-center gap-3">
          {/* Refresh Demo Data */}
          {isDemoMode && (
            <button
              onClick={refreshMockData}
              className="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
              title="Refresh demo data"
            >
              <RefreshCw className="w-4 h-4" />
            </button>
          )}

          {/* Demo/Live Toggle */}
          <button
            onClick={toggleDemoMode}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              isDemoMode
                ? 'bg-actoris-100 text-actoris-700 hover:bg-actoris-200 border border-actoris-300'
                : 'bg-green-100 text-green-700 hover:bg-green-200 border border-green-300'
            }`}
          >
            {isDemoMode ? (
              <>
                <WifiOff className="w-4 h-4" />
                <span>Demo Mode</span>
              </>
            ) : (
              <>
                <Wifi className="w-4 h-4" />
                <span>Live Mode</span>
              </>
            )}
          </button>

          {/* Integration Button */}
          <button
            onClick={() => setShowIntegration(true)}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              connectionStatus === 'connected'
                ? 'bg-green-600 text-white hover:bg-green-700'
                : 'bg-gray-900 text-white hover:bg-gray-800'
            }`}
          >
            <Plug className="w-4 h-4" />
            <span>Integrations</span>
            {connectionStatus === 'connected' && (
              <Check className="w-4 h-4" />
            )}
          </button>
        </div>
      </header>

      {/* Integration Modal */}
      <IntegrationModal
        isOpen={showIntegration}
        onClose={() => setShowIntegration(false)}
        onConnect={handleConnect}
        connectionStatus={connectionStatus}
      />
    </>
  )
}
