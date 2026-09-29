import { navigateWorkspace, openWorkspaceSettings } from './workspace-navigation';
import { expect, test } from '@playwright/test';

test('findings are scoped and mechanical review opens a closed compact inspector', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  const live = page.getByRole('switch', { name: /Live preview/i });
  if (await live.isChecked()) await live.uncheck();
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await expect(page.getByRole('button', { name: /Layout findings: \d+/ })).toBeVisible();
  const mechanical = page.getByRole('button', { name: /Mechanical findings: \d+/ });
  await expect(mechanical).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  const settings = page.getByRole('button', { name: 'Case settings', exact: true });
  if (await settings.getAttribute('aria-expanded') === 'true') await settings.click();
  await mechanical.click();
  const summary = page.locator('.wb-mechanical-panel summary').filter({ hasText: 'Mechanical diagnostics' });
  await expect(summary).toBeFocused();
  await expect(summary.locator('..')).toHaveAttribute('open', '');
  const count = Number((await mechanical.innerText()).match(/\d+/)?.[0]);
  await expect(summary.locator('small')).toHaveText(String(count));
});

test('unavailable case layers retain visibility choices after generation', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  const live = page.getByRole('switch', { name: /Live preview/i });
  if (await live.isChecked()) await live.uncheck();
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  const layers = page.getByRole('region', { name: 'Canvas layers' });
  await layers.getByRole('button', { name: 'Layers', exact: true }).click();
  const plate = layers.getByRole('button', { name: /^(Hide|Show) Plate$/, exact: true });
  await expect(plate).toContainText('Not generated');
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await plate.click();
  await page.getByRole('region', { name: 'Case generation' }).getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Case generation' }).getByRole('button', { name: 'Export geometry' })).toBeEnabled({ timeout: 45000 });
  await expect(plate).not.toContainText('Not generated');
  await expect(plate).toHaveAttribute('aria-pressed', 'false');
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
});

test('compact case settings stay labeled and disclosure choices survive edits', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  const live = page.getByRole('switch', { name: /Live preview/i });
  if (await live.isChecked()) await live.uncheck();
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  await expect(page.locator('.wb-mech-revision').first()).toContainText('Configuration resolved');
  const construction = page.locator('.wb-mechanical-panel summary').filter({ hasText: 'Construction' });
  await construction.click();
  const thickness = page.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true });
  await thickness.fill('2.5'); await thickness.press('Tab');
  await expect(construction.locator('..')).not.toHaveAttribute('open');
  await page.setViewportSize({ width: 390, height: 844 });
  const settings = page.getByRole('button', { name: 'Case settings', exact: true });
  await expect(settings).toBeVisible();
  await expect(settings).toHaveAccessibleName('Case settings');
  if (await settings.getAttribute('aria-expanded') === 'true') await settings.click();
  await settings.click();
  await expect(construction).toBeVisible();
  await expect(construction.locator('..')).not.toHaveAttribute('open');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('case controls reflow in a 200 percent zoom-equivalent viewport in both themes', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/');
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Configure mechanical stack', exact: true }).click();
  for (const theme of ['light', 'dark']) {
    await openWorkspaceSettings(page);
    await page.getByRole('combobox', { name: 'Color theme', exact: true }).selectOption(theme);
    await page.getByRole('button', { name: 'Close project menu', exact: true }).click();
    // Browser zoom halves the CSS viewport; CSS zoom alone does not update media queries.
    await page.setViewportSize({ width: 720, height: 500 });
    const controls = page.getByRole('region', { name: 'Case generation' });
    await expect(controls.getByRole('button', { name: 'Update preview', exact: true })).toBeVisible();
    expect(await controls.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
    await expect(page.getByRole('button', { name: 'Case settings', exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.setViewportSize({ width: 1440, height: 1000 });
  }
});
