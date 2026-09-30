import { auditSceneBounds } from './scene-bounds';
import { expect, test } from '@playwright/test';
import { navigateWorkspace } from './workspace-navigation';
import { splitFixture } from './splitMechanicalFixture';

/**
 * Contract tests for the live case path.  These deliberately use the public
 * status and action names so they continue to describe the workflow if the
 * inspector layout changes.
 */
async function openCase(page: import('@playwright/test').Page, fixture = false, legacyGasket = false) {
  page.setDefaultTimeout(20_000);
  await auditSceneBounds(page);
  await page.addInitScript(() => {
    const scope = window as typeof window & { __livePreviewAudit?: { requests: any[]; replies: any[]; cadDelay: number } };
    const audit = scope.__livePreviewAudit = { requests: [], replies: [], assemblies: [], bounds: [], fullScenes: 0, cadDelay: 0 };
    const original = Worker.prototype.postMessage;
    const observed = new WeakSet<Worker>();
    Worker.prototype.postMessage = function (message: any, ...rest: any[]) {
      if (!observed.has(this)) {
        observed.add(this);
        this.addEventListener('message', event => {
          if (event.data?.kind === 'preview') audit.replies.push({ revision: event.data.result?.revision });
          if (event.data?.kind === 'mechanical-resolved') audit.assemblies.push(event.data.assembly);
          if (!event.data.patch && event.data?.prepared?.bounds) audit.bounds.push(event.data.prepared.bounds);
        });
      }
      if (message?.scene?.kind === 'assembly') audit.fullScenes += 1;
      if (message?.kind === 'preview') {
        audit.requests.push({ revision: message.ir?.revision });
        if (audit.cadDelay > 0) {
          setTimeout(() => original.call(this, message, ...rest), audit.cadDelay);
          return;
        }
      }
      return original.call(this, message, ...rest);
    };
  });
  await page.goto('/');
  if (fixture) {
    await page.evaluate(async document => {
      const db = await new Promise<IDBDatabase>((resolve, reject) => {
        const request = indexedDB.open('boardstudio-v2', 1);
        request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
      });
      await new Promise<void>((resolve, reject) => {
        const tx = db.transaction('projects', 'readwrite');
        tx.objectStore('projects').put(document);
        tx.oncomplete = () => resolve(); tx.onerror = () => reject(tx.error);
      });
      db.close(); localStorage.setItem('boardstudio-v2-active-project', document.id);
    }, splitFixture(legacyGasket));
    await page.reload();
  }
  await navigateWorkspace(page, 'Case');
  await expect(page.locator('.wb-assembly-scene canvas')).toBeVisible();
  await expect(page.getByRole('region', { name: 'Case generation' })).toBeVisible();
  const configure = page.getByRole('button', { name: 'Configure mechanical stack', exact: true });
  const canConfigure = await configure.count();
  if (canConfigure) await configure.click();
  if (fixture || canConfigure) {
    await expect(page.getByText(/^Generated assembly preview ·/)).toHaveCount(1);
    await expect(page.getByRole('region', { name: 'Case generation' })).toHaveAttribute('data-state', 'ready', { timeout: 60_000 });
  }
}

function liveSwitch(page: import('@playwright/test').Page) {
  return page.getByRole('switch', { name: /Live preview/i }).first();
}

test.describe('live case preview', () => {
  test('starts enabled with an initial current preview', async ({ page }) => {
    test.setTimeout(90_000);
    await openCase(page);
    await expect(liveSwitch(page)).toBeChecked();
    await expect(page.getByText(/Preview current|Geometry current/i).first()).toBeVisible({ timeout: 60_000 });
    await expect(page.locator('.wb-case-generation').getByRole('button', { name: /Update preview/i })).toHaveCount(1);
  });

  test('numeric edits schedule the latest preview and keep the canvas usable', async ({ page }) => {
    test.setTimeout(120_000);
    await openCase(page, true);
    await expect(page.getByText(/Preview current|Geometry current/i).first()).toBeVisible({ timeout: 60_000 });
    const canvas = page.locator('.wb-assembly-scene canvas');
    const before = await page.evaluate(() => (window as any).__livePreviewAudit.replies.length);
    await expect(page.getByText('Preparing 3D geometry…', { exact: true })).toHaveCount(0);
    const fullScenes = await page.evaluate(() => (window as any).__livePreviewAudit.fullScenes);
    const thickness = page.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true });
    await page.locator('.wb-mechanical-panel summary').filter({ hasText: 'Dimensions & clearances' }).click();
    await thickness.fill('2.5');
    await thickness.press('Tab');
    await expect(page.getByText(/Updating preview|Preview current|Geometry current/i).first()).toBeVisible();
    await expect.poll(() => page.evaluate(() => (window as any).__livePreviewAudit.replies.length), { timeout: 60_000 }).toBeGreaterThan(before);
    const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
    await expect.poll(() => page.evaluate(() => (window as any).__livePreviewAudit.replies.at(-1)?.revision)).toBe(revision);
    await expect(page.getByText('Preparing 3D geometry…', { exact: true })).toHaveCount(0);
    expect(await page.evaluate(() => (window as any).__livePreviewAudit.fullScenes)).toBe(fullScenes);
    expect((await canvas.screenshot()).length).toBeGreaterThan(0);
  });

  test('pause retains the last usable solids and exposes one manual update action', async ({ page }) => {
    test.setTimeout(120_000);
    await openCase(page);
    await expect(page.getByText(/Preview current|Geometry current/i).first()).toBeVisible({ timeout: 60_000 });
    await liveSwitch(page).uncheck();
    await expect(liveSwitch(page)).not.toBeChecked();
    const before = await page.evaluate(() => (window as any).__livePreviewAudit.replies.length);
    const thickness = page.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true });
    await page.locator('.wb-mechanical-panel summary').filter({ hasText: 'Dimensions & clearances' }).click();
    await thickness.fill('2.5');
    await thickness.press('Tab');
    await expect(page.getByRole('button', { name: /Update preview/i })).toHaveCount(1);
    await expect(page.getByText(/previous geometry|Preview paused|Update preview/i).first()).toBeVisible();
    await page.waitForTimeout(250);
    expect(await page.evaluate(() => (window as any).__livePreviewAudit.replies.length)).toBe(before);
    await expect(page.locator('.wb-case-generation')).toHaveAttribute('data-state', /required|paused/);
  });

  test('cancel pauses the live run and stale work cannot replace the retained preview', async ({ page }) => {
    test.setTimeout(120_000);
    await openCase(page);
    await expect(page.getByText(/Preview current|Geometry current/i).first()).toBeVisible({ timeout: 60_000 });
    await page.evaluate(() => { (window as any).__livePreviewAudit.cadDelay = 1200; });
    const thickness = page.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true });
    await page.locator('.wb-mechanical-panel summary').filter({ hasText: 'Dimensions & clearances' }).click();
    await thickness.fill('2.5');
    await thickness.press('Tab');
    await page.getByRole('button', { name: /Cancel/i }).click();
    await expect(liveSwitch(page)).not.toBeChecked();
    await expect(page.getByRole('button', { name: /Update preview/i })).toBeVisible();
    await expect(page.getByText(/previous geometry|Preview paused/i).first()).toBeVisible();
  });

  test('mobile keeps the live status and update action reachable in the collapsed inspector', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openCase(page);
    const settings = page.getByRole('button', { name: 'Case settings', exact: true });
    await expect(settings).toBeVisible();
    if (await settings.getAttribute('aria-expanded') === 'false') await settings.click();
    await expect(liveSwitch(page)).toBeVisible();
    await expect(page.getByRole('region', { name: 'Case generation' })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });

  test('Escape exits an active case edit without creating a second document revision', async ({ page }) => {
    test.setTimeout(90_000);
    await openCase(page);
    await expect(page.getByText(/Preview current|Geometry current/i).first()).toBeVisible({ timeout: 60_000 });
    await page.getByRole('combobox', { name: 'Mount style', exact: true }).selectOption('gasket');
    const edit = page.getByRole('button', { name: 'Edit gaskets', exact: true });
    await expect(edit).toBeVisible();
    await edit.click();
    const revision = await page.locator('.wb-root').getAttribute('data-revision');
    await page.keyboard.press('Escape');
    await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
    await expect(page.getByRole('button', { name: 'Edit gaskets', exact: true })).toBeVisible();
  });

  test('gasket drag commits once on release, Undo restores it, and Escape cancels a draft', async ({ page }) => {
    test.setTimeout(120_000);
    await openCase(page, true, true);
    await expect(page.getByRole('combobox', { name: 'Mount style', exact: true })).toHaveValue('gasket');
    await expect.poll(() => page.evaluate(() => (window as any).__livePreviewAudit.assemblies.at(-1)?.gasketSupports?.length)).toBe(12);
    await page.getByRole('button', { name: /Update preview/i }).first().click();
    await expect(page.getByText(/Preview current|Geometry current/i).first()).toBeVisible({ timeout: 60_000 });
    await page.getByRole('button', { name: 'Edit gaskets', exact: true }).click();
    await page.getByRole('button', { name: 'Fit', exact: true }).click();
    await page.getByRole('button', { name: 'Top', exact: true }).click();

    const beforeRevision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
    const canvas = page.locator('.wb-assembly-scene canvas');
    const box = (await canvas.boundingBox())!;
    const bounds: number[] = await page.evaluate(() => (window as any).__fitBounds);
    const assembly: any = await page.evaluate(() => (window as any).__livePreviewAudit.assemblies.at(-1));
    const retainer = assembly.stack.find((layer: any) => layer.id === 'retainer');
    const support = assembly.gasketSupports.find((item: any) => item.regionId === 'left');
    const vertical = 17 * Math.PI / 180;
    const horizontal = Math.atan(Math.tan(vertical) * box.width / box.height);
    const distance = bounds[3] / Math.sin(Math.min(vertical, horizontal)) * 1.16;
    const screen = (x: number, y: number) => {
      const dx = x - bounds[0], dy = y - bounds[1], dz = retainer.z + retainer.thickness + 1 - bounds[2];
      const depth = distance - dx * Math.cos(1.56) - dz * Math.sin(1.56);
      const scale = box.height / (2 * depth * Math.tan(vertical));
      return { x: box.x + box.width / 2 + (dx * Math.sin(1.56) - dz * Math.cos(1.56)) * scale, y: box.y + box.height / 2 - dy * scale };
    };
    const point = screen(support.at.x, support.at.y);
    const moved = screen(support.at.x + support.tangent.x * 2, support.at.y + support.tangent.y * 2);
    await page.mouse.move(point.x, point.y);
    await page.mouse.down();
    await page.mouse.move(moved.x, moved.y, { steps: 4 });
    await page.keyboard.press('Escape');
    await page.mouse.up();
    await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', String(beforeRevision));

    // A second drag exercises the release transaction. It must create one
    // document revision and one preview request, rather than one per move.
    await page.mouse.move(point.x, point.y);
    await page.mouse.down();
    await page.mouse.move(moved.x, moved.y, { steps: 5 });
    await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', String(beforeRevision));
    await page.mouse.up();
    await expect.poll(() => page.evaluate((id) => (window as any).__livePreviewAudit.assemblies.at(-1)?.gasketSupports.find((item: any) => item.id === id)?.anchor, support.id)).not.toBe(support.anchor);
    const committedAnchor = await page.evaluate((id) => (window as any).__livePreviewAudit.assemblies.at(-1)?.gasketSupports.find((item: any) => item.id === id)?.anchor, support.id);
    await page.getByRole('button', { name: 'Undo', exact: true }).click();
    await expect.poll(() => page.evaluate((id) => (window as any).__livePreviewAudit.assemblies.at(-1)?.gasketSupports.find((item: any) => item.id === id)?.anchor, support.id)).toBe(support.anchor);
    expect(committedAnchor).not.toBe(support.anchor);
  });
});
