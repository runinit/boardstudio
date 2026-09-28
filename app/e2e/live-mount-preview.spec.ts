import { auditSceneBounds } from './scene-bounds';
import { expect, test, type Page } from '@playwright/test';
import { navigateWorkspace } from './workspace-navigation';
import { splitFixture } from './splitMechanicalFixture';

const mountId = 'authored-mount';

function authoredFixture(clearance: number) {
  const document = splitFixture();
  document.mechanical = undefined;
  document.caseBodies = [{
    id: 'authored-tray', name: 'Authored tray', boardId: 'main-board', kind: 'tray',
    thickness: 3, clearance, materialId: 'pla', z: -10, wallHeight: 14,
    wallThickness: 2, mounts: [{ id: mountId, kind: 'hole', at: { x: 170, y: 5 }, holeDiameter: 2.5 }],
  }];
  return document;
}

async function saved(page: Page) {
  return page.evaluate(async () => {
    const id = localStorage.getItem('boardstudio-v2-active-project');
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('boardstudio-v2', 1);
      request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
    });
    return new Promise<any>((resolve, reject) => {
      const request = db.transaction('projects').objectStore('projects').get(id);
      request.onsuccess = () => { db.close(); resolve(request.result); };
      request.onerror = () => { db.close(); reject(request.error); };
    });
  });
}

async function openAuthoredCase(page: Page, clearance: number) {
  page.setDefaultTimeout(20_000);
  await auditSceneBounds(page);
  await page.addInitScript(() => {
    const scope = window as typeof window & { __mountAudit?: { requests: any[]; bounds: any[] } };
    scope.__mountAudit = { requests: [], bounds: [] } as any;
    const original = Worker.prototype.postMessage;
    const observed = new WeakSet<Worker>();
    Worker.prototype.postMessage = function (message: any, ...rest: any[]) {
      if (!observed.has(this)) {
        observed.add(this);
        this.addEventListener('message', event => {
          if (!event.data?.patch && event.data?.prepared?.bounds) scope.__mountAudit!.bounds.push(event.data.prepared.bounds);
        });
      }
      if (message?.kind === 'preview') scope.__mountAudit!.requests.push({ revision: message.ir?.revision, ir: message.ir });
      return original.call(this, message, ...rest);
    };
  });
  await page.goto('/');
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
  }, authoredFixture(clearance));
  await page.reload();
  await navigateWorkspace(page, 'Case');
  await expect(page.getByRole('region', { name: 'Case generation' })).toBeVisible();
  await expect(page.getByText(/Preview current|Geometry current/i).first()).toBeVisible({ timeout: 60_000 });
}

for (const { clearance, targetX, label } of [
  { clearance: 0.5, targetX: 180, label: 'inside the PCB' },
  { clearance: 5, targetX: 157, label: 'outside the PCB in expanded case material' },
]) test(`authored case mount ${label} previews, commits once, and undoes independently of the PCB`, async ({ page }) => {
  test.setTimeout(120_000);
  await openAuthoredCase(page, clearance);
  await page.getByRole('button', { name: 'Edit mounts', exact: true }).click();
  await page.getByRole('button', { name: 'Fit', exact: true }).click();
  await page.getByRole('button', { name: 'Top', exact: true }).click();

  const canvas = page.locator('.wb-assembly-scene canvas');
  const box = (await canvas.boundingBox())!;
  const bounds: number[] = await page.evaluate(() => (window as any).__fitBounds ?? []);
  expect(bounds).toHaveLength(4);
  const previewBounds = bounds;
  const vertical = 17 * Math.PI / 180;
  const horizontal = Math.atan(Math.tan(vertical) * box.width / box.height);
  const distance = previewBounds[3] / Math.sin(Math.min(vertical, horizontal)) * 1.16;
  const screen = (x: number, y: number) => {
    const dx = x - previewBounds[0], dy = y - previewBounds[1], dz = -9.2 - previewBounds[2];
    const depth = distance - dx * Math.cos(1.56) - dz * Math.sin(1.56);
    const scale = box.height / (2 * depth * Math.tan(vertical));
    return { x: box.x + box.width / 2 + (dx * Math.sin(1.56) - dz * Math.cos(1.56)) * scale, y: box.y + box.height / 2 - dy * scale };
  };
  const point = screen(170, 5), moved = screen(targetX, 5);
  await canvas.evaluate(element => {
    (window as any).__dragCanvasBounds = [];
    new ResizeObserver(() => {
      const { y, height } = element.getBoundingClientRect();
      (window as any).__dragCanvasBounds.push({ y, height });
    }).observe(element);
  });
  const before = await saved(page);
  const beforeRevision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
  expect(Number(before.revision)).toBe(beforeRevision);
  const pcbState = { boards: before.boards, outline: before.outline, parts: before.parts, matrices: before.matrices };
  await page.mouse.move(point.x, point.y); await page.mouse.down();
  await page.mouse.move(moved.x, moved.y, { steps: 5 });
  await expect.poll(() => page.evaluate(targetX => (window as any).__mountAudit.requests.some((entry: any) => entry.ir?.bodies?.some((body: any) => body.body?.mounts?.some((mount: any) => mount.id === 'authored-mount' && Math.abs(mount.at.x - targetX) < 0.1))), targetX)).toBe(true);
  expect(await page.locator('.wb-root').getAttribute('data-revision')).toBe(String(beforeRevision));
  const dragBounds = await page.evaluate(() => (window as any).__dragCanvasBounds as { y: number; height: number }[]);
  expect(dragBounds.length).toBeGreaterThan(0);
  for (const current of dragBounds) {
    expect(current.y).toBeCloseTo(box.y, 1);
    expect(current.height).toBeCloseTo(box.height, 1);
  }
  await page.keyboard.press('Escape'); await page.mouse.up();
  expect(await saved(page)).toMatchObject({ revision: beforeRevision, caseBodies: before.caseBodies });

  await page.mouse.move(point.x, point.y); await page.mouse.down();
  await page.mouse.move(moved.x, moved.y, { steps: 5 }); await page.mouse.up();
  await expect.poll(async () => (await saved(page)).caseBodies[0].mounts[0].at.x).toBeCloseTo(targetX, 0);
  expect((await saved(page)).revision).toBe(beforeRevision + 1);
  expect((await saved(page)).boards).toEqual(pcbState.boards);
  expect((await saved(page)).outline).toEqual(pcbState.outline);
  expect((await saved(page)).parts).toEqual(pcbState.parts);
  expect((await saved(page)).matrices).toEqual(pcbState.matrices);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(async () => (await saved(page)).caseBodies[0].mounts[0].at.x).toBe(170);
  expect((await saved(page)).boards).toEqual(pcbState.boards);
  expect((await saved(page)).outline).toEqual(pcbState.outline);
  expect((await saved(page)).parts).toEqual(pcbState.parts);
  expect((await saved(page)).matrices).toEqual(pcbState.matrices);
});
