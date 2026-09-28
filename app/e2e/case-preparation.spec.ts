import { navigateWorkspace } from './workspace-navigation';
import { expect, test } from '@playwright/test';

test('lazily loads CAD and prepares a current case preview and STEP export offline', async ({ page, context }) => {
  test.setTimeout(60_000);
  const wasmRequests = new Set<string>();
  page.on('response', (response) => {
    if (/\.wasm(?:$|\?)/i.test(response.url())) wasmRequests.add(response.url());
  });
  await page.addInitScript(() => {
    const requests = (window as typeof window & { __casePreparationRequests?: unknown[] }).__casePreparationRequests = [];
    const postMessage = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message: unknown, ...transfer: Transferable[]) {
      if (message && typeof message === 'object' && 'kind' in message && message.kind === 'prepare-case') {
        requests.push(message);
        (window as typeof window & { __casePreparationWorker?: Worker; __casePreparationIR?: unknown }).__casePreparationWorker = this;
        (window as typeof window & { __casePreparationIR?: unknown }).__casePreparationIR = message.ir;
      }
      return postMessage.call(this, message, ...transfer);
    };
  });

  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await expect.poll(() => wasmRequests.size).toBeGreaterThan(0);
  expect([...wasmRequests].some((url) => /boardstudio_renderer_wasm_bg/i.test(url))).toBe(false);
  const wasmBeforeCase = wasmRequests.size;
  await navigateWorkspace(page, 'Case');
  const live = page.getByRole('switch', { name: /Live preview/i });
  if (await live.isChecked()) await live.uncheck();
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByText('Preview current', { exact: true })).toBeVisible({ timeout: 45_000 });
  await expect.poll(() => wasmRequests.size).toBeGreaterThan(wasmBeforeCase);
  await expect(page.locator('.wb-assembly-scene canvas')).toBeVisible();
  await expect(page.getByRole('group', { name: 'Assembly camera' }).getByRole('button', { name: 'Fit', exact: true })).toBeEnabled();
  const rendererTiming = await page.evaluate(() => {
    const wasm = performance.getEntriesByName('boardstudio.renderer.wasm.cold-start').at(-1);
    const canvas = performance.getEntriesByName('boardstudio.renderer.canvas.cold-start').at(-1);
    const resource = performance.getEntriesByType('resource').find((entry) => /boardstudio_renderer_wasm_bg/i.test(entry.name));
    return { wasmInitMs: wasm?.duration ?? NaN, canvasSceneMs: canvas?.duration ?? NaN, transferMs: resource?.duration ?? NaN, bytes: resource?.encodedBodySize ?? NaN };
  });
  expect(rendererTiming.canvasSceneMs).toBeGreaterThan(0);
  console.info(`Renderer WASM cold start: ${JSON.stringify(rendererTiming)}`);
  await page.getByRole('combobox', { name: 'Body type' }).selectOption('tray');
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '1');
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByText('Preview current', { exact: true })).toBeVisible({ timeout: 45_000 });

  const requests = await page.evaluate(() => (window as typeof window & { __casePreparationRequests: unknown[] }).__casePreparationRequests);
  expect(requests.length).toBeGreaterThanOrEqual(2);

  const diagnostic = await page.evaluate(async () => {
    const scope = window as typeof window & { __casePreparationWorker?: Worker; __casePreparationIR?: { revision: number; bodies: { contours: { points: unknown[] }[] }[] } };
    const worker = scope.__casePreparationWorker;
    const ir = scope.__casePreparationIR;
    if (!worker || !ir) throw new Error('Case preparation worker was not observed');

    const sample = () => new Promise<{ totalMs: number; wasmMs: number }>((resolve, reject) => {
      const id = crypto.randomUUID();
      const started = performance.now();
      const onMessage = (event: MessageEvent<{ id: string; kind: string; message?: string; timing?: { wasmMs: number } }>) => {
        if (event.data.id !== id) return;
        worker.removeEventListener('message', onMessage as EventListener);
        if (event.data.kind !== 'case-prepared') {
          reject(new Error(event.data.message ?? `Unexpected preparation reply: ${event.data.kind}`));
          return;
        }
        resolve({ totalMs: performance.now() - started, wasmMs: event.data.timing?.wasmMs ?? NaN });
      };
      worker.addEventListener('message', onMessage as EventListener);
      worker.postMessage({ id, kind: 'prepare-case', ir, diagnostics: true });
    });
    const percentile = (values: number[], fraction: number) => {
      const sorted = [...values].sort((a, b) => a - b);
      return sorted[Math.ceil((sorted.length - 1) * fraction)];
    };
    const warmupSamples = 2;
    const measuredSamples = 8;
    for (let index = 0; index < warmupSamples; index += 1) await sample();
    const measured = [];
    for (let index = 0; index < measuredSamples; index += 1) measured.push(await sample());
    const total = measured.map((item) => item.totalMs);
    const wasm = measured.map((item) => item.wasmMs);
    return {
      bodies: ir.bodies.length,
      vertices: ir.bodies.reduce((count, body) => count + body.contours.reduce((bodyCount, contour) => bodyCount + contour.points.length, 0), 0),
      warmupSamples,
      measuredSamples,
      totalMs: { p50: percentile(total, 0.5), p95: percentile(total, 0.95) },
      wasmMs: { p50: percentile(wasm, 0.5), p95: percentile(wasm, 0.95) },
    };
  });
  expect(diagnostic.bodies).toBeGreaterThan(0);
  expect(diagnostic.vertices).toBeGreaterThan(2);
  expect(diagnostic.measuredSamples).toBe(8);
  console.info(`Case preparation diagnostic: ${JSON.stringify(diagnostic)}`);

  await page.evaluate(() => navigator.serviceWorker.ready);
  await expect.poll(() => page.evaluate(() => Boolean(navigator.serviceWorker.controller))).toBe(true);
  await context.setOffline(true);
  await page.reload();
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Case');
  const offlineLive = page.getByRole('switch', { name: /Live preview/i });
  if (await offlineLive.isChecked()) await offlineLive.uncheck();
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByText('Preview current', { exact: true })).toBeVisible({ timeout: 45_000 });

  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const download = page.waitForEvent('download');
  await page.locator('.wb-export-row').filter({ hasText: 'Case STEP' }).getByRole('button', { name: 'Export' }).click();
  expect((await download).suggestedFilename()).toBe('Starter keyboard-case.step');
});
