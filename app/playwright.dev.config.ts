import { defineConfig } from '@playwright/test';
import base from './playwright.config';
import { savedProjectState } from './e2e/savedProjectState';

// CAD glue resolution differs between Vite development and production builds.
export default defineConfig({
  ...base,
  testMatch: ['cad-loading.spec.ts', 'ergogen-library.spec.ts'],
  use: { ...base.use, baseURL: 'http://127.0.0.1:4329/', storageState: savedProjectState('http://127.0.0.1:4329/') },
  webServer: {
    command: 'node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 4329 --strictPort',
    url: 'http://127.0.0.1:4329/',
    reuseExistingServer: false,
  },
});
