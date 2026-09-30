import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    coverage: {
      provider: 'v8',
      include: ['src/**/*.{ts,tsx}'],
      exclude: ['src/main.tsx'],
      reporter: ['text', 'json-summary'],
      thresholds: {
        statements: 67,
        branches: 55,
        functions: 55,
        lines: 71,
      },
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:5030',
        changeOrigin: true,
      }
    }
  }
})
