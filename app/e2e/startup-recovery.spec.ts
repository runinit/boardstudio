import { expect, test } from '@playwright/test';

test('an incompatible saved project offers recovery without overwriting it', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Project', exact: true })).toBeVisible();
  await expect(page.getByLabel('Saved locally', { exact: true })).toBeVisible();
  await page.evaluate(() => new Promise<void>((resolve, reject) => {
    const open = indexedDB.open('boardstudio-v2', 1);
    open.onerror = () => reject(open.error);
    open.onsuccess = () => {
      const db = open.result;
      const transaction = db.transaction('projects', 'readwrite');
      const store = transaction.objectStore('projects');
      const read = store.get('starter');
      read.onsuccess = () => { const project = read.result; project.matrices[0].diodes = { enabled: true }; store.put(project); };
      transaction.oncomplete = () => { db.close(); resolve(); };
      transaction.onerror = () => { db.close(); reject(transaction.error); };
    };
  }));
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Could not open the saved project' })).toBeVisible();
  await page.getByText('Error details', { exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('diodes');
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await expect(page.getByRole('button', { name: 'Open Sofle v2 demo', exact: true })).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByRole('button', { name: 'Open Sofle v2 demo', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Project', exact: true })).toBeVisible();
  const preserved = await page.evaluate(() => new Promise<boolean>((resolve) => {
    const open = indexedDB.open('boardstudio-v2', 1);
    open.onsuccess = () => { const db = open.result; const read = db.transaction('projects').objectStore('projects').get('starter'); read.onsuccess = () => { db.close(); resolve(read.result.matrices[0].diodes.enabled); }; };
  }));
  expect(preserved).toBe(true);
  await page.reload();
  await expect(page.getByRole('button', { name: 'Project', exact: true })).toBeVisible();
});
