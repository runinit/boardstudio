import { navigateWorkspace } from './workspace-navigation';
import { expect, test } from '@playwright/test';

test('case generation actions remain readable at desktop, intermediate and narrow widths', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  const controls = page.getByRole('region', { name: 'Case generation' });
  await expect(controls).toHaveCount(1);
  await expect(controls.getByRole('button', { name: 'Generate', exact: true })).toHaveClass(/wb-primary/);
  await expect(controls.getByRole('button', { name: 'Export geometry' })).toBeDisabled();

  for (const width of [1280, 900, 390]) {
    await page.setViewportSize({ width, height: 844 });
    if (width <= 820 && await page.locator('.wb-mechanical-panel').isVisible()) await page.getByRole('button', { name: 'Case settings', exact: true }).click();
    await expect(controls).toBeVisible();
    for (const name of ['Generate', 'Export geometry']) {
      const button = controls.getByRole('button', { name, exact: true });
      await expect.poll(async () => {
        const bounds = await button.boundingBox();
        return Boolean(bounds && bounds.x >= 0 && bounds.x + bounds.width <= width);
      }).toBe(true);
      const bounds = await button.boundingBox();
      expect(bounds).not.toBeNull();
      expect(bounds!.x).toBeGreaterThanOrEqual(0);
      expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(width);
      expect(bounds!.height).toBeLessThan(50);
    }
    expect(await controls.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
  }
});

test('blocked generation opens and focuses mechanical diagnostics', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  await page.getByRole('button', { name: 'Back to objects', exact: true }).click();
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  const controls = page.getByRole('region', { name: 'Case generation' });
  await expect(controls).toHaveCount(1);
  await controls.getByRole('button', { name: 'Generate', exact: true }).click();
  await expect(controls.getByRole('status')).toContainText('Generation blocked');
  await expect(controls.getByRole('button', { name: 'Export geometry' })).toBeDisabled();
  const summary = page.locator('.wb-mechanical-panel summary').filter({ hasText: 'Mechanical diagnostics' });
  if (await summary.locator('..').getAttribute('open') !== null) await summary.click();
  await page.getByRole('button', { name: /Mechanical findings:/ }).click();
  await expect(summary).toBeFocused();
  await expect(summary.locator('..')).toHaveAttribute('open', '');
});

test('workbenches share layer disclosure and view-only visibility behavior', async ({ page }) => {
  await page.goto('/');
  const layers = page.getByRole('region', { name: 'Canvas layers' });
  for (const mode of ['Layout', 'PCB', 'Case']) {
    await page.getByRole('tab', { name: mode, exact: true }).click();
    await expect(layers).toHaveCount(1);
    if (await layers.getByRole('button', { name: 'Layers', exact: true }).getAttribute('aria-expanded') === 'false') await layers.getByRole('button', { name: 'Layers', exact: true }).click();
    await expect(layers.getByRole('button', { name: 'Layers', exact: true })).toHaveAttribute('aria-expanded', 'true');
    const revision = await page.locator('.wb-root').getAttribute('data-revision');
    const toggle = layers.locator('button[aria-pressed]').first();
    await expect(toggle).toHaveAttribute('aria-pressed', 'true');
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-pressed', 'false');
    await toggle.click();
    await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(layers.getByRole('button', { name: 'Layers', exact: true })).toHaveAttribute('aria-expanded', 'false');
  await layers.getByRole('button', { name: 'Layers', exact: true }).click();
  await expect(layers.locator('button[aria-pressed]').first()).toBeVisible();
});


test('case settings keep generation available without layout relationship tabs', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await expect(page.getByRole('tab', { name: 'Relations', exact: true })).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'Case generation' }).getByRole('button', { name: 'Generate', exact: true })).toBeVisible();
});
