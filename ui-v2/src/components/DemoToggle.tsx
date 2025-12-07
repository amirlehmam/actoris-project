'use client'

import { useDemoMode } from '@/lib/demo-context'
import { RefreshCw, Wifi, WifiOff } from 'lucide-react'

export function DemoToggle() {
  const { isDemoMode, toggleDemoMode, refreshMockData } = useDemoMode()

  return (
    <div className="flex items-center gap-2">
      {isDemoMode && (
        <button
          onClick={refreshMockData}
          className="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
          title="Refresh demo data"
        >
          <RefreshCw className="w-4 h-4" />
        </button>
      )}

      <button
        onClick={toggleDemoMode}
        className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-all ${
          isDemoMode
            ? 'bg-actoris-100 text-actoris-700 hover:bg-actoris-200'
            : 'bg-green-100 text-green-700 hover:bg-green-200'
        }`}
      >
        {isDemoMode ? (
          <>
            <WifiOff className="w-4 h-4" />
            <span>Demo</span>
          </>
        ) : (
          <>
            <Wifi className="w-4 h-4" />
            <span>Live</span>
          </>
        )}
      </button>
    </div>
  )
}
