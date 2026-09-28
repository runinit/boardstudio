import { navigateWorkspace } from './workspace-navigation';
import { expect, test } from '@playwright/test';

test('Layers floats above the drawing and remembers visibility when collapsed', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  const layers = page.getByRole('region', { name: 'Canvas layers' });
  for (const mode of ['Layout', 'PCB', 'Case']) {
    await page.getByRole('tab', { name: mode, exact: true }).click();
    await expect(layers).toHaveAttribute('data-compact', 'false');
    const drawing = mode === 'Case' ? page.locator('.wb-assembly-viewport canvas') : page.getByRole('application', { name: /Board layout canvas/ });
    if (await layers.getAttribute('data-open') === 'true') await layers.getByRole('button', { name: 'Layers', exact: true }).click();
    await expect(layers).toHaveAttribute('data-open', 'false');
    await layers.getByRole('button', { name: 'Layers', exact: true }).click();
    const width = (await page.locator('.wb-canvas-stage').boundingBox())!.width;
    const row = layers.locator('button[aria-pressed]').first();
    await row.click();
    await layers.getByRole('button', { name: 'Layers', exact: true }).click();
    await expect.poll(async () => (await page.locator('.wb-canvas-stage').boundingBox())!.width).toBe(width);
    await layers.getByRole('button', { name: 'Layers', exact: true }).click();
    await expect(row).toHaveAttribute('aria-pressed', 'false');
    await row.click();
  }
  await navigateWorkspace(page, 'Parts');
  const preview = page.getByRole('img', { name: 'Footprint preview' });
  if (await layers.getAttribute('data-open') === 'true') await layers.getByRole('button', { name: 'Layers', exact: true }).click();
  await expect(layers).toHaveAttribute('data-open', 'false');
  await expect(preview).toBeVisible();
});

test('Layers uses canvas width and compact dismissal restores focus', async ({ page }) => {
  await page.setViewportSize({ width: 900, height: 844 });
  await page.goto('/');
  const layers = page.getByRole('region', { name: 'Canvas layers' });
  await expect(layers).toHaveAttribute('data-compact', 'true');
  const trigger = layers.getByRole('button', { name: 'Layers', exact: true });
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await trigger.click();
  await layers.locator('button[aria-pressed]').first().focus();
  await page.keyboard.press('Escape');
  await expect(trigger).toBeFocused();
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await trigger.click();
  await layers.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(trigger).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
