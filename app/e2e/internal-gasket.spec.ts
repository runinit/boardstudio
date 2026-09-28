import { expect, test } from '@playwright/test';
import { navigateWorkspace } from './workspace-navigation';

test('gasket controls generate internal geometry and retain dimensions through undo and reopen', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await page.getByRole('combobox', { name: 'Mount style', exact: true }).selectOption('gasket');
  const panel = page.locator('.wb-mechanical-panel');
  await expect(panel.getByRole('combobox', { name: 'Gasket size', exact: true })).toHaveValue('custom');
  await expect(panel.getByRole('button', { name: /^Top case / })).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 45_000 });

  const size = panel.getByRole('combobox', { name: 'Gasket size', exact: true });
  await expect(size.locator('option')).toHaveCount(14);
  await size.selectOption('B3');
  await expect(panel.getByRole('spinbutton', { name: 'Gasket length mm', exact: true })).toHaveValue('20');
  await expect(panel.getByRole('spinbutton', { name: 'Gasket width mm', exact: true })).toHaveValue('4');
  await expect(panel.getByRole('spinbutton', { name: 'Gasket thickness mm', exact: true })).toHaveValue('3');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(size).toHaveValue('custom');
  await expect(panel.getByRole('spinbutton', { name: 'Gasket thickness mm', exact: true })).toHaveValue('2');
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(size).toHaveValue('B3');
  await expect(page.locator('.wb-save-state')).toHaveClass(/is-saved/);

  await page.reload();
  await navigateWorkspace(page, 'Case');
  await expect(size).toHaveValue('B3');
  await expect(panel.getByRole('spinbutton', { name: 'Gasket thickness mm', exact: true })).toHaveValue('3');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 45_000 });
});

test('advanced gasket edits cancel on Escape and commit as custom dimensions', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await page.getByRole('combobox', { name: 'Mount style', exact: true }).selectOption('gasket');
  const thickness = page.getByRole('spinbutton', { name: 'Gasket thickness mm', exact: true });
  await thickness.fill('4');
  await thickness.press('Escape');
  await expect(thickness).toHaveValue('2');
  await thickness.fill('3');
  await thickness.press('Enter');
  await expect(thickness).toHaveValue('3');
  await expect(page.getByRole('combobox', { name: 'Gasket size', exact: true })).toHaveValue('custom');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(thickness).toHaveValue('2');
});

test('support count preserves adopted closures and an invalid wall blocks export', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await page.getByRole('combobox', { name: 'Mount style', exact: true }).selectOption('gasket');
  const panel = page.locator('.wb-mechanical-panel');
  const exportButton = page.getByRole('button', { name: 'Export geometry', exact: true });
  await expect(exportButton).toBeEnabled({ timeout: 45_000 });
  await panel.locator('summary').filter({ hasText: 'Suggested mount locations' }).click();
  await page.getByRole('button', { name: 'Adopt closure positions', exact: true }).click();
  const closures = panel.locator('details').filter({ has: page.locator('summary').filter({ hasText: /^Closure screws/ }) });
  const xs = closures.getByRole('spinbutton', { name: 'Position X mm', exact: true });
  const ys = closures.getByRole('spinbutton', { name: 'Position Y mm', exact: true });
  await expect(xs).toHaveCount(4);
  const positions = await Promise.all([xs.evaluateAll(inputs => inputs.map(input => (input as HTMLInputElement).value)), ys.evaluateAll(inputs => inputs.map(input => (input as HTMLInputElement).value))]);
  const count = panel.getByRole('spinbutton', { name: 'Supports per region', exact: true });
  await count.fill('3');
  await count.press('Enter');
  await expect(exportButton).toBeEnabled({ timeout: 45_000 });
  expect(await xs.evaluateAll(inputs => inputs.map(input => (input as HTMLInputElement).value))).toEqual(positions[0]);
  expect(await ys.evaluateAll(inputs => inputs.map(input => (input as HTMLInputElement).value))).toEqual(positions[1]);
  await panel.locator('summary').filter({ hasText: 'Advanced gasket clearances' }).click();
  const wall = panel.getByRole('spinbutton', { name: 'Minimum wall behind gasket pockets mm', exact: true });
  await wall.fill('3');
  await wall.press('Enter');
  await expect(page.getByText('Nominal wall is smaller than the minimum wall behind gasket pockets.', { exact: true })).toBeVisible();
  await expect(exportButton).toBeDisabled();
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(wall).toHaveValue('2');
  await expect(exportButton).toBeEnabled({ timeout: 45_000 });
});
