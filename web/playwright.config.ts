import { defineConfig } from '@playwright/test';
import { shouldLaunchNodes } from './e2e/env';

export default defineConfig({
  // Increased for node startup and login flows
  expect: { timeout: 20_000 },

  // Increased for slower React rendering
  retries: process.env.CI ? 1 : 0,

  testDir: './e2e',
  // A clean checkout can need one shared debug Rust build before real nodes
  // start. Each owned build command has a ten-minute deadline, with cleanup
  // before the last canceled node's stop() returns.
  timeout: 900_000,
  use: {
    headless: process.env.HEADLESS !== 'false',
    screenshot: 'only-on-failure',
    // Allow HEADLESS=false to run in headed mode
    trace: 'on-first-retry',
    video: 'retain-on-failure',
  },
  // Each local worker can own a real Rust node; serialize them to avoid
  // concurrent builds and port/fixture ownership races.
  workers: process.env.CI || shouldLaunchNodes() ? 1 : 2,
});
