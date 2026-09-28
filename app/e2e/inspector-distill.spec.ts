import { navigateWorkspace } from './workspace-navigation';
import { chooseScope } from './selection';
import { expect, test } from '@playwright/test';

test('assembly selection has one placement action and no unrelated footprint settings', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('option', { name: 'MX Hotswap RGB', exact: true }).click();
  const inspector = page.getByRole('complementary', { name: 'Parts inspector' });
  await expect(inspector.getByRole('button', { name: 'Place key assembly' })).toBeVisible();
  await expect(inspector.getByRole('button', { name: 'Place component' })).toHaveCount(0);
  await expect(inspector.getByRole('combobox', { name: 'Switch orientation', exact: true })).toBeVisible();
  await expect(inspector.locator('input, select, textarea')).toHaveCount(1);
  await page.getByRole('button', { name: 'Place key assembly' }).click();
  await page.keyboard.press('Escape');
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
});

test('optional component sections remain accessible by keyboard and preserve committed edits', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /^SW1, MX switch/ }).click();
  await chooseScope(page, 'component');
  await expect(page.getByRole('spinbutton', { name: 'X mm', exact: true })).toBeVisible();
  await expect(page.getByLabel('Use board margin', { exact: true })).toBeHidden();
  const outline = page.locator('summary').filter({ hasText: 'Board outline' });
  await outline.focus();
  await page.keyboard.press('Enter');
  await page.getByLabel('Use board margin', { exact: true }).click();
  await expect(page.getByLabel('Use board margin', { exact: true })).not.toBeChecked();
  const margin = page.getByLabel('Part edge margin', { exact: true });
  await margin.fill('2');
  await margin.press('Enter');
  await outline.click();
  await expect(margin).toBeHidden();
  await outline.click();
  await expect(margin).toHaveValue('2');
  await page.reload();
  await page.getByRole('button', { name: /^SW1, MX switch/ }).click();
  await chooseScope(page, 'component');
  await outline.click();
  await expect(margin).toHaveValue('2');
});

test('advanced generator options keep saved values while the normal inspector stays focused', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('searchbox', { name: 'Search footprints' }).fill('ceoloide/switch_mx');
  await page.getByRole('option', { name: 'MX switch', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'reversible', exact: true })).toBeVisible();
  const trace = page.getByRole('spinbutton', { name: 'Trace Width', exact: true });
  await expect(trace).toBeHidden();
  await page.locator('summary').filter({ hasText: 'Advanced footprint options' }).click();
  await trace.fill('0.47');
  const revision = page.locator('.wb-root');
  const before = await revision.getAttribute('data-revision');
  await page.getByRole('button', { name: 'Apply generator settings' }).click();
  await expect(revision).not.toHaveAttribute('data-revision', before!);
  await page.reload();
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('searchbox', { name: 'Search footprints' }).fill('ceoloide/switch_mx');
  await page.getByRole('option', { name: 'MX switch', exact: true }).click();
  await expect(trace).toBeHidden();
  await page.locator('summary').filter({ hasText: 'Advanced footprint options' }).click();
  await expect(trace).toHaveValue('0.47');
});
