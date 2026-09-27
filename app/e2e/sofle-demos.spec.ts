import { expect, test } from '@playwright/test';

for (const [variant, name, parts] of [['v2', 'Sofle v2', 70], ['rgb', 'Sofle RGB', 106], ['choc', 'Sofle Choc', 99]] as const) {
  test(`${name} opens from the project menu and survives reload`, async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Project', exact: true }).click();
    await page.getByRole('combobox', { name: 'Open demo keyboard' }).selectOption(variant);
    await expect(page.locator('.wb-scene-part')).toHaveCount(parts);
    await expect(page.getByRole('button', { name: 'Project', exact: true })).toHaveAttribute('title', `Project menu — ${name}`);
    await expect(page.getByRole('alert')).toHaveCount(0);
    await expect(page.getByLabel('Saved locally', { exact: true })).toBeVisible();
    await page.screenshot({ animations: 'disabled', path: `test-results/sofle-${variant}-desktop.png` });
    await page.getByRole('combobox', { name: 'Selected board', exact: true }).selectOption('right');
    await expect(page.locator('.wb-scene-part')).toHaveCount(parts);
    await expect(page.getByRole('textbox', { name: 'Board name' })).toHaveValue('Right PCB');
    await page.reload();
    await expect(page.locator('.wb-scene-part')).toHaveCount(parts);
    await expect(page.getByRole('button', { name: 'Project', exact: true })).toHaveAttribute('title', `Project menu — ${name}`);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByRole('button', { name: 'Project', exact: true }).click();
    await expect(page.getByRole('combobox', { name: 'Open demo keyboard' })).toBeInViewport();
    await page.screenshot({ animations: 'disabled', path: `test-results/sofle-${variant}-mobile.png` });
  });
}
