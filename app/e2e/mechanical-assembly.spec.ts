import { navigateWorkspace } from './workspace-navigation';
import { expect, test, type Page } from '@playwright/test';
import type { ProjectDoc } from '@boardstudio/v2-contracts';

async function saved(page: Page): Promise<ProjectDoc> {
  return page.evaluate(async () => {
    const projectId = localStorage.getItem('boardstudio-v2-active-project') ?? 'starter';
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('boardstudio-v2', 1);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    return new Promise<ProjectDoc>((resolve, reject) => {
      const request = db.transaction('projects').objectStore('projects').get(projectId);
      request.onsuccess = () => { db.close(); resolve(request.result); };
      request.onerror = () => { db.close(); reject(request.error); };
    });
  });
}

async function configure(page: Page) {
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await expect(page.getByText('Configuration resolved', { exact: false })).toBeVisible();
  return page.locator('.wb-mechanical-panel');
}

test('mechanical configuration is opt-in, undoable and persistent without replacing authored bodies', async ({ page }) => {
  const panel = await configure(page);
  await expect.poll(async () => (await saved(page)).mechanical?.method).toBe('printed');
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByText(/Generated CAD solids · 4 parts at revision/)).toBeVisible({ timeout: 60_000 });
  const originalBodies = (await saved(page)).caseBodies;
  await panel.getByRole('combobox', { name: 'Bottom construction', exact: true }).selectOption('sheet');
  await expect.poll(async () => (await saved(page)).mechanical?.bottomStyle).toBe('sheet');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(panel.getByRole('combobox', { name: 'Bottom construction', exact: true })).toHaveValue('shell');
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(panel.getByRole('combobox', { name: 'Bottom construction', exact: true })).toHaveValue('sheet');
  await page.reload();
  await navigateWorkspace(page, 'Case');
  await expect(panel.getByRole('combobox', { name: 'Bottom construction', exact: true })).toHaveValue('sheet');
  expect((await saved(page)).caseBodies).toEqual(originalBodies);
  await panel.locator('summary').filter({ hasText: 'Configuration management' }).click();
  await panel.getByRole('button', { name: 'Disable mechanical stack', exact: true }).click();
  await expect.poll(async () => (await saved(page)).mechanical ?? undefined).toBeUndefined();
  expect((await saved(page)).caseBodies).toEqual(originalBodies);
});

test('library profile resolves real cutouts and view controls leave the committed configuration unchanged', async ({ page }) => {
  test.setTimeout(90000);
  const panel = await configure(page);
  await panel.locator('summary').filter({ hasText: 'Advanced source geometry' }).click();
  await panel.getByRole('combobox', { name: 'Library fit profile', exact: true }).selectOption('mx-switch');
  await panel.getByRole('combobox', { name: 'Assign library fit profile to', exact: true }).selectOption('ergogen:ceoloide/switch_mx');
  await expect.poll(async () => (await saved(page)).mechanical?.profiles.length, { timeout: 30_000 }).toBe(1);
  const profile = (await saved(page)).mechanical!.profiles[0];
  expect(profile.cutouts).toHaveLength(1);
  expect(profile.cutouts[0]).toHaveLength(4);
  expect(Math.max(...profile.cutouts[0].map(p => p.x)) - Math.min(...profile.cutouts[0].map(p => p.x))).toBe(14);
  expect(profile.source).toContain('14 x 14 mm');
  await panel.locator('summary').filter({ hasText: 'Per-part process overrides' }).click();
  const materials = panel.getByRole('combobox', { name: 'Material', exact: true });
  await materials.first().selectOption('PLA');
  await expect.poll(async () => (await saved(page)).mechanical?.partProcesses?.find((entry) => entry.partId === 'plate')?.material).toBe('PLA');
  await materials.last().selectOption('PLA');
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByText(/Generated CAD solids · 4 parts at revision/)).toBeVisible({ timeout: 60000 });

  await expect(panel.getByLabel('Resolved mechanical stack')).toBeVisible();
  await panel.getByLabel('Resolved mechanical stack').getByRole('button').filter({ hasText: 'Plate' }).first().click();
  const revision = (await saved(page)).revision;
  await page.getByRole('button', { name: 'Exploded', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Exploded', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'Section', exact: true }).click();
  await expect(page.getByText('Section at board centre · half removed', { exact: true })).toBeVisible();
  expect((await saved(page)).revision).toBe(revision);
  await page.reload();
  expect((await saved(page)).mechanical!.profiles[0]).toEqual(profile);
});

test('clicking a generated solid selects its resolved stack layer', async ({ page }) => {
  test.setTimeout(90_000);
  const panel = await configure(page);
  await panel.locator('summary').filter({ hasText: 'Advanced source geometry' }).click();
  await panel.getByRole('combobox', { name: 'Library fit profile', exact: true }).selectOption('mx-switch');
  await panel.getByRole('combobox', { name: 'Assign library fit profile to', exact: true }).selectOption('ergogen:ceoloide/switch_mx');
  await panel.locator('summary').filter({ hasText: 'Per-part process overrides' }).click();
  const materials = panel.getByRole('combobox', { name: 'Material', exact: true });
  await materials.first().selectOption('PLA');
  await materials.last().selectOption('PLA');
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByText(/Generated CAD solids · 4 parts at revision/)).toBeVisible({ timeout: 60_000 });
  const plate = panel.getByLabel('Resolved mechanical stack').getByRole('button').filter({ hasText: 'Plate' }).first();
  await expect(plate).toHaveAttribute('aria-pressed', 'false');
  await page.getByRole('region', { name: 'Canvas layers' }).getByRole('button', { name: 'Layers', exact: true }).click();
  await page.getByRole('button', { name: /^(Hide|Show) PCB$/, exact: true }).click();
  await page.getByRole('button', { name: /^(Hide|Show) Bottom$/, exact: true }).click();
  await page.getByRole('button', { name: /^(Hide|Show) Plate foam$/, exact: true }).click();
  await page.getByRole('button', { name: /^(Hide|Show) Bottom foam$/, exact: true }).click();
  await page.getByRole('button', { name: 'Top', exact: true }).click();
  const canvas = page.getByLabel('3D PCB assembly. Drag to orbit, scroll to zoom.');
  const bounds = await canvas.boundingBox();
  if (!bounds) throw new Error('Assembly canvas is not visible');
  // Include the outer rim: the floating controls leave centre probes that can hit switch openings.
  for (const [x, y] of [[0.5, 0.28], [0.35, 0.35], [0.5, 0.35], [0.65, 0.35], [0.35, 0.5], [0.5, 0.5], [0.65, 0.5], [0.35, 0.65], [0.5, 0.65], [0.65, 0.65]]) {
    await canvas.click({ position: { x: bounds.width * x, y: bounds.height * y } });
    if (await plate.getAttribute('aria-pressed') === 'true') break;
  }
  await expect(plate).toHaveAttribute('aria-pressed', 'true');
});

test('hardware and critical-fit drawing specifications persist with undo', async ({ page }) => {
  test.setTimeout(90_000);
  const panel = await configure(page);
  await panel.locator('summary').filter({ hasText: 'Suspension mounts' }).click();
  await panel.getByRole('button', { name: 'Add suspension mount', exact: true }).click();
  await expect.poll(async () => (await saved(page)).mechanical?.mounts.length).toBeGreaterThan(0);
  await panel.locator('summary').filter({ hasText: 'Hardware & critical fits' }).click();
  await panel.getByRole('button', { name: 'Add hardware', exact: true }).click();
  await panel.getByRole('textbox', { name: 'Thread for hardware 1', exact: true }).fill('M2 x 0.4 - 6g');
  await expect.poll(async () => (await saved(page)).mechanical?.hardware?.[0]?.thread).toBe('M2 x 0.4 - 6g');
  await panel.getByRole('button', { name: 'Add fit dimension', exact: true }).click();
  await panel.getByRole('textbox', { name: 'Label for critical fit 1', exact: true }).fill('Switch retention');
  await expect.poll(async () => (await saved(page)).mechanical?.criticalFits?.[0]?.label).toBe('Switch retention');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(panel.getByRole('textbox', { name: 'Label for critical fit 1', exact: true })).toHaveValue('Critical dimension');
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect.poll(async () => (await saved(page)).mechanical?.criticalFits?.[0]?.label).toBe('Switch retention');
  await page.reload();
  await navigateWorkspace(page, 'Case');
  await expect(panel.getByRole('textbox', { name: 'Thread for hardware 1', exact: true })).toHaveValue('M2 x 0.4 - 6g');
  await expect(panel.getByRole('textbox', { name: 'Label for critical fit 1', exact: true })).toHaveValue('Switch retention');
});

test('generated preview separates saved authored bodies and scopes configuration to its board', async ({ page }) => {
  await configure(page);
  await expect(page.getByRole('combobox', { name: 'Body type', exact: true })).toHaveCount(0);
  await expect(page.getByText(/Generated assembly preview.*authored case bodies remain saved/)).toBeVisible();
  await page.getByRole('button', { name: 'New board', exact: true }).click();
  await navigateWorkspace(page, 'Case');
  await expect(page.getByText(/Mechanical stack belongs to/)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Disable mechanical stack', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Show configured board', exact: true }).click();
  await expect(page.getByText('Configuration resolved', { exact: false })).toBeVisible();
});

test('generated-only boards keep their export ready when leaving the Case view', async ({ page }) => {
  test.setTimeout(90000);
  await page.goto('/');
  await expect.poll(async () => (await saved(page))?.boards.length).toBeGreaterThan(0);
  await page.evaluate(async () => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const opening = indexedDB.open('boardstudio-v2', 1);
      opening.onsuccess = () => resolve(opening.result);
      opening.onerror = () => reject(opening.error);
    });
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction('projects', 'readwrite');
      const store = transaction.objectStore('projects');
      const request = store.get('starter');
      request.onsuccess = () => store.put({ ...request.result, caseBodies: [] });
      transaction.oncomplete = () => { db.close(); resolve(); };
      transaction.onerror = () => { db.close(); reject(transaction.error); };
    });
  });
  const panel = await configure(page);
  await panel.locator('summary').filter({ hasText: 'Advanced source geometry' }).click();
  await panel.getByRole('combobox', { name: 'Library fit profile', exact: true }).selectOption('mx-switch');
  await panel.getByRole('combobox', { name: 'Assign library fit profile to', exact: true }).selectOption('ergogen:ceoloide/switch_mx');
  await panel.locator('summary').filter({ hasText: 'Per-part process overrides' }).click();
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Case generation' }).getByText(/Geometry current/)).toBeVisible({ timeout: 60000 });
  expect((await saved(page)).caseBodies).toHaveLength(0);
  await page.getByRole('button', { name: 'Export', exact: true }).first().click();
  const generated = page.locator('.wb-export-row').filter({ hasText: 'Generated mechanical package' });
  await expect(generated.getByRole('button', { name: 'Export', exact: true })).toBeEnabled();
  const authored = page.locator('.wb-export-row').filter({ hasText: 'Authored Case STEP' });
  await expect(authored.getByRole('button', { name: 'Export Authored Case STEP', exact: true })).toBeDisabled();
});
