import { expect, test } from '@playwright/test';
import { chooseScope } from './selection';

test('desktop scopes and compact review retain status and fit actions', async ({ page }) => {
 await page.goto('/');
 await expect(page.getByRole('button', { name: /^Select:/ })).toBeVisible();
 await page.getByRole('treeitem', { name: 'Column 1 3 keys', exact: true }).click();
 await chooseScope(page, 'key');
 await expect(page.getByRole('button', { name: 'Select: Key', exact: true })).toBeVisible();
 const before = await page.locator('.wb-canvas').getAttribute('viewBox');
 await page.getByRole('button', { name: 'Fit selection', exact: true }).click();
 await expect(page.locator('.wb-canvas')).not.toHaveAttribute('viewBox', before!);
 await page.getByRole('button', { name: 'Fit board', exact: true }).click();
 for (const width of [760, 390]) {
  await page.setViewportSize({ width, height: 844 });
  const inspect = page.getByRole('button', { name: 'Inspect', exact: true });
  if (await inspect.getAttribute('aria-expanded') === 'true') await inspect.click();
  await expect(page.getByRole('combobox', { name: 'Workspace', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: /^Select:/ })).toBeVisible();
  await expect(page.getByRole('group', { name: 'Selection scope' })).toBeHidden();
  await expect(page.locator('.wb-save-state summary')).toHaveAccessibleName('Saved locally');
  await page.locator('.wb-save-state summary').click();
  await expect(page.getByRole('status').filter({ hasText: 'Changes are saved' })).toBeVisible();
  await page.locator('.wb-save-state summary').click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.getByRole('button', { name: 'Fit selection', exact: true }).click();
 }
});

test('a failed local write replaces the saved indicator with recovery guidance', async ({ page }) => {
 await page.goto('/');
 await expect(page.locator('.wb-save-state summary')).toHaveAccessibleName('Saved locally');
 await page.evaluate(() => {
  IDBObjectStore.prototype.put = function () { throw new DOMException('Storage is full', 'QuotaExceededError'); };
 });
 await page.getByRole('textbox', { name: 'Board name', exact: true }).fill('Unsaved name');
 await page.getByRole('textbox', { name: 'Board name', exact: true }).blur();
 await expect(page.locator('.wb-save-state summary')).toHaveAccessibleName('Local save failed');
 await page.locator('.wb-save-state summary').click();
 await expect(page.getByRole('status').filter({ hasText: 'could not be saved' })).toBeVisible();
});
