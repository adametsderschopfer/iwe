import { defineConfig } from '@playwright/test'

const basePath = `/${process.env.NUXT_APP_BASE_URL || '/'}/`.replace(/\/+/g, '/')
const externalURL = process.env.PLAYWRIGHT_BASE_URL
const baseURL = externalURL
  ? `${externalURL.replace(/\/+$/, '')}/`
  : `http://127.0.0.1:4182${basePath}`

export default defineConfig({
  testDir: './tests/browser',
  workers: 1,
  fullyParallel: false,
  timeout: 20_000,
  globalTimeout: 120_000,
  expect: { timeout: 5000 },
  reporter: 'line',
  use: {
    baseURL,
    browserName: 'chromium',
    channel: process.env.PLAYWRIGHT_CHANNEL,
    viewport: { width: 1280, height: 800 },
    reducedMotion: 'no-preference',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: externalURL
    ? undefined
    : {
        command: 'node tests/support/server.mjs',
        url: baseURL,
        env: { NUXT_APP_BASE_URL: basePath },
        reuseExistingServer: false,
        timeout: 10_000,
      },
})
