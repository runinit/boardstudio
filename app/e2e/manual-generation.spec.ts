import { auditSceneBounds } from './scene-bounds';
import { navigateWorkspace } from './workspace-navigation';
import { expect, test, type Page } from '@playwright/test';
import { splitFixture } from './splitMechanicalFixture';
import type { MechanicalAssembly } from '@boardstudio/v2-contracts';


async function setup(page: Page, legacyGasket = false) {
  page.setDefaultTimeout(15_000);
  await auditSceneBounds(page);
  await page.addInitScript(() => {
    const audit = (window as any).__generationAudit = { requests: [], progress: [], assemblies: [], prepared: [], jobs: [], bounds: [], cadDelay: 0 };
    const original = Worker.prototype.postMessage;
    const observed = new WeakSet<Worker>();
    Worker.prototype.postMessage = function (message: any, ...rest: any[]) {
      if (!observed.has(this)) {
        observed.add(this);
        this.addEventListener('message', event => {
          if (!event.data.patch && event.data.prepared?.bounds) audit.bounds.push(event.data.prepared.bounds);
          if (event.data.kind === 'progress') audit.progress.push(event.data.progress);
          if (event.data.kind === 'mechanical-resolved') audit.assemblies.push(event.data.assembly);
          if (event.data.kind === 'case-prepared') audit.prepared.push(event.data.ir);
          if (event.data.kind === 'preview') audit.jobs.push({ revision: event.data.result.revision, hasStep: 'step' in event.data.result });
        });
      }
      if (message.kind) audit.requests.push({ kind: message.kind, revision: message.ir?.revision });
      if (message.kind === 'preview' && audit.cadDelay) {
        setTimeout(() => { try { original.call(this, message, ...rest); } catch {} }, audit.cadDelay);
        return;
      }
      return original.call(this, message, ...rest);
    };
  });
  await page.goto('/?cadMetrics=1');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await page.evaluate(async document => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('boardstudio-v2', 1);
      request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
    });
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction('projects', 'readwrite');
      transaction.objectStore('projects').put(document);
      transaction.oncomplete = () => resolve(); transaction.onerror = () => reject(transaction.error);
    });
    db.close(); localStorage.setItem('boardstudio-v2-active-project', document.id);
  }, splitFixture(legacyGasket));
  await page.reload();
  await page.getByRole('button', { name: /^Collapse left half$/i }).click({ timeout: 10000 });
  await page.getByRole('button', { name: /^Collapse right half$/i }).click({ timeout: 10000 });
  await page.getByRole('tab', { name: 'Case', exact: true }).click({ timeout: 15000 });
  await expect.poll(() => page.evaluate(() => (window as any).__generationAudit.assemblies.length)).toBeGreaterThan(0);
  // These tests exercise the explicit CAD action and cancellation path.  Live
  // preview is the product default, so pause it before asserting no request or
  // invoking Update preview directly.
  const live = page.getByRole('switch', { name: /Live preview/i });
  await expect(live).toBeChecked();
  await live.uncheck();
  await expect(live).not.toBeChecked();
  await page.evaluate(() => { const audit = (window as any).__generationAudit; audit.requests = []; audit.jobs = []; });
  return page.locator('.wb-mechanical-panel');
}

test('split CAD is manual, cancellable, cached and independent of display changes', async ({ page }, info) => {
  test.setTimeout(180_000);
  const panel = await setup(page);
  expect(await page.evaluate(() => (window as any).__generationAudit.requests.filter((request: any) => request.kind === 'preview').length)).toBe(0);
  await page.evaluate(() => { (window as any).__generationAudit.cadDelay = 1500; });
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Case generation' }).getByText(/Preview paused/i)).toBeVisible();
  await page.evaluate(() => { (window as any).__generationAudit.cadDelay = 0; });
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByText(/Generated CAD solids · 4 parts/)).toBeVisible({ timeout: 90_000 });
  await expect(page.getByText('Preparing 3D geometry…', { exact: true })).toHaveCount(0);
  const audit = await page.evaluate(() => (window as any).__generationAudit);
  expect(audit.progress.some((progress: any) => progress.stage === 'building')).toBe(true);
  expect(audit.progress.some((progress: any) => progress.stage === 'tessellating')).toBe(true);
  expect(audit.jobs.every((job: any) => !job.hasStep)).toBe(true);
  const plate = audit.prepared.at(-1).bodies.find((body: any) => body.body.id === 'plate');
  expect(plate.regions).toHaveLength(2);
  expect(plate.regions.reduce((count: number, region: any) => count + region.holes.length, 0)).toBe(70);
  const measures = () => page.evaluate(() => performance.getEntriesByType('measure').filter(entry => /renderer\.(prepare|upload)$/.test(entry.name)).map(entry => ({ name: entry.name, duration: entry.duration })));
  const before = await measures();
  const canvas = page.locator('.wb-assembly-scene canvas');
  const images: Buffer[] = [];
  for (const mode of ['Shaded', 'Wireframe', 'Hybrid']) {
    await page.getByRole('button', { name: mode, exact: true }).click();
    await expect(page.getByRole('button', { name: mode, exact: true })).toHaveAttribute('aria-pressed', 'true');
    images.push(await canvas.screenshot({ path: info.outputPath(`split-${mode.toLowerCase()}.png`) }));
  }
  expect(images[0].equals(images[1])).toBe(false);
  expect(images[1].equals(images[2])).toBe(false);
  for (const view of ['Top', 'Bottom', 'Isometric', 'Exploded', 'Section', 'Assembled']) await page.getByRole('button', { name: view, exact: true }).click();
  await page.getByRole('region', { name: 'Canvas layers' }).getByRole('button', { name: 'Layers', exact: true }).click();
  await page.getByRole('button', { name: /^(Hide|Show) Copper$/, exact: true }).click();
  await page.getByRole('button', { name: /^(Hide|Show) Copper$/, exact: true }).click();
  expect(await measures()).toEqual(before);
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Case generation' }).getByText(/Geometry current/)).toBeVisible();
  const timings = await page.evaluate(() => performance.getEntriesByType('measure').filter(entry => /boardstudio\.(cad|renderer)/.test(entry.name)).map(entry => ({ name: entry.name, ms: Math.round(entry.duration * 100) / 100 })));
  console.info('Split generation timings', JSON.stringify(timings));
  await info.attach('split-timings', { body: JSON.stringify(timings, null, 2), contentType: 'application/json' });
  await panel.locator('summary').filter({ hasText: 'Dimensions & clearances' }).click();
  await panel.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true }).fill('2.5');
  await panel.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true }).press('Tab');
  await expect(page.getByRole('region', { name: 'Case generation' }).getByText(/Preview out of date|Preview paused/i)).toBeVisible();
  await expect(page.getByText(/previous geometry/)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(panel.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true })).toHaveValue('2');
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(panel.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true })).toHaveValue('2.5');
});

test('gaskets create six linked support pairs, a retainer and editable preview handles', async ({ page }, info) => {
  test.setTimeout(180_000);
  const panel = await setup(page, true);
  await expect(panel.getByRole('combobox', { name: 'Mount style', exact: true })).toHaveValue('gasket');
  await expect.poll(async () => page.evaluate(() => (window as any).__generationAudit.assemblies.at(-1)?.gasketSupports.length)).toBe(12);
  const assembly: MechanicalAssembly = await page.evaluate(() => (window as any).__generationAudit.assemblies.at(-1));
  expect(assembly.generationBlocked, JSON.stringify(assembly.diagnostics)).toBe(false);
  expect(assembly.generatedHardware).toHaveLength(24);
  expect(assembly.case.bodies.filter(body => body.body.id.startsWith('gasket:'))).toHaveLength(24);
  for (const support of assembly.gasketSupports.filter(support => support.regionId === 'left')) {
    const pair = assembly.gasketSupports.find(item => item.id === support.pairId)!;
    expect(pair.at.x).toBeCloseTo(305 - support.at.x, 4);
    expect(pair.at.y).toBeCloseTo(support.at.y, 4);
  }
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByText(/Generated CAD solids · 29 parts/)).toBeVisible({ timeout: 120_000 });
  await page.getByRole('button', { name: 'Edit gaskets', exact: true }).click();
  await page.locator('.wb-assembly-scene canvas').screenshot({ path: info.outputPath('gasket-handles.png') });
  await expect(page.getByText(/Drag.*gasket|perimeter/i).first()).toBeVisible();
  await page.getByRole('button', { name: 'Fit', exact: true }).click();
  await page.getByRole('button', { name: 'Top', exact: true }).click();
  const canvas = page.locator('.wb-assembly-scene canvas');
  const box = (await canvas.boundingBox())!;
  const bounds: number[] = await page.evaluate(() => (window as any).__fitBounds);
  const retainer = assembly.stack.find(layer => layer.id === 'retainer')!;
  const support = assembly.gasketSupports.find(item => item.regionId === 'left')!;
  const vertical = 17 * Math.PI / 180;
  const horizontal = Math.atan(Math.tan(vertical) * box.width / box.height);
  const distance = bounds[3] / Math.sin(Math.min(vertical, horizontal)) * 1.16;
  const screen = (x: number, y: number) => {
    const dx = x - bounds[0], dy = y - bounds[1], dz = retainer.z + retainer.thickness + 1 - bounds[2];
    const depth = distance - dx * Math.cos(1.56) - dz * Math.sin(1.56);
    const scale = box.height / (2 * depth * Math.tan(vertical));
    return { x: box.x + box.width / 2 + (dx * Math.sin(1.56) - dz * Math.cos(1.56)) * scale, y: box.y + box.height / 2 - dy * scale };
  };
  const start = screen(support.at.x, support.at.y);
  const end = screen(support.at.x + support.tangent.x * 2, support.at.y + support.tangent.y * 2);
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(end.x, end.y, { steps: 6 });
  await page.mouse.up();
  await expect(page.getByRole('region', { name: 'Case generation' }).getByText(/Preview out of date|Preview paused/i)).toBeVisible();
  await expect.poll(async () => page.evaluate(() => (window as any).__generationAudit.assemblies.at(-1).gasketSupports[0].anchor)).not.toBe(support.anchor);
  const moved: MechanicalAssembly = await page.evaluate(() => (window as any).__generationAudit.assemblies.at(-1));
  expect(moved.generationBlocked).toBe(false);
  const movedSource = moved.gasketSupports.find(item => item.id === support.id)!;
  const movedPair = moved.gasketSupports.find(item => item.id === movedSource.pairId)!;
  expect(movedPair.at.x).toBeCloseTo(305 - movedSource.at.x, 4);
  expect(movedPair.at.y).toBeCloseTo(movedSource.at.y, 4);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__generationAudit.assemblies.at(-1).gasketSupports[0].anchor)).toBe(support.anchor);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__generationAudit.assemblies.at(-1).gasketSupports[0].anchor)).toBe(movedSource.anchor);
  await page.reload();
  await page.getByRole('button', { name: /^Collapse left half$/i }).click();
  await page.getByRole('button', { name: /^Collapse right half$/i }).click();
  await navigateWorkspace(page, 'Case');
  await expect.poll(() => page.evaluate(() => (window as any).__generationAudit.assemblies.at(-1)?.gasketSupports[0].anchor)).toBe(movedSource.anchor);
  await expect(page.getByRole('switch', { name: 'Live preview', exact: true })).toBeChecked();
  await expect(page.locator('.wb-case-generation')).toHaveAttribute('data-state', 'ready', { timeout: 60_000 });

});
