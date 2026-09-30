import { defineConfig } from '@playwright/test';
import base from './playwright.config';

const baseURL = `${base.use!.baseURL}boardstudio/`;
const port = new URL(baseURL).port;

export default defineConfig({
  ...base,
  testDir: './e2e-pages',
  use: { ...base.use, baseURL },
  webServer: {
    command: `node node_modules/vite/bin/vite.js preview --host 127.0.0.1 --port ${port} --strictPort --base /boardstudio/`,
    url: baseURL,
    reuseExistingServer: false,
  },
});
