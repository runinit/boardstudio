import { openSetupGuide } from './workspace-navigation';
import { expect, test, type Page } from '@playwright/test';

async function projectAction(page: Page, name: string) {
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name, exact: true }).click();
}

async function saveCopy(page: Page) {
  const downloading = page.waitForEvent('download');
  await projectAction(page, 'Save project copy…');
  const path = await (await downloading).path();
  if (!path) throw new Error('Project archive missing');
  return path;
}

async function openCopy(page: Page, path: string) {
  const choosing = page.waitForEvent('filechooser');
  await projectAction(page, 'Open project…');
  await (await choosing).setFiles(path);
  await expect(page.locator('.wb-save-state summary')).toHaveAccessibleName('Saved locally');
}

test('setup starts after a new project and stays skipped across project switches and reload', async ({ page }) => {
  await page.goto('/');
  const guide = page.getByRole('region', { name: 'Project setup guide' });
  await expect(guide).toHaveCount(0);
  const original = await saveCopy(page);
  await projectAction(page, 'New project');
  await expect(guide).toBeVisible();
  await expect(guide.getByRole('heading', { name: 'Setup guide' })).toBeFocused();
  await expect(guide.getByRole('button', { name: /Project & hardware/ })).toHaveAttribute('aria-current', 'step');
  const created = await saveCopy(page);
  await guide.getByRole('button', { name: 'Back to objects' }).click();
  await expect(guide).toHaveCount(0);
  await openCopy(page, original);
  await expect(page.getByRole('treeitem', { name: 'Matrix 1 15 keys', exact: true })).toBeVisible();
  await expect(guide).toHaveCount(0);
  await openCopy(page, created);
  await expect(page.getByRole('treeitem', { name: /Matrix 1/ })).toHaveCount(0);
  await expect(guide).toHaveCount(0);
  await page.reload();
  await expect(page.getByRole('button', { name: 'Project', exact: true })).toBeVisible();
  await expect(guide).toHaveCount(0);
  await openSetupGuide(page);
  await expect(guide).toBeVisible();
  await guide.getByRole('button', { name: /Review & export/ }).click();
  await expect(guide.getByRole('button', { name: /Review & export/ })).toHaveAttribute('aria-current', 'step');
  await guide.getByRole('button', { name: 'Open export options' }).click();
  await expect(page.getByRole('button', { name: 'KiCad board', exact: true })).toHaveCount(0);
  const boardExport = page.locator('.wb-export-row').filter({ hasText: /^KiCad board/ });
  await expect(boardExport.getByRole('button')).toBeDisabled();
  await page.reload();
  await expect(guide.getByRole('button', { name: /Review & export/ })).toHaveAttribute('aria-current', 'step');
});

test('compact guide uses the existing drawer and opens real layout and wiring controls', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await projectAction(page, 'New project');
  const guide = page.getByRole('region', { name: 'Project setup guide' });
  await expect(guide).toBeVisible();
  await expect(guide.getByRole('heading', { name: 'Setup guide' })).toBeFocused();
  await guide.getByRole('button', { name: 'One keyboard', exact: true }).click();
  await expect(guide.getByRole('button', { name: /Project & hardware/ })).toHaveClass(/is-ready/);
  await guide.getByRole('button', { name: 'Continue to layout' }).click();
  await guide.getByRole('button', { name: 'Add key matrix' }).click();
  await expect(page.getByRole('form', { name: 'New matrix' })).toBeVisible();
  await page.getByRole('form', { name: 'New matrix' }).getByRole('button', { name: 'Cancel', exact: true }).click();
  await openSetupGuide(page);
  await guide.getByRole('button', { name: /Controller & wiring/ }).click();
  await guide.getByRole('button', { name: 'Review controller & wiring' }).click();
  await expect(page.locator('#wb-inspector')).toHaveAttribute('aria-hidden', 'false');
  await expect(page.locator('#wb-inventory')).toHaveAttribute('aria-hidden', 'true');
  await expect(page.locator('#wb-inspector .wb-inspector-content :focus')).toHaveCount(1);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Inspect', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Objects', exact: true }).click();
  await expect(guide).toBeVisible();
  for (const button of await guide.getByRole('button').all()) {
    const box = await button.boundingBox();
    expect(box?.height).toBeGreaterThanOrEqual(44);
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('a failed new-project save never opens the guide', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-save-state summary')).toHaveAccessibleName('Saved locally');
  await page.evaluate(() => {
    IDBObjectStore.prototype.put = function () { throw new DOMException('Storage is full', 'QuotaExceededError'); };
  });
  await projectAction(page, 'New project');
  await expect(page.getByRole('alert')).toContainText('Storage is full');
  await expect(page.getByRole('region', { name: 'Project setup guide' })).toHaveCount(0);
  await expect(page.getByRole('treeitem', { name: 'Matrix 1 15 keys', exact: true })).toBeVisible();
});
