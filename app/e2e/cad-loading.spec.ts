import { navigateWorkspace } from './workspace-navigation';
import { expect, test } from '@playwright/test';

test('CAD kernel loads its glue and builds a case preview', async ({ page }) => {
  test.setTimeout(60_000);
  const failed: string[] = [];
  page.on('response', (response) => {
    if (response.status() >= 400 && /cadrum|\.wasm(?:$|\?)/i.test(response.url())) failed.push(response.url());
  });
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Case');
  await page.getByRole('button', { name: 'Update preview', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Case generation' }).getByRole('status')).toHaveText('Preview current', { timeout: 45_000 });
  await expect(page.getByRole('alert')).toHaveCount(0);
  expect(failed).toEqual([]);
});
