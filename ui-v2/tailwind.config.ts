import type { Config } from 'tailwindcss'

const config: Config = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        // Actoris brand colors
        actoris: {
          50: '#fef7ee',
          100: '#fcecd6',
          200: '#f8d5ac',
          300: '#f3b778',
          400: '#ed8f41',
          500: '#e97316', // Primary orange
          600: '#db5c0c',
          700: '#b6440c',
          800: '#913712',
          900: '#752f12',
        },
        trust: {
          champion: '#22c55e',
          neutral: '#eab308',
          warning: '#f97316',
          culled: '#ef4444',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [],
}
export default config
