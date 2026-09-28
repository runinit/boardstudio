import { expect, test } from '@playwright/test';

test('saved keyboards remain discoverable and reopen after creating another project', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Your keyboards', exact: true })).toBeVisible();
  await page.getByRole('textbox', { name: 'Project name', exact: true }).fill('My saved keyboard');
  await page.getByRole('textbox', { name: 'Project name', exact: true }).press('Enter');
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Untitled keyboard');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Open My saved keyboard', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('My saved keyboard');
  await page.reload();
  await expect(page.locator('.wb-project-name')).toHaveText('My saved keyboard');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Open Untitled keyboard', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open My saved keyboard', exact: true }).locator('svg rect')).not.toHaveCount(0);
});

test('demo gallery opens editable copies and keeps earlier work', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Start Sofle v2', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('textbox', { name: 'Project name', exact: true }).fill('My Sofle');
  await page.getByRole('textbox', { name: 'Project name', exact: true }).press('Enter');
  await page.getByRole('button', { name: 'Start Sofle v2', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Open My Sofle', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('My Sofle');
});

test('keyboard browser fits small screens and always offers a way back', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  const library = page.locator('#wb-project-dropdown');
  const bounds = (await library.boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(320);
  expect(bounds.y + bounds.height).toBeLessThanOrEqual(640);
  await page.getByRole('searchbox', { name: 'Search saved keyboards' }).fill('no matching keyboard');
  await expect(page.getByText('No keyboards match your search.')).toBeVisible();
  await page.getByRole('button', { name: 'Clear search', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Workspace settings', exact: true })).toBeInViewport();
  await page.getByRole('button', { name: 'Workspace settings', exact: true }).click();
  await page.getByRole('button', { name: 'Back to project menu', exact: true }).click();
  await page.getByRole('button', { name: 'Close project menu', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Project', exact: true })).toBeFocused();
});
