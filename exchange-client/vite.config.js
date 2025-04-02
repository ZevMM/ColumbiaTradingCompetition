import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  optimizeDeps: {
    include: ['react-plotly.js', 'plotly.js'],
  },
  server: {
    host: '0.0.0.0', // Listen on all available network interfaces
  },
  base: "/ColumbiaTradingCompetition/"
})
