import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  base: './',
  plugins: [react()],
  // Assets are same-origin. Vary: Origin prevents precached assets matching offline module requests.
  preview: { cors: false },
  build: {
    target: 'es2022',
    rolldownOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        bench: fileURLToPath(new URL('./bench.html', import.meta.url)),
        workbenchBench: fileURLToPath(new URL('./bench-workbench.html', import.meta.url)),
      },
    },
  },
  worker: { format: 'es' },
});
