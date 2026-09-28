import { expect, test } from '@playwright/test';

test('navigation stays in one header row from phone to wide desktop', async ({ page }) => {
  await page.goto('/');
  for (const width of [320, 390, 768, 1024, 1680]) {
    await page.setViewportSize({ width, height: 844 });
    const header = page.locator('.wb-topbar');
    await expect.poll(async () => (await page.locator('.wb-workspace').boundingBox())?.y).toBeLessThanOrEqual(44);
    expect((await header.boundingBox())!.height).toBeLessThanOrEqual(44);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    await expect(header.getByRole('button', { name: 'Project', exact: true })).toBeVisible();
    await expect(header.getByRole('button', { name: 'Export', exact: true })).toBeVisible();
    let right = 0;
    for (const control of await header.locator('button:visible, select:visible, summary:visible').all()) {
      const bounds = (await control.boundingBox())!;
      expect(bounds.x).toBeGreaterThanOrEqual(right - 1);
      right = bounds.x + bounds.width;
      expect(right).toBeLessThanOrEqual(width);
    }
    if (width <= 820) {
      await header.getByRole('combobox', { name: 'Workspace', exact: true }).selectOption('PCB');
      await expect(page.getByRole('region', { name: 'PCB canvas', exact: true })).toBeVisible();
      await header.getByRole('combobox', { name: 'Workspace', exact: true }).selectOption('Design');
      for (const command of await page.getByRole('toolbar', { name: 'Layout commands' }).getByRole('button').all()) {
        const bounds = (await command.boundingBox())!;
        expect(bounds.x + bounds.width).toBeLessThanOrEqual(width);
      }
    } else {
      await expect(header.getByRole('tab', { name: 'Layout', exact: true })).toBeVisible();
      await expect(header.getByRole('tab', { name: 'Parts', exact: true })).toBeVisible();
    }
  }
});

test('project menu owns setup and workspace preferences with a visible return', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Workspace settings', exact: true }).click();
  await page.getByRole('combobox', { name: 'Color theme' }).selectOption('light');
  await page.getByRole('button', { name: 'Back to project menu', exact: true }).click();
  await page.getByRole('button', { name: 'Setup guide', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Project setup guide' })).toBeVisible();
  await expect(page.locator('#wb-project-dropdown')).toHaveCount(0);
});

test('history is beside the canvas and compact project menu keeps it reachable', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-canvas-footer').getByRole('button', { name: 'Undo', exact: true })).toBeVisible();
  await expect(page.locator('.wb-topbar').getByRole('button', { name: 'Undo', exact: true })).toHaveCount(0);
  await page.setViewportSize({ width: 320, height: 844 });
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(page.locator('#wb-project-dropdown').getByRole('button', { name: 'Undo', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close project menu', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Project', exact: true })).toBeFocused();
});
