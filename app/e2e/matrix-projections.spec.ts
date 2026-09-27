import { expect, test, type Page } from '@playwright/test';
import { configureMatrix } from './matrix-setup';

declare global {
  interface Window {
    matrixDraftTest: { requests: number; replies: number; hold: boolean; release: (index: number) => void };
  }
}

async function nextPaint(page: Page) {
  await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const queued: (() => void)[] = [];
    const state = window.matrixDraftTest = { requests: 0, replies: 0, hold: false, release: (index: number) => queued[index]() };
    const OriginalWorker = window.Worker;
    window.Worker = class extends OriginalWorker {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        this.addEventListener('message', (event) => { if (event.data?.kind === 'matrix-projections') state.replies += 1; });
      }
      postMessage(message: unknown, transfer: Transferable[] = []): void {
        if ((message as { kind?: string })?.kind === 'project-matrices') {
          state.requests += 1;
          if (state.hold) { queued.push(() => super.postMessage(message, transfer)); return; }
        }
        super.postMessage(message, transfer);
      }
    };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await page.getByRole('button', { name: 'Skip guide', exact: true }).click();
});

test('pointer motion translates cached origin projection without another core request', async ({ page }) => {
  await configureMatrix(page, 2, 2);
  const ghost = page.locator('.is-placement-preview');
  await expect(ghost.locator('.wb-matrix-cell')).toHaveCount(4);
  await expect.poll(() => page.evaluate(() => window.matrixDraftTest.requests)).toBe(1);
  const canvas = page.locator('svg.wb-canvas');
  const bounds = await canvas.boundingBox();
  const before = await ghost.evaluate((element) => element.parentElement?.getAttribute('transform'));
  await page.mouse.move(bounds!.x + bounds!.width * 0.4, bounds!.y + bounds!.height * 0.4);
  await page.mouse.move(bounds!.x + bounds!.width * 0.6, bounds!.y + bounds!.height * 0.6);
  await nextPaint(page);
  expect(await ghost.evaluate((element) => element.parentElement?.getAttribute('transform'))).not.toBe(before);
  expect(await page.evaluate(() => window.matrixDraftTest.requests)).toBe(1);
  await page.keyboard.press('Escape');
  await expect(ghost).toHaveCount(0);
});

test('cancelled and superseded draft replies cannot restore an obsolete ghost', async ({ page }) => {
  await page.evaluate(() => { window.matrixDraftTest.hold = true; });
  await configureMatrix(page, 2, 2);
  await expect.poll(() => page.evaluate(() => window.matrixDraftTest.requests)).toBe(1);
  await page.keyboard.press('Escape');
  await page.evaluate(() => window.matrixDraftTest.release(0));
  await expect.poll(() => page.evaluate(() => window.matrixDraftTest.replies)).toBe(1);
  await nextPaint(page);
  await expect(page.locator('.is-placement-preview')).toHaveCount(0);

  await configureMatrix(page, 2, 3);
  await expect.poll(() => page.evaluate(() => window.matrixDraftTest.requests)).toBe(2);
  await page.keyboard.press('Escape');
  await configureMatrix(page, 3, 3);
  await expect.poll(() => page.evaluate(() => window.matrixDraftTest.requests)).toBe(3);
  await page.evaluate(() => window.matrixDraftTest.release(2));
  await expect(page.locator('.is-placement-preview .wb-matrix-cell')).toHaveCount(9);
  await page.evaluate(() => window.matrixDraftTest.release(1));
  await expect.poll(() => page.evaluate(() => window.matrixDraftTest.replies)).toBe(3);
  await nextPaint(page);
  await expect(page.locator('.is-placement-preview .wb-matrix-cell')).toHaveCount(9);
  await page.getByRole('button', { name: 'Ghost key, row 1, column 1' }).click();
  await expect(page.locator('.is-placement-preview')).toHaveCount(0);
});

test('switching boards cancels a held placement draft', async ({ page }) => {
  await expect(page.getByRole('treeitem', { name: /0 parts/ }).first()).toBeVisible();
  const boardPicker = page.getByRole('combobox', { name: 'Selected board' });
  const firstBoard = await boardPicker.inputValue();
  await page.getByRole('button', { name: 'New board', exact: true }).click();
  await expect(boardPicker).not.toHaveValue(firstBoard);
  const secondBoard = await boardPicker.inputValue();
  await boardPicker.selectOption(firstBoard);
  await page.evaluate(() => { window.matrixDraftTest.hold = true; });
  await configureMatrix(page, 2, 2);
  await expect.poll(() => page.evaluate(() => window.matrixDraftTest.requests)).toBe(1);
  await boardPicker.selectOption(secondBoard);
  await page.evaluate(() => window.matrixDraftTest.release(0));
  await expect.poll(() => page.evaluate(() => window.matrixDraftTest.replies)).toBe(1);
  await nextPaint(page);
  await expect(page.locator('svg.wb-canvas')).not.toHaveClass(/is-placing-matrix/);
  await expect(page.locator('.is-placement-preview')).toHaveCount(0);
  await page.locator('svg.wb-canvas').focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('treeitem', { name: /0 parts/ }).first()).toBeVisible();
  await boardPicker.selectOption(firstBoard);
  await expect(page.getByRole('treeitem', { name: /0 parts/ }).first()).toBeVisible();
});

test('paired drafts share one request and translate together', async ({ page }) => {
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Mirrored pair…', exact: true }).click();
  await page.getByRole('spinbutton', { name: 'Rows per half', exact: true }).fill('2');
  await page.getByRole('spinbutton', { name: 'Columns per half', exact: true }).fill('3');
  await page.getByRole('button', { name: 'Preview placement', exact: true }).click();
  const ghosts = page.locator('.wb-matrix-ghost.is-placement-preview');
  await expect(ghosts).toHaveCount(2);
  await expect(ghosts.nth(0).locator('.wb-matrix-cell')).toHaveCount(6);
  await expect(ghosts.nth(1).locator('.wb-matrix-cell')).toHaveCount(6);
  expect(await page.evaluate(() => window.matrixDraftTest.requests)).toBe(1);
  const before = await ghosts.first().evaluate((element) => element.parentElement?.getAttribute('transform'));
  const bounds = (await page.locator('.wb-canvas').boundingBox())!;
  await page.mouse.move(bounds.x + bounds.width * 0.65, bounds.y + bounds.height * 0.65);
  await nextPaint(page);
  expect(await ghosts.first().evaluate((element) => element.parentElement?.getAttribute('transform'))).not.toBe(before);
  expect(await page.evaluate(() => window.matrixDraftTest.requests)).toBe(1);
  await page.keyboard.press('Escape');
  await expect(ghosts).toHaveCount(0);
});
