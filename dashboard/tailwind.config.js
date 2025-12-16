/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Univrs.io organic bioluminescence palette
        void: '#0a0d0b',
        'deep-earth': '#0f1411',
        'forest-floor': '#141a16',
        moss: '#1a221d',
        bark: '#232d27',
        // Bioluminescent accents
        glow: {
          cyan: '#00ffd5',
          'cyan-dim': 'rgba(0, 255, 213, 0.25)',
          gold: '#ffd700',
          'gold-dim': 'rgba(255, 215, 0, 0.19)',
        },
        spore: {
          purple: '#b088f9',
        },
        mycelium: {
          white: '#e8f4ec',
        },
        'soft-gray': '#8a9a8f',
        'border-subtle': '#2a3a30',
        // Legacy mycelial palette for compatibility
        mycelial: {
          50: '#f0fdf4',
          100: '#dcfce7',
          200: '#bbf7d0',
          300: '#86efac',
          400: '#00ffd5', // Updated to glow-cyan
          500: '#00b8a0',
          600: '#008b75',
          700: '#15803d',
          800: '#166534',
          900: '#14532d',
          950: '#052e16',
        },
        surface: {
          dark: '#0a0d0b', // void
          DEFAULT: '#0f1411', // deep-earth
          light: '#141a16', // forest-floor
        },
      },
      fontFamily: {
        display: ['Syne', 'sans-serif'],
        body: ['Crimson Pro', 'Georgia', 'serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'glow': 'glow 2s ease-in-out infinite alternate',
        'pulse-bg': 'pulse-bg 20s ease-in-out infinite alternate',
      },
      keyframes: {
        glow: {
          '0%': { boxShadow: '0 0 5px rgba(0, 255, 213, 0.25), 0 0 10px rgba(0, 255, 213, 0.25)' },
          '100%': { boxShadow: '0 0 20px rgba(0, 255, 213, 0.25), 0 0 30px rgba(0, 255, 213, 0.25)' },
        },
        'pulse-bg': {
          '0%': { opacity: '0.4', transform: 'scale(1)' },
          '100%': { opacity: '0.7', transform: 'scale(1.1)' },
        },
      },
      boxShadow: {
        'glow-sm': '0 0 10px rgba(0, 255, 213, 0.25)',
        'glow-md': '0 0 20px rgba(0, 255, 213, 0.25), 0 0 40px rgba(0, 255, 213, 0.25)',
        'glow-lg': '0 0 30px rgba(0, 255, 213, 0.25), 0 0 60px rgba(0, 255, 213, 0.25), 0 0 90px rgba(0, 255, 213, 0.25)',
        'card': '0 4px 20px rgba(0, 0, 0, 0.4)',
      },
    },
  },
  plugins: [],
};
