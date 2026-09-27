import { navigateWorkspace, openWorkspaceSettings } from './workspace-navigation';
import { expect, test } from '@playwright/test';

test('outline settings have a way back that preserves the selected column', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: /^Column 2 / }).click();
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Board outline…', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Board outline', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Back to selection', exact: true }).click();
  await expect(page.getByRole('spinbutton', { name: 'Splay °', exact: true })).toBeVisible();
  await expect(page.getByRole('treeitem', { name: /^Column 2 / })).toHaveAttribute('aria-selected', 'true');
});

test('view navigation remains available when the object panel is collapsed', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await page.getByRole('button', { name: 'Collapse objects', exact: true }).click();
  await navigateWorkspace(page, 'PCB');
  await expect(page.getByRole('region', { name: 'PCB canvas', exact: true })).toBeVisible();
  await navigateWorkspace(page, 'Layout');
  await expect(page.getByRole('region', { name: 'Design canvas', exact: true })).toBeVisible();
});

test('export has a dedicated page and returns to the originating view', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'PCB');
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Export workspace' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Fit board' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Back to PCB', exact: true }).click();
  await expect(page.getByRole('tab', { name: 'PCB', exact: true })).toHaveAttribute('aria-selected', 'true');
});

test('export remains usable when a selected object drawer becomes compact', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: /^Column 2 / }).click();
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole('button', { name: 'Close panels' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Review wiring' }).click();
  await expect(page.getByRole('combobox', { name: 'Workspace', exact: true })).toHaveValue('PCB');
});

test('workspace preferences restore hidden panels without changing the project', async ({ page }) => {
  await page.goto('/');
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await page.getByRole('button', { name: 'Inspector options', exact: true }).click();
  await page.getByRole('button', { name: 'Collapse inspector', exact: true }).click();
  await openWorkspaceSettings(page);
  await page.getByRole('button', { name: 'Restore default panels', exact: true }).click();
  await expect(page.locator('#wb-inspector')).toHaveAttribute('aria-hidden', 'false');
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
});

test('3D preview does not offer hidden 2D editing tools', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '3D assembly', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Transform', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: '2D', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Transform', exact: true })).toBeVisible();
});

test('compact panels and command popovers have visible close controls', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Inspect', exact: true }).click();
  await page.getByRole('button', { name: 'Close inspector', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Inspect', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Snap', exact: true }).click();
  await page.getByRole('button', { name: 'Close Snap', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Snap', exact: true })).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('new project leaves Parts and cancelling guided creation returns to the guide', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  const guide = page.getByRole('region', { name: 'Project setup guide' });
  await expect(guide).toBeVisible();
  await guide.getByRole('button', { name: 'Continue to layout' }).click();
  await guide.getByRole('button', { name: 'Add key matrix' }).click();
  await page.getByRole('form', { name: 'New matrix' }).getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(guide).toBeVisible();
  await expect(guide.getByRole('button', { name: /Layout & assemblies/ })).toHaveAttribute('aria-current', 'step');
});
