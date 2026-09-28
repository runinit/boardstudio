import { expect, test } from '@playwright/test';
import { writeFile } from 'node:fs/promises';
import { splitFixture } from './splitMechanicalFixture';
import { auditSceneBounds } from './scene-bounds';

test('retains bounded memory through full-workbench editing', async ({ page }) => {
  test.skip(!process.env.BOARDSTUDIO_UI_SOAK_RESULT, 'Explicit long-session validation');
  test.setTimeout(1_800_000);
  page.setDefaultTimeout(20_000);
  await auditSceneBounds(page);
  await page.addInitScript(() => {
    const scope = window as any;
    scope.__soakPending = new Set();
    scope.__soakRequestCount = 0;
    const observed = new WeakSet<Worker>(), ids = new WeakMap<Worker, Set<string>>();
    const post = Worker.prototype.postMessage, terminate = Worker.prototype.terminate;
    Worker.prototype.postMessage = function(message: any, ...rest: any[]) {
      if (!observed.has(this)) {
        observed.add(this); ids.set(this, new Set());
        this.addEventListener('message', event => {
          if (event.data.kind === 'mechanical-resolved') scope.__soakAssembly = event.data.assembly;
          if (event.data.kind !== 'progress' && event.data.id) {
            scope.__soakPending.delete(event.data.id); ids.get(this)?.delete(event.data.id);
          }
        });
      }
      if (message?.kind === 'preview') { scope.__soakRequestCount++; scope.__soakPending.add(message.id); ids.get(this)!.add(message.id); }
      return post.call(this, message, ...rest);
    };
    Worker.prototype.terminate = function() {
      for (const id of ids.get(this) ?? []) scope.__soakPending.delete(id);
      return terminate.call(this);
    };
  });
  await page.goto('/?cadMetrics=1');
  await expect(page.locator('.wb-root')).toBeVisible();
  const numeric = splitFixture(); numeric.id = 'soak-numeric'; numeric.name = 'Soak numeric';
  const gasket = splitFixture(); gasket.id = 'soak-gasket'; gasket.name = 'Soak gasket';
  gasket.mechanical = { ...gasket.mechanical!, mount: 'gasket', integratedPlateFrame: false, gasketTravel: 0.3, bottomStyle: 'shell' };
  for (const document of [numeric, gasket]) document.boards.push({ ...document.boards[0], id: 'spare', name: 'Spare board', partIds: [], outlineIds: [], netIds: [] });
  await page.evaluate(async documents => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('boardstudio-v2', 1);
      request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
    });
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction('projects', 'readwrite');
      for (const document of documents) transaction.objectStore('projects').put(document);
      transaction.oncomplete = () => resolve(); transaction.onerror = () => reject(transaction.error);
    });
    db.close(); localStorage.setItem('boardstudio-v2-active-project', documents[0].id);
  }, [numeric, gasket]);
  await page.reload();
  const ready = async () => {
    await expect(page.locator('.wb-case-generation')).toHaveAttribute('data-state', 'ready', { timeout: 60_000 });
    await expect.poll(() => page.evaluate(() => (window as any).__soakPending.size)).toBe(0);
  };
  await page.getByRole('tab', { name: 'Case', exact: true }).click();
  await ready();
  const profiler = await page.context().newCDPSession(page);
  await profiler.send('HeapProfiler.enable');
  const rows: { cycle: number; project: string; jsHeapBytes: number; wasmBytes: number | null; ms: number }[] = [];
  const started = Date.now();
  let current = 'numeric', exports = 0, cancellations = 0, boardSwitches = 0, projectSwitches = 0;
  const moveGasket = async (mode: 'commit' | 'cancel', delta: number) => {
    const edit = page.getByRole('button', { name: 'Edit gaskets', exact: true });
    if (await edit.getAttribute('aria-pressed') !== 'true') await edit.click();
    await page.getByRole('button', { name: 'Fit', exact: true }).click();
    await page.getByRole('button', { name: 'Top', exact: true }).click();
    const box = (await page.locator('.wb-assembly-scene canvas').boundingBox())!;
    const { assembly, bounds } = await page.evaluate(() => ({ assembly: (window as any).__soakAssembly, bounds: (window as any).__fitBounds }));
    const support = assembly.gasketSupports.find((item: any) => item.regionId === 'left');
    const retainer = assembly.stack.find((layer: any) => layer.id === 'retainer');
    const vertical = 17 * Math.PI / 180, horizontal = Math.atan(Math.tan(vertical) * box.width / box.height);
    const distance = bounds[3] / Math.sin(Math.min(vertical, horizontal)) * 1.16;
    const screen = (x: number, y: number) => {
      const dx = x - bounds[0], dy = y - bounds[1], dz = retainer.z + retainer.thickness + 1 - bounds[2];
      const scale = box.height / (2 * (distance - dx * Math.cos(1.56) - dz * Math.sin(1.56)) * Math.tan(vertical));
      return { x: box.x + box.width / 2 + (dx * Math.sin(1.56) - dz * Math.cos(1.56)) * scale, y: box.y + box.height / 2 - dy * scale };
    };
    const from = screen(support.at.x, support.at.y);
    const to = screen(support.at.x + support.tangent.x * delta, support.at.y + support.tangent.y * delta);
    const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
    const requests = await page.evaluate(() => (window as any).__soakRequestCount);
    await page.mouse.move(from.x, from.y); await page.mouse.down(); await page.mouse.move(to.x, to.y, { steps: 6 });
    await expect.poll(() => page.evaluate(() => (window as any).__soakRequestCount)).toBeGreaterThan(requests);
    if (mode === 'cancel') { await page.keyboard.press('Escape'); cancellations++; }
    await page.mouse.up();
    await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', String(revision + (mode === 'commit' ? 1 : 0)));
    await ready();
  };
  for (let cycle = 1; cycle <= 256; cycle++) {
    if (current === 'numeric') {
      const input = page.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true });
      const before = await input.inputValue();
      const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
      await input.fill(String(2.1 + (cycle % 16) * 0.05)); await input.press('Tab');
      await expect.poll(async () => Number(await page.locator('.wb-root').getAttribute('data-revision'))).toBeGreaterThan(revision);
      await ready();
      await page.getByRole('button', { name: 'Undo', exact: true }).click();
      await expect(input).toHaveValue(before); await ready();
    } else {
      await moveGasket('commit', 1 + (cycle % 8) * 0.1);
      await page.getByRole('button', { name: 'Undo', exact: true }).click(); await ready();
      if (cycle % 16 === 0) await moveGasket('cancel', 2);
    }
    if (cycle % 32 === 0) {
      console.info(`UI soak ${cycle}: board switch`);
      await page.getByRole('combobox', { name: 'Selected board', exact: true }).selectOption('spare');
      await page.getByRole('button', { name: 'Show configured board', exact: true }).click();
      await ready(); boardSwitches++;
      console.info(`UI soak ${cycle}: export`);
      await page.getByRole('button', { name: 'Export', exact: true }).first().click();
      const download = page.waitForEvent('download', { timeout: 120_000 });
      await page.locator('.wb-export-row').filter({ hasText: 'Generated mechanical package' }).getByRole('button', { name: 'Export', exact: true }).click();
      const file = await download; expect(await file.failure()).toBeNull(); await file.delete(); exports++;
      await page.getByRole('tab', { name: 'Case', exact: true }).click(); await ready();
      current = current === 'numeric' ? 'gasket' : 'numeric';
      console.info(`UI soak ${cycle}: project switch`);
      await page.getByRole('button', { name: 'Project', exact: true }).click();
      await page.getByRole('button', { name: `Open Soak ${current}`, exact: true }).click();
      await page.getByRole('tab', { name: 'Case', exact: true }).click(); await ready(); projectSwitches++;
    }
    if (cycle % 8 === 0) {
      await profiler.send('HeapProfiler.collectGarbage');
      const heap = await profiler.send('Runtime.getHeapUsage');
      const wasmBytes = await page.evaluate(() => {
        const requests = performance.getEntriesByName('boardstudio.cad.request') as PerformanceMeasure[];
        return [...requests].reverse().find(entry => entry.detail?.wasmAllocatedBytes?.end)?.detail.wasmAllocatedBytes.end ?? null;
      });
      rows.push({ cycle, project: current, jsHeapBytes: heap.usedSize, wasmBytes, ms: Date.now() - started });
      console.info(`UI soak ${cycle}/256`);
    }
  }
  const median = (values: number[]) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];
  const trends = Object.fromEntries(['numeric', 'gasket'].map(project => {
    const previous = rows.filter(row => row.project === project && row.cycle > 128 && row.cycle <= 192);
    const latest = rows.filter(row => row.project === project && row.cycle > 192);
    return [project, Object.fromEntries(['jsHeapBytes', 'wasmBytes'].map(key => {
      const earlier = previous.map(row => row[key as 'jsHeapBytes' | 'wasmBytes']).filter((value): value is number => value !== null);
      const later = latest.map(row => row[key as 'jsHeapBytes' | 'wasmBytes']).filter((value): value is number => value !== null);
      const before = median(earlier), after = median(later), allowance = Math.max(2 * 1024 * 1024, before * (key === 'jsHeapBytes' ? 0.1 : 0.05));
      return [key, { before, after, allowance, passed: earlier.length > 0 && later.length > 0 && after <= before + allowance }];
    }))];
  }));
  await writeFile(process.env.BOARDSTUDIO_UI_SOAK_RESULT!, JSON.stringify({ cycles: 256, elapsedMs: Date.now() - started, exports, cancellations, boardSwitches, projectSwitches, rows, trends, processMemory: 'not sampled', physicalDevice: false }, null, 2));
  for (const project of Object.values(trends)) for (const trend of Object.values(project)) expect(trend.passed).toBe(true);
  await profiler.detach();
});
