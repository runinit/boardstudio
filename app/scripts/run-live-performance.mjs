import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { compareLive } from './compare-live-performance.mjs';

const app = fileURLToPath(new URL('../', import.meta.url));
const output = resolve(process.argv[2] ?? 'app/performance-results/live-latest');
const reference = process.argv[3] ? JSON.parse(await readFile(process.argv[3])) : undefined;
await mkdir(output, { recursive: true });
const reports = [];
for (let session = 1; session <= 5; session += 1) {
  const path = join(output, `session-${session}.json`);
  const run = spawnSync('pnpm', ['exec', 'playwright', 'test', '--config', 'playwright.performance.config.ts',
    'e2e/live-preview-performance.spec.ts', '--workers=1', '--headed'], {
    cwd: app, stdio: 'inherit', env: { ...process.env, BOARDSTUDIO_CHROMIUM: process.env.BOARDSTUDIO_CHROMIUM ?? '/usr/bin/chromium',
      BOARDSTUDIO_LIVE_PERF_RESULT: path },
  });
  if (run.error || run.status !== 0) throw run.error ?? new Error(`Live session ${session} failed`);
  reports.push(JSON.parse(await readFile(path)));
}
const percentile = (values, fraction) => [...values].sort((a, b) => a - b)[Math.max(0, Math.ceil(values.length * fraction) - 1)];
const summary = { sessions: reports.length, provenance: reports.map(report => report.provenance),
  scenarios: Object.fromEntries(['numeric', 'undo', 'gaskets', 'mounts'].map(name => {
    const values = reports.flatMap(report => report.scenarios[name].releaseToPaint.samples);
    return [name, { samples: values.length, p50: percentile(values, 0.5), p95: percentile(values, 0.95),
      sessionP95: reports.map(report => report.scenarios[name].releaseToPaint.p95) }];
  })) };
await writeFile(join(output, 'summary.json'), JSON.stringify(summary, null, 2));
if (reference) {
  const comparison = compareLive(summary, reference);
  await writeFile(join(output, 'comparison.json'), JSON.stringify(comparison, null, 2));
  console.info(JSON.stringify(comparison, null, 2));
  if (!comparison.regressionsPassed || !comparison.improvementPassed) process.exitCode = 1;
}
