import { expect, test, type Locator, type Page } from '@playwright/test';
import { configureMatrix } from './matrix-setup';
import { chooseScope } from './selection';

async function createMatrix(page: Page): Promise<void> {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await page.getByRole('button', { name: 'Back to objects', exact: true }).click();
  await configureMatrix(page);
  await expect(page.locator('.wb-scene-part')).toHaveCount(60);
}

async function beginDrag(page: Page, target: Locator, dx = 40): Promise<void> {
  const box = await target.boundingBox();
  expect(box).not.toBeNull();
  const x = box!.x + box!.width / 2;
  const y = box!.y + box!.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y, { steps: 5 });
}

test('pointercancel restores a moved part before any persisted commit', async ({ page }) => {
  await createMatrix(page);
  const part = page.getByRole('button', { name: /^SW1, switch mx,/ });
  const before = await part.getAttribute('aria-label');
  const revision = await page.locator('.wb-root').getAttribute('data-revision');

  await beginDrag(page, part);
  await expect(part).not.toHaveAttribute('aria-label', before!);
  await part.evaluate((element) => element.dispatchEvent(new PointerEvent('pointercancel', { bubbles: true, pointerId: 1 })));

  await expect(part).toHaveAttribute('aria-label', before!);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
});

test('Escape cancels a matrix drag and restores every affected key', async ({ page }) => {
  await createMatrix(page);
  await chooseScope(page, 'matrix');
  const first = page.getByRole('button', { name: /^SW1, switch mx,/ });
  const second = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(1);
  const firstBefore = await first.getAttribute('aria-label');
  const secondBefore = await second.getAttribute('aria-label');
  const revision = await page.locator('.wb-root').getAttribute('data-revision');

  await beginDrag(page, first);
  await expect(first).not.toHaveAttribute('aria-label', firstBefore!);
  await expect(second).not.toHaveAttribute('aria-label', secondBefore!);
  await page.keyboard.press('Escape');

  await expect(first).toHaveAttribute('aria-label', firstBefore!);
  await expect(second).toHaveAttribute('aria-label', secondBefore!);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
});

test('switching boards cancels an active drag without carrying it across boards', async ({ page }) => {
  await createMatrix(page);
  const beforeBoard = Number(await page.locator('.wb-root').getAttribute('data-revision'));
  await page.getByRole('button', { name: 'New board', exact: true }).click();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', String(beforeBoard + 1));
  await expect(page.getByRole('treeitem', { name: /0 parts/ }).first()).toBeVisible();
  await page.getByRole('combobox', { name: 'Selected board' }).selectOption({ label: 'Main board' });
  await expect(page.getByRole('treeitem', { name: /60 parts/ }).first()).toBeVisible();

  const part = page.getByRole('button', { name: /^SW1, switch mx,/ });
  const before = await part.getAttribute('aria-label');
  const revision = await page.locator('.wb-root').getAttribute('data-revision');

  await beginDrag(page, part);
  await expect(part).not.toHaveAttribute('aria-label', before!);
  await page.getByRole('combobox', { name: 'Selected board' }).selectOption({ label: 'Board 2' });
  await expect(page.getByRole('treeitem', { name: /0 parts/ }).first()).toBeVisible();
  await page.mouse.up();
  await page.getByRole('combobox', { name: 'Selected board' }).selectOption({ label: 'Main board' });
  await expect(part).toHaveAttribute('aria-label', before!);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
});

test('a committed matrix drag is restored by exactly one undo', async ({ page }) => {
  await createMatrix(page);
  await chooseScope(page, 'matrix');
  const first = page.getByRole('button', { name: /^SW1, switch mx,/ });
  const second = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(1);
  const firstBefore = await first.getAttribute('aria-label');
  const secondBefore = await second.getAttribute('aria-label');
  const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));

  await beginDrag(page, first);
  await page.mouse.up();
  await expect(first).not.toHaveAttribute('aria-label', firstBefore!);
  await expect(second).not.toHaveAttribute('aria-label', secondBefore!);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', String(revision + 1));

  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(first).toHaveAttribute('aria-label', firstBefore!);
  await expect(second).toHaveAttribute('aria-label', secondBefore!);
});
