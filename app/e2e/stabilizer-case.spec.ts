import { expect, test } from '@playwright/test';
import { navigateWorkspace } from './workspace-navigation';

for (const name of ['GH60 · ANSI', 'Discipline · ANSI']) {
  test(`${name} generates a case with imported stabilizers`, async ({ page }) => {
    test.setTimeout(120_000);
    await page.goto('/');
    await page.getByRole('button', { name: 'Project', exact: true }).click();
    await page.getByRole('button', { name: `Start ${name}`, exact: true }).click();
    await expect(page.locator('.wb-project-name')).toHaveText(name);
    await navigateWorkspace(page, 'Case');
    await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 90_000 });
    await expect(page.getByText(/Generated CAD solids/)).toBeVisible();
    await expect(page.getByText('Selected mechanical profiles have incompatible plate-to-PCB engagement distances.', { exact: false })).toHaveCount(0);
  });
}

test('REVIUNG41 generates a gasket case without support collisions', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Start REVIUNG41', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('REVIUNG41');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await page.getByRole('combobox', { name: 'Mount style', exact: true }).selectOption('gasket');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 90_000 });
  await expect(page.getByText(/Generated CAD solids/)).toBeVisible();
  await expect(page.getByText('Export worker failed and restarted', { exact: true })).toHaveCount(0);
});

test('Sofle automatically receives six closure screws per half', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Start Sofle v2', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await page.getByRole('combobox', { name: 'Mount style', exact: true }).selectOption('gasket');
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 90_000 });
  await expect(page.locator('summary').filter({ hasText: 'Suggested mount locations' })).toContainText('6 candidates');
  await page.getByRole('treeitem', { name: 'Right case assembly', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Export geometry', exact: true })).toBeEnabled({ timeout: 90_000 });
  await expect(page.locator('summary').filter({ hasText: 'Suggested mount locations' })).toContainText('6 candidates');
});
