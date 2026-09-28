import { expect, test } from '@playwright/test';
import { writeFile } from 'node:fs/promises';
import { cpus } from 'node:os';

type Result = { p50: number; p95: number; samples: number };
type Measurements = {
  worker: Result;
  painted: Result;
  parts: number;
  stages?: { wasm: Result; parse: Result; transportQueue: Result; react: Result; frame: Result };
};

type Provenance = {
  browser: string;
  node: string;
  cpu: string;
  viewport: { width: number; height: number; deviceScaleFactor: number };
};

declare global {
  interface Window {
    runWorkbenchBenchmark: (keys: 100 | 200 | 500, scope: 'single' | 'row', captureStages?: 'off' | 'on') => Promise<Measurements>;
    benchmarkDiagnostics: () => { heapBytes: number | null; transferredBytes: number; longestTaskMs: number; longTasksOver100Ms: number };
  }
}

test('records mounted workbench assembly latency', async ({ page }) => {
  test.setTimeout(240_000);
  const provenance: Provenance = {
    browser: page.context().browser()?.version() ?? 'unknown',
    node: process.version,
    cpu: cpus()[0]?.model ?? 'unknown',
    viewport: { width: 1280, height: 720, deviceScaleFactor: 1 },
  };
  console.info(`Performance provenance: ${JSON.stringify(provenance)}`);
  await page.goto('/bench-workbench.html');
  const coldLoad = await page.evaluate(() => {
    const navigation = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
    const resources = performance.getEntriesByType('resource') as PerformanceResourceTiming[];
    return {
      navigationMs: navigation.duration,
      transferredBytes: resources.reduce((sum, entry) => sum + entry.transferSize, 0),
    };
  });
  console.info(`Workbench cold load: ${JSON.stringify(coldLoad)}`);
  const report: Record<string, Measurements> & { provenance?: Provenance } = { provenance };

  for (const keys of [100, 200] as const) {
    for (const scope of ['single', 'row'] as const) {
      const result = await page.evaluate(([count, target]) => window.runWorkbenchBenchmark(count, target), [keys, scope] as const);

      console.info(`Workbench ${keys} ${scope}: ${JSON.stringify(result)}`);
      report[`${keys}-${scope}`] = result;
      expect(result.parts).toBe(keys * 3);
      expect(result.worker.samples).toBe(100);
      expect(result.painted.samples).toBe(100);
    }
  }

  if (process.env.BOARDSTUDIO_PERF_STAGES) {
    const stages = await page.evaluate(() => window.runWorkbenchBenchmark(200, 'row', 'on'));
    console.info(`Workbench stage diagnostics: ${JSON.stringify(stages.stages)}`);
    expect(stages.stages?.wasm.samples).toBe(100);
  }

  if (process.env.BOARDSTUDIO_PERF_STRESS) {
    const stress = await page.evaluate(() => window.runWorkbenchBenchmark(500, 'row'));

    console.info(`Workbench 500 row diagnostic: ${JSON.stringify(stress)}`);
    expect(stress.parts).toBe(1500);
  }

  console.info(`Workbench diagnostics: ${JSON.stringify(await page.evaluate(() => window.benchmarkDiagnostics()))}`);

  if (process.env.BOARDSTUDIO_PERF_RESULT) {
    await writeFile(process.env.BOARDSTUDIO_PERF_RESULT, JSON.stringify(report));
  }
});
