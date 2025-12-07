import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatNumber(num: number, decimals = 2): string {
  if (num >= 1_000_000) return `${(num / 1_000_000).toFixed(decimals)}M`
  if (num >= 1_000) return `${(num / 1_000).toFixed(decimals)}K`
  return num.toFixed(decimals)
}

export function formatCurrency(num: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
  }).format(num)
}

export function formatPercent(num: number): string {
  return `${(num * 100).toFixed(2)}%`
}

export function getTrustTier(score: number): { tier: number; label: string; color: string } {
  if (score >= 751) return { tier: 3, label: 'Elite', color: 'text-green-600' }
  if (score >= 501) return { tier: 2, label: 'Trusted', color: 'text-blue-600' }
  if (score >= 251) return { tier: 1, label: 'Standard', color: 'text-yellow-600' }
  return { tier: 0, label: 'Probation', color: 'text-red-600' }
}

export function getFitnessColor(classification: string): string {
  switch (classification) {
    case 'champion': return 'text-green-600 bg-green-50'
    case 'neutral': return 'text-yellow-600 bg-yellow-50'
    case 'underperformer': return 'text-red-600 bg-red-50'
    default: return 'text-gray-600 bg-gray-50'
  }
}

export function truncateId(id: string, length = 8): string {
  if (id.length <= length) return id
  return `${id.slice(0, length)}...`
}
