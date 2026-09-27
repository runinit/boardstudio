import type { Page } from '@playwright/test';

export async function configureMatrix(page: Page, rows = 6, columns = 5) {
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Matrix…', exact: true }).click();
  await page.getByRole('spinbutton', { name: 'New matrix rows' }).fill(String(rows));
  await page.getByRole('spinbutton', { name: 'New matrix columns' }).fill(String(columns));
  await page.getByRole('button', { name: 'Create matrix' }).click();
}
