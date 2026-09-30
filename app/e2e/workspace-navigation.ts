import { expect, type Page } from '@playwright/test';

export async function navigateWorkspace(page: Page, view: 'Layout' | 'PCB' | 'Keymap' | 'Keycaps' | 'Case' | 'Parts') {
  await expect(page.locator('.wb-root')).toBeVisible();
  const selector = page.getByRole('combobox', { name: 'Workspace', exact: true });
  if (await selector.isVisible()) {
    await selector.selectOption(view === 'Layout' ? 'Design' : view === 'Parts' ? 'Library' : view);
  } else {
    await page.getByRole('tab', { name: view, exact: true }).click();
  }
}

export async function openWorkspaceSettings(page: Page) {
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Workspace settings', exact: true }).click();
}

export async function openSetupGuide(page: Page) {
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Setup guide', exact: true }).click();
}
