import { expect, test, type Page } from '@playwright/test';

async function storedCounts(page: Page) {
  return page.evaluate(() => new Promise<number[]>((resolve, reject) => {
    const open = indexedDB.open('boardstudio-v2', 1);
    open.onerror = () => reject(open.error);
    open.onsuccess = () => {
      const db = open.result;
      const tx = db.transaction(['projects', 'assets']);
      const projects = tx.objectStore('projects').count();
      const assets = tx.objectStore('assets').count();
      tx.oncomplete = () => { db.close(); resolve([projects.result, assets.result]); };
    };
  }));
}

test('reset requires confirmation, clears local projects and assets, and starts the guide', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByLabel('Saved locally', { exact: true })).toBeVisible();
  await page.evaluate(() => new Promise<void>(resolve => {
    localStorage.setItem('boardstudio:v2:setup-guide:old-test', '{"open":false}');
    localStorage.setItem('unrelated-test-setting', 'keep');
    const open = indexedDB.open('boardstudio-v2', 1);
    open.onsuccess = () => {
      const db = open.result;
      const tx = db.transaction(['projects', 'assets'], 'readwrite');
      tx.objectStore('projects').put({ id: 'old-test' });
      tx.objectStore('assets').put(new Uint8Array([1, 2]), 'test-asset');
      tx.oncomplete = () => { db.close(); resolve(); };
    };
  }));
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'Workspace settings', exact: true }).click();
  page.once('dialog', dialog => dialog.dismiss());
  await page.getByRole('button', { name: 'Reset local projects…', exact: true }).click();
  expect(await storedCounts(page)).toEqual([2, 1]);
  page.once('dialog', async dialog => {
    expect(dialog.message()).toContain('cannot be undone');
    await dialog.accept();
  });
  await page.getByRole('button', { name: 'Reset local projects…', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Project setup guide' })).toBeVisible();
  expect(await storedCounts(page)).toEqual([1, 0]);
  expect(await page.evaluate(() => localStorage.getItem('boardstudio:v2:setup-guide:old-test'))).toBeNull();
  expect(await page.evaluate(() => localStorage.getItem('unrelated-test-setting'))).toBe('keep');
  await page.reload();
  await expect(page.getByRole('region', { name: 'Project setup guide' })).toBeVisible();
  await expect(page.getByRole('treeitem', { name: /Matrix 1/ })).toHaveCount(0);
  expect(await storedCounts(page)).toEqual([1, 0]);
});
