import { navigateWorkspace } from './workspace-navigation';
import { expect, test } from '@playwright/test';

test('keycap layers and PCB visibility are view-only and preserve selection', async ({ page }) => {
  await page.goto('/');
  const layers = page.getByRole('region', { name: 'Canvas layers' });
  await expect(page.locator('.wb-keycap-overlay').first()).toBeVisible();
  await layers.getByRole('button', { name: 'Layers', exact: true }).click();
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await layers.getByRole('button', { name: 'Hide Keycaps', exact: true }).click();
  await expect(page.locator('.wb-keycap-overlay')).toHaveCount(0);
  await layers.getByRole('button', { name: 'Show Keycaps', exact: true }).click();
  await expect(page.locator('.wb-keycap-overlay').first()).toBeVisible();
  await navigateWorkspace(page, 'PCB');
  await expect(page.locator('.wb-keycap-overlay')).toHaveCount(0);
  await expect(page.locator('.wb-scene-footprint .wb-part-pad').first()).toBeVisible();
  await layers.getByRole('button', { name: 'Hide Pads', exact: true }).click();
  await expect(page.locator('.wb-scene-footprint .wb-part-pad')).toHaveCount(0);
  await layers.getByRole('button', { name: 'Show Pads', exact: true }).click();
  await expect(page.locator('.wb-scene-footprint .wb-part-pad').first()).toBeVisible();
  await layers.getByRole('button', { name: 'Hide Edge.Cuts', exact: true }).click();
  await expect(page.locator('.wb-outline-shape')).toHaveCount(0);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
});

test('command details stay attached and usable at desktop and narrow widths', async ({ page }) => {
  await page.goto('/');
  for (const width of [1440, 780]) {
    await page.setViewportSize({ width, height: 900 });
    await page.getByRole('button', { name: 'Snap', exact: true }).click();
    const panel = page.getByRole('dialog', { name: 'Snap', exact: true });
    await expect(panel).toBeVisible();
    expect(await panel.evaluate((element) => Boolean(element.closest('.wb-canvas-toolbar')))).toBe(true);
    await panel.getByRole('combobox', { name: 'Snap increment' }).selectOption('0.5');
    const bounds = await panel.boundingBox();
    expect(bounds!.x).toBeGreaterThanOrEqual(0);
    expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(width);
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Snap', exact: true })).toBeFocused();
  }
});
