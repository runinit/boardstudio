import { navigateWorkspace } from './workspace-navigation';
import { chooseScope } from './selection';
import { expect, test } from '@playwright/test';

test('unified Design branches share selection and restore the active view', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('tab', { name: 'PCB', exact: true })).toBeVisible();
  await page.getByRole('treeitem', { name: 'Column 3 3 keys', exact: true }).click();
  const count = await page.locator('.wb-scene-part').count();
  await navigateWorkspace(page, 'PCB');
  await expect(page.getByRole('region', { name: 'PCB canvas', exact: true })).toBeVisible();
  await expect(page.getByRole('tab', { name: 'PCB', exact: true })).toHaveAttribute('aria-selected', 'true');
  await expect(page.locator('.wb-scene-part')).toHaveCount(count);
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('button', { name: 'Back to PCB', exact: true }).click();
  await expect(page.getByRole('region', { name: 'PCB canvas', exact: true })).toBeVisible();
  await navigateWorkspace(page, 'Layout');
  await expect(page.getByRole('spinbutton', { name: 'Splay °', exact: true })).toBeVisible();
  await navigateWorkspace(page, 'Case');
  await expect(page.getByRole('region', { name: 'Case canvas', exact: true })).toBeVisible();
});

test('origin changes preserve positions, splay is undoable and saved', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 3 3 keys', exact: true }).click();
  const key = page.locator('.wb-scene-part.is-selected').first();
  const before = await key.getAttribute('transform');
  const origin = page.getByRole('spinbutton', { name: 'Origin Y mm', exact: true });
  await origin.fill('-30'); await origin.press('Enter');
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '1');
  await expect(key).toHaveAttribute('transform', before!);
  await page.getByRole('combobox', { name: 'Splay affects' }).selectOption('column');
  const other = page.getByRole('button', { name: /^SW4, MX switch/ });
  const otherBefore = await other.getAttribute('transform');
  const angle = page.getByRole('spinbutton', { name: 'Splay °', exact: true });
  await angle.fill('12'); await angle.press('Enter');
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '2');
  await expect(other).toHaveAttribute('transform', otherBefore!);
  await expect(key).not.toHaveAttribute('transform', before!);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(key).toHaveAttribute('transform', before!);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(angle).toHaveValue('12');
  await page.reload();
  await page.getByRole('treeitem', { name: 'Column 3 3 keys', exact: true }).click();
  await expect(origin).toHaveValue('-30');
  await expect(angle).toHaveValue('12');
});

test('labeled commands expose scope, snap and context actions with focus recovery', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 3 3 keys', exact: true }).click();
  await chooseScope(page, 'key');
  await expect(page.getByRole('button', { name: 'Select: Key', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Snap', exact: true }).click();
  await page.getByRole('combobox', { name: 'Snap increment' }).selectOption('0');
  await page.getByRole('checkbox', { name: 'Geometry snap', exact: true }).uncheck();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Snap', exact: true })).toBeFocused();
  await expect(page.getByRole('button', { name: 'Grid off', exact: true })).toBeVisible();
  await expect(page.getByText('Geometry snap off', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Add column', exact: true })).toBeVisible();
});

test('origin and splay handles preview, cancel and commit as one undo step', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 3 3 keys', exact: true }).click();
  await page.getByRole('button', { name: 'Transform', exact: true }).click();
  await page.getByRole('button', { name: 'Splay', exact: true }).click();
  const key = page.getByRole('button', { name: /^SW8, MX switch/ });
  const before = await key.getAttribute('transform');
  const origin = page.getByRole('button', { name: 'Move splay origin', exact: true });
  const originBefore = await origin.getAttribute('transform');
  const marker = (await origin.locator('.wb-handle-target').boundingBox())!;
  await page.mouse.move(marker.x + marker.width / 2, marker.y + marker.height / 2);
  await page.mouse.down();
  await page.mouse.move(marker.x + marker.width / 2 + 40, marker.y + marker.height / 2 + 50, { steps: 5 });
  await expect(origin).not.toHaveAttribute('transform', originBefore!);
  await expect(key).toHaveAttribute('transform', before!);
  await page.mouse.up();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '1');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(origin).toHaveAttribute('transform', originBefore!);

  const angle = page.getByRole('button', { name: 'Drag to splay', exact: true });
  const handle = (await angle.locator('.wb-handle-target').boundingBox())!;
  const start = { x: handle.x + handle.width / 2, y: handle.y + handle.height / 2 };
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + 35, start.y + 8, { steps: 5 });
  await expect(key).not.toHaveAttribute('transform', before!);
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await page.keyboard.press('Escape');
  await page.mouse.up();
  await expect(key).toHaveAttribute('transform', before!);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + 35, start.y + 8, { steps: 5 });
  await page.mouse.up();
  await expect(key).not.toHaveAttribute('transform', before!);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(key).toHaveAttribute('transform', before!);
});

test('row properties expose row offsets without column stagger settings', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 3 3 keys', exact: true }).click();
  await chooseScope(page, 'row');
  await expect(page.getByRole('spinbutton', { name: 'Offset X mm', exact: true })).toBeVisible();
  await expect(page.getByRole('spinbutton', { name: 'Stagger mm', exact: true })).toHaveCount(0);
});

test('dark comp, desktop and narrow layouts retain the canvas and usable controls', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('boardstudio:v2:theme', 'dark'));
  await page.setViewportSize({ width: 1672, height: 941 });
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 3 3 keys', exact: true }).click();
  await page.getByRole('button', { name: 'Transform', exact: true }).click();
  await page.getByRole('button', { name: 'Splay', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Move splay origin' })).toBeVisible();
  for (const width of [1440, 1280]) {
    await page.setViewportSize({ width, height: 900 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator('.wb-inspector')).toHaveClass(/is-compact/);
  await expect(page.getByRole('spinbutton', { name: 'Origin X mm', exact: true })).toBeVisible();
  expect(await page.locator('.wb-inspector-content').evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
  await page.getByRole('button', { name: 'Close panels', exact: true }).click({ position: { x: 3, y: 100 } });
  await page.getByRole('button', { name: 'Snap', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Snap increment' })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
