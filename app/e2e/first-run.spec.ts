import { expect, test } from '@playwright/test';

test.use({ storageState: { cookies: [], origins: [] } });

test('first run offers setup or a demo without creating a keyboard', async ({ page }) => {
  await page.goto('/');
  const menu = page.getByLabel('Project menu', { exact: true });
  await expect(menu).toBeVisible();
  const keyboards = menu.getByRole('region', { name: 'Your keyboards', exact: true });
  await expect(keyboards.getByRole('heading', { name: 'Your keyboards 0', exact: true })).toBeVisible();
  await expect(keyboards.getByRole('button', { name: 'Create new keyboard', exact: true })).toBeFocused();
  await expect(menu.getByRole('textbox', { name: 'Project name', exact: true })).toHaveCount(0);
  await expect(keyboards.getByRole('button', { name: /^Open / })).toHaveCount(0);
  await page.reload();
  await expect(keyboards.getByRole('heading', { name: 'Your keyboards 0', exact: true })).toBeVisible();
  await expect(menu.getByRole('button', { name: 'Start Sofle v2', exact: true })).toBeVisible();
});

test('the plus card creates a keyboard and opens setup, then reload resumes it', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('region', { name: 'Your keyboards', exact: true }).getByRole('button', { name: 'Create new keyboard', exact: true }).click();
  const guide = page.getByRole('region', { name: 'Project setup guide', exact: true });
  await expect(guide).toBeVisible();
  await expect(guide.getByRole('heading', { name: 'Setup guide', exact: true })).toBeFocused();
  await guide.getByRole('button', { name: 'Split keyboard', exact: true }).click();
  await expect(guide.getByRole('button', { name: 'Split keyboard', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await page.reload();
  await expect(guide.getByRole('button', { name: 'Split keyboard', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  const keyboards = page.getByRole('region', { name: 'Your keyboards', exact: true });
  await expect(keyboards.getByRole('heading', { name: 'Your keyboards 1', exact: true })).toBeVisible();
  await expect(keyboards.getByRole('button', { name: 'Open Untitled keyboard', exact: true })).toBeVisible();
  await expect(keyboards.getByRole('button', { name: 'Open Starter keyboard', exact: true })).toHaveCount(0);
  await keyboards.getByRole('button', { name: 'Create new keyboard', exact: true }).click();
  await expect(guide.getByRole('button', { name: 'Split keyboard', exact: true })).toHaveAttribute('aria-pressed', 'false');
  await guide.getByRole('button', { name: 'One keyboard', exact: true }).click();
  await expect(guide.getByRole('button', { name: 'One keyboard', exact: true })).toHaveAttribute('aria-pressed', 'true');
});

test('cloning a demo creates only the chosen keyboard and preserves editable copies', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Start Sofle v2', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await expect(page.getByRole('region', { name: 'Project setup guide', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  const keyboards = page.getByRole('region', { name: 'Your keyboards', exact: true });
  await expect(keyboards.getByRole('heading', { name: 'Your keyboards 1', exact: true })).toBeVisible();
  await expect(keyboards.getByRole('button', { name: /^Open / })).toHaveCount(1);
  await page.getByRole('textbox', { name: 'Project name', exact: true }).fill('My Sofle');
  await page.getByRole('textbox', { name: 'Project name', exact: true }).press('Enter');
  await expect(page.locator('.wb-project-name')).toHaveText('My Sofle');
  await page.getByRole('button', { name: 'Start Sofle v2', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('Sofle v2');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(keyboards.getByRole('heading', { name: 'Your keyboards 2', exact: true })).toBeVisible();
  await keyboards.getByRole('button', { name: 'Open My Sofle', exact: true }).click();
  await expect(page.locator('.wb-project-name')).toHaveText('My Sofle');
  await page.reload();
  await expect(page.locator('.wb-project-name')).toHaveText('My Sofle');
  await expect(page.getByLabel('Project menu', { exact: true })).toHaveCount(0);
});

test('a storage failure leaves first-run creation available without entering setup', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Create new keyboard', exact: true })).toBeVisible();
  await page.evaluate(() => {
    IDBObjectStore.prototype.put = function () { throw new DOMException('Storage is full', 'QuotaExceededError'); };
  });
  await page.getByRole('button', { name: 'Create new keyboard', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Storage is full');
  await expect(page.getByRole('region', { name: 'Project setup guide', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Create new keyboard', exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Your keyboards 0', exact: true })).toBeVisible();
});

test('first-run choices fit a narrow screen and keyboard navigation reaches demos', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  const create = page.getByRole('button', { name: 'Create new keyboard', exact: true });
  await expect(create).toBeFocused();
  await expect(create).toBeInViewport();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('button', { name: 'Start Sofle v2', exact: true })).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.keyboard.press('Shift+Tab');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('region', { name: 'Project setup guide', exact: true })).toBeVisible();
});
