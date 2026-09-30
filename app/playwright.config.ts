import { defineConfig, devices } from '@playwright/test';

const PORT = Number(process.env.BOARDSTUDIO_TEST_PORT ?? 4328);

export default defineConfig({
  testDir: './e2e',
  testIgnore: '**/*performance.spec.ts',
  retries: 0,
  // Keep CAD-heavy functional tests within a predictable memory budget.
  workers: 1,
  use: {
    screenshot: 'only-on-failure',
    baseURL: `http://127.0.0.1:${PORT}/`,
    ...devices['Desktop Chrome'],
    viewport: { width: 1280, height: 720 },
    deviceScaleFactor: 1,
    launchOptions: {
      executablePath: process.env.BOARDSTUDIO_CHROMIUM || undefined,
    },
  },
  webServer: {
    // pnpm 12 detaches the child process group, preventing Playwright teardown.
    command: `node node_modules/vite/bin/vite.js preview --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: `http://127.0.0.1:${PORT}/`,
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
