import type { Config } from 'tailwindcss'

export default {
  content: ['./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}'],
  theme: {
    extend: {
      colors: {
        primary: {
          50:  '#FFF8F2',
          100: '#FFE8D2',
          200: '#FFCBA4',
          300: '#FFB07A',
          400: '#FF8F4F',
          500: '#F07230',
          600: '#E8733A',
          700: '#C85A2C',
          800: '#9E3F1A',
          900: '#6B270E',
          950: '#2D1B0E',
        },
      },
      fontFamily: {
        display: ['Fraunces', 'Georgia', 'serif'],
        mono: ['DM Mono', 'ui-monospace', 'monospace'],
      },
      keyframes: {
        'fade-up': {
          '0%': { opacity: '0', transform: 'translateY(20px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
      },
      animation: {
        'fade-up': 'fade-up 0.5s ease forwards',
      },
    },
  },
  plugins: [],
} satisfies Config
