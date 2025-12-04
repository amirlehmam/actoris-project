import type { Metadata } from 'next'
import { Inter } from 'next/font/google'
import './globals.css'
import { Providers } from './providers'

const inter = Inter({ subsets: ['latin'] })

export const metadata: Metadata = {
  title: 'ACTORIS - Economic OS for AI Agents',
  description: 'Autonomous Contract-based Trust Operating & Resource Interoperability System',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <Providers>
          <div className="min-h-screen bg-gray-50">
            {/* Header */}
            <header className="bg-white border-b border-gray-200 sticky top-0 z-50">
              <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div className="flex justify-between items-center h-16">
                  <div className="flex items-center">
                    <span className="text-2xl font-bold text-primary-600">ACTORIS</span>
                    <span className="ml-2 text-xs bg-primary-100 text-primary-700 px-2 py-0.5 rounded-full">
                      Economic OS
                    </span>
                  </div>
                  <nav className="flex space-x-8">
                    <a href="/" className="text-gray-900 hover:text-primary-600 font-medium">
                      Dashboard
                    </a>
                    <a href="/agents" className="text-gray-500 hover:text-primary-600 font-medium">
                      Agents
                    </a>
                    <a href="/actions" className="text-gray-500 hover:text-primary-600 font-medium">
                      Actions
                    </a>
                    <a href="/praxis" className="text-gray-500 hover:text-primary-600 font-medium">
                      PRAXIS
                    </a>
                    <a href="/wallet" className="text-gray-500 hover:text-primary-600 font-medium">
                      Wallet
                    </a>
                  </nav>
                </div>
              </div>
            </header>

            {/* Main content */}
            <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
              {children}
            </main>

            {/* Footer */}
            <footer className="bg-white border-t border-gray-200 mt-auto">
              <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
                <p className="text-center text-gray-500 text-sm">
                  ACTORIS Economic OS - Powered by TrustLedger Consensus
                </p>
              </div>
            </footer>
          </div>
        </Providers>
      </body>
    </html>
  )
}
