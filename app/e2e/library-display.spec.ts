import { navigateWorkspace } from './workspace-navigation';
import { expect, test } from '@playwright/test';

const assemblies = ['MX Solder', 'MX Hotswap', 'Choc V1 Solder', 'Choc V1 Hotswap', 'MX RGB', 'Choc V1 RGB', 'MX Hotswap RGB', 'Choc V1 Hotswap RGB'];

test('all assembly previews show centred keycaps and preserve the project', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  const root = page.locator('.wb-root');
  const revision = await root.getAttribute('data-revision');
  const preview = page.getByRole('img', { name: 'Footprint preview' });
  const choices = page.getByRole('listbox', { name: 'Key assemblies' });
  for (const name of assemblies) {
    await choices.getByRole('option', { name, exact: true }).click();
    await expect(preview.locator('.wb-preview-keycap')).toHaveCount(1);
    await expect(preview.locator(':scope > g')).toHaveCount(name.includes('RGB') ? 3 : 2);
    const outline = await preview.locator('.wb-preview-keycap').getAttribute('points');
    const points = outline!.split(' ').map(pair => pair.split(',').map(Number));
    expect(points.reduce((sum, point) => sum + point[0], 0)).toBeCloseTo(0);
    expect(points.reduce((sum, point) => sum + point[1], 0)).toBeCloseTo(0);
    await expect(root).toHaveAttribute('data-revision', revision!);
  }
  const layers = page.getByRole('region', { name: 'Canvas layers' });
  await layers.getByRole('button', { name: 'Layers', exact: true }).click();
  const viewBox = await preview.getAttribute('viewBox');
  await layers.getByRole('button', { name: 'Hide Keycap', exact: true }).click();
  await expect(preview.locator('.wb-preview-keycap')).toHaveCount(0);
  await expect(preview).toHaveAttribute('viewBox', viewBox!);
  await layers.getByRole('button', { name: 'Show Keycap', exact: true }).click();
  await expect(preview.locator('.wb-preview-keycap')).toHaveCount(1);
  await expect(root).toHaveAttribute('data-revision', revision!);
});

test('assembly 3D view stays in the library and returns to the same 2D preview', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('listbox', { name: 'Key assemblies' }).getByRole('option', { name: 'MX Hotswap RGB', exact: true }).click();
  const preview = page.getByRole('img', { name: 'Footprint preview' });
  const outline = await preview.locator('.wb-preview-keycap').getAttribute('points');
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await page.getByRole('button', { name: '3D model', exact: true }).click();
  await expect(page.getByLabel('3D footprint model preview', { exact: true })).toBeVisible();
  await expect(page.locator('.wb-library-model-workspace canvas')).toBeVisible({ timeout: 45_000 });
  await page.getByRole('button', { name: '2D footprint', exact: true }).click();
  await expect(preview.locator('.wb-preview-keycap')).toHaveAttribute('points', outline!);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
});

for (const theme of ['dark', 'light']) {
  test(`${theme} assembly preview fits desktop and narrow screens`, async ({ page }, testInfo) => {
    await page.addInitScript(theme => localStorage.setItem('boardstudio:v2:theme', theme), theme);
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto('/');
    await navigateWorkspace(page, 'Parts');
    await page.getByRole('listbox', { name: 'Key assemblies' }).getByRole('option', { name: 'Choc V1 Hotswap RGB', exact: true }).click();
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: 900 });
      if (width === 390) {
        const toggle = page.getByRole('button', { name: 'Inspect', exact: true });
        await expect(toggle).toBeVisible();
        if (await toggle.getAttribute('aria-expanded') === 'true') await toggle.click();
        await expect(page.locator('#wb-inspector')).toHaveAttribute('aria-hidden', 'true');
      }
      const preview = page.getByRole('img', { name: 'Footprint preview' });
      await expect(preview).toBeVisible();
      await expect(preview.locator('.wb-preview-keycap')).toHaveCount(1);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      const box = await preview.boundingBox();
      expect(box!.x).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width).toBeLessThanOrEqual(width);
      await page.screenshot({ animations: 'disabled', path: testInfo.outputPath(`assembly-${theme}-${width}.png`) });
    }
  });
}
