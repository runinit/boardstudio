import { expect, test, type Page } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { unzipSync, strFromU8 } from 'fflate';
import { catalogue, normalizeDefinition } from '@boardstudio/v2-ergogen';
import { demoProject } from '../src/demo';
import { navigateWorkspace } from './workspace-navigation';

async function openFixture(page: Page) {
  const doc = demoProject(); doc.id = 'keycaps-browser'; doc.name = 'Keycap keyboard';
  const source = catalogue().find(definition => definition.generator?.source === 'ceoloide/mcu_nice_nano')!;
  const mcu = normalizeDefinition({ ...source, generator: { ...source.generator!, parameters: { reversible: true } } });
  doc.definitions.push(mcu);
  doc.parts.push({ id: 'controller', reference: 'U1', definitionId: mcu.id, pose: { at: { x: 110, y: 19 }, rotation: 0 }, side: 'front' });
  doc.boards[0].partIds.push('controller'); doc.nets = []; doc.boards[0].netIds = [];
  doc.hardware = { topology: 'unibody', transport: 'none', instances: [], sharedConstruction: null, boards: [{ boardId: 'main-board', controllerPartId: 'controller', mode: 'direct', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null }] };
  await page.goto('/');
  await expect(page.locator('.wb-root')).toBeVisible();
  await page.evaluate(async document => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => { const request = indexedDB.open('boardstudio-v2', 1); request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error); });
    await new Promise<void>((resolve, reject) => { const transaction = db.transaction('projects', 'readwrite'); transaction.objectStore('projects').put(document); transaction.oncomplete = () => resolve(); transaction.onerror = () => reject(transaction.error); });
    db.close(); localStorage.setItem('boardstudio-v2-active-project', document.id);
  }, doc);
  await page.reload(); await navigateWorkspace(page, 'Keymap');
}

test('keymap profiles, legends, colors, Undo, persistence, CAD preview and exports', async ({ page }) => {
  test.setTimeout(120_000);
  const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    const state = window as any; state.__capSpecs = []; state.__capMeshes = [];
    const OriginalWorker = window.Worker;
    window.Worker = class extends OriginalWorker {
      constructor(url: string | URL, options?: WorkerOptions) { super(url, options); this.addEventListener('message', event => {
        if (event.data.kind === 'case' && event.data.result?.bodies?.some((body: any) => body.id.startsWith('keycap:'))) state.__capMeshes.push(event.data.result.bodies.map((body: any) => ({ id: body.id, count: body.positions.length })));
      }); }
      postMessage(message: any, options?: any) { if (message.kind === 'keycaps') state.__capSpecs = message.specs; super.postMessage(message, options); }
    } as typeof Worker;
  });
  await openFixture(page);
  const panel = page.locator('.wb-keymap-panel');
  await expect(panel.getByRole('heading', { name: 'Keymap & keycaps' })).toBeVisible();
  await panel.getByRole('combobox', { name: 'Keycap profile for matrix' }).selectOption('dsa');
  await page.getByRole('button', { name: 'Edit key SW1', exact: true }).click();
  await panel.getByRole('combobox', { name: 'Binding for SW1', exact: true }).selectOption('&kp A');
  await panel.getByLabel('Keycap color for SW1', { exact: true }).fill('#336699');
  await expect(page.locator('.wb-keymap-layout .is-selected rect')).toHaveAttribute('fill', '#336699');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.locator('.wb-keymap-layout .is-selected rect')).toHaveAttribute('fill', '#e8e4dc');
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(page.locator('.wb-keymap-layout .is-selected rect')).toHaveAttribute('fill', '#336699');
  await panel.getByLabel('Legend for SW1', { exact: true }).fill('Q');
  await panel.getByLabel('Legend for SW1', { exact: true }).press('Tab');
  await expect(page.locator('.wb-keymap-layout .is-selected text').first()).toHaveText('Q');
  await page.reload(); await navigateWorkspace(page, 'Keymap');
  await page.getByRole('button', { name: 'Edit key SW1', exact: true }).click();
  await expect(panel.getByRole('combobox', { name: 'Binding for SW1', exact: true })).toHaveValue('&kp A');
  await expect(panel.getByLabel('Legend for SW1', { exact: true })).toHaveValue('Q');
  await panel.getByRole('button', { name: 'Use binding legend', exact: true }).click();
  await expect(page.locator('.wb-keymap-layout .is-selected text').first()).toHaveText('A');
  await page.screenshot({ path: '/tmp/boardstudio-keycaps-desktop.png', fullPage: true });
  await page.getByRole('button', { name: '3D assembly', exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__capMeshes.at(-1)?.filter((body: any) => body.id.startsWith('keycap:')).length), { timeout: 60_000 }).toBe(15);
  await expect.poll(() => page.evaluate(() => (window as any).__capSpecs.find((spec: any) => spec.reference === 'SW1')?.legend)).toBe('A');
  await expect(page.getByRole('alert')).toHaveCount(0);
  await page.screenshot({ path: '/tmp/boardstudio-keycaps-assembly.png', fullPage: true });
  const stepDownload = page.waitForEvent('download');
  await panel.getByRole('button', { name: 'Export keycap STEP', exact: true }).click();
  const step = await stepDownload; expect(step.suggestedFilename()).toMatch(/keycaps\.step$/);
  const stepBytes = await readFile((await step.path())!); expect(stepBytes.toString()).toContain('ISO-10303-21');
  const zmkDownload = page.waitForEvent('download');
  await panel.getByRole('button', { name: 'Export ZMK source', exact: true }).click();
  const zmk = await zmkDownload; const files = unzipSync(await readFile((await zmk.path())!));
  expect(strFromU8(files['build-local.sh'])).toContain('west init -l config');
  expect(strFromU8(files['config/boards/shields/boardstudio/boardstudio.keymap'])).toContain('&kp A');
  expect(strFromU8(files['config/west.yml'])).toContain('v0.3.0');
  expect(errors).toEqual([]);
});

test('keymap controls remain usable on a narrow screen', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openFixture(page);
  await expect(page.locator('.wb-keymap-panel').getByRole('heading', { name: 'Keymap & keycaps' })).toBeVisible();
  await page.getByRole('combobox', { name: 'Keycap profile for matrix' }).selectOption('cherry');
  await expect(page.getByRole('combobox', { name: 'Keycap profile for matrix' })).toHaveValue('cherry');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await expect.poll(async () => {
    const controls = await page.locator('.wb-keymap-panel').locator('input:visible, select:visible').evaluateAll(elements => elements.map(element => ({ name: element.getAttribute('aria-label'), right: element.getBoundingClientRect().right, width: element.getBoundingClientRect().width })));
    return controls.filter(control => control.right > 390 || control.width < 40);
  }).toEqual([]);
  await page.screenshot({ path: '/tmp/boardstudio-keycaps-mobile.png', fullPage: true });
});
