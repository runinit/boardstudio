import { expect, test, type Page } from '@playwright/test';
import { navigateWorkspace } from './workspace-navigation';

async function configureGaskets(page: Page) {
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await page.getByRole('combobox', { name: 'Mount style', exact: true }).selectOption('gasket');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 45_000 });
}

test('case assembly tree groups gaskets and selects focused part inspectors', async ({ page }) => {
  test.setTimeout(90_000);
  await configureGaskets(page);
  const tree = page.getByRole('tree', { name: 'CAD structure' });
  await expect(tree.getByRole('button', { name: 'Expand PCB', exact: true })).toBeVisible();
  await expect(tree.getByRole('treeitem', { name: /^Gasket \d/ })).toHaveCount(4);
  await tree.getByRole('treeitem', { name: 'Plate', exact: true }).click();
  const panel = page.locator('.wb-mechanical-panel');
  await expect(panel.getByRole('heading', { name: 'Plate', exact: true })).toBeVisible();
  await expect(panel.getByRole('spinbutton', { name: 'Plate thickness mm', exact: true })).toBeVisible();
  await expect(panel.getByText('Closure hardware', { exact: true })).toHaveCount(0);
  await tree.getByRole('treeitem', { name: /^Gasket 1 / }).click();
  await expect(panel.getByRole('heading', { name: 'Gasket 1', exact: true })).toBeVisible();
  await expect(panel.getByRole('spinbutton', { name: 'Cut length mm', exact: true })).toBeVisible();
  await expect(panel.getByRole('spinbutton', { name: 'Plate thickness mm', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Edit gaskets', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await tree.getByRole('treeitem', { name: 'Plate', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Edit gaskets', exact: true })).toHaveAttribute('aria-pressed', 'false');
});

test('an invalid individual gasket stays saved and resizing repairs it with undo and redo', async ({ page }) => {
  test.setTimeout(90_000);
  await configureGaskets(page);
  const tree = page.getByRole('tree', { name: 'CAD structure' });
  await tree.getByRole('treeitem', { name: /^Gasket 1 / }).click();
  const length = page.getByRole('spinbutton', { name: 'Cut length mm', exact: true });
  const original = await length.inputValue();
  await length.fill('80');
  await length.press('Enter');
  await expect(page.locator('.wb-mech-error').first()).toContainText('Gasket');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeDisabled();
  await expect(length).toHaveValue('80');
  await expect(tree.getByRole('treeitem', { name: /^Gasket 1 / })).toContainText('Does not fit');
  await expect(page.locator('.wb-save-state')).toHaveClass(/is-saved/);
  await page.reload();
  await navigateWorkspace(page, 'Case');
  await tree.getByRole('treeitem', { name: /^Gasket 1 / }).click();
  await expect(length).toHaveValue('80');
  await length.fill(original);
  await length.press('Enter');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 45_000 });
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(length).toHaveValue('80');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(length).toHaveValue(original);
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 45_000 });
});

test('gasket material controls cancel edits and retain stock changes through reopen', async ({ page }) => {
  test.setTimeout(90_000);
  await configureGaskets(page);
  const group = page.getByRole('treeitem', { name: /^Gaskets / });
  await group.click();
  const thickness = page.getByRole('spinbutton', { name: 'Gasket thickness mm', exact: true });
  await thickness.fill('4');
  await thickness.press('Escape');
  await expect(thickness).toHaveValue('2');
  const stock = page.getByRole('combobox', { name: 'Foam stock', exact: true });
  await expect(stock.locator('option')).toHaveCount(14);
  await stock.selectOption('B2');
  await expect(page.getByRole('spinbutton', { name: 'Gasket width mm', exact: true })).toHaveValue('4');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(stock).toHaveValue('custom');
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(stock).toHaveValue('B2');
  await expect(page.locator('.wb-save-state')).toHaveClass(/is-saved/);
  await page.reload();
  await navigateWorkspace(page, 'Case');
  await group.click();
  await expect(stock).toHaveValue('B2');
  await expect(page.getByRole('checkbox', { name: 'Fit cut lengths automatically on four sides', exact: true })).toBeChecked();
});

test('changing support count preserves adopted closures and invalid walls block export', async ({ page }) => {
  test.setTimeout(90_000);
  await configureGaskets(page);
  const panel = page.locator('.wb-mechanical-panel');
  await panel.locator('summary').filter({ hasText: 'Suggested mount locations' }).click();
  await page.getByRole('button', { name: 'Adopt closure positions', exact: true }).click();
  const closures = panel.locator('details').filter({ has: page.locator('summary').filter({ hasText: /^Closure screws/ }) });
  const xs = closures.getByRole('spinbutton', { name: 'Position X mm', exact: true });
  const ys = closures.getByRole('spinbutton', { name: 'Position Y mm', exact: true });
  await expect(xs).toHaveCount(4);
  const positions = await Promise.all([xs.evaluateAll(inputs => inputs.map(input => (input as HTMLInputElement).value)), ys.evaluateAll(inputs => inputs.map(input => (input as HTMLInputElement).value))]);
  await page.getByRole('treeitem', { name: /^Gaskets / }).click();
  const count = page.getByRole('spinbutton', { name: 'Supports per region', exact: true });
  await count.fill('5');
  await count.press('Enter');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 45_000 });
  await page.getByRole('button', { name: 'Assembly settings', exact: true }).click();
  await expect(xs).toHaveCount(4);
  expect(await xs.evaluateAll(inputs => inputs.map(input => (input as HTMLInputElement).value))).toEqual(positions[0]);
  expect(await ys.evaluateAll(inputs => inputs.map(input => (input as HTMLInputElement).value))).toEqual(positions[1]);
  await page.getByRole('treeitem', { name: /^Gaskets / }).click();
  await panel.locator('summary').filter({ hasText: 'Advanced gasket clearances' }).click();
  const wall = panel.getByRole('spinbutton', { name: 'Minimum wall behind gasket pockets mm', exact: true });
  await wall.fill('3');
  await wall.press('Enter');
  await expect(page.getByText('Nominal wall is smaller than the minimum wall behind gasket pockets.', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(wall).toHaveValue('2');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 45_000 });
});
