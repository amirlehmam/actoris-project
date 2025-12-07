'use client'

import { DemoToggle } from './DemoToggle'

export function Header() {
  return (
    <header className="fixed top-0 right-0 left-64 h-16 bg-white border-b border-gray-200 flex items-center justify-end px-8 z-10">
      <DemoToggle />
    </header>
  )
}
