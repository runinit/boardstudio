import { expect, test } from '@playwright/test';
import measurements from '../src/demos/keyboard-layouts.json' with { type: 'json' };

for (const [id, layout] of Object.entries(measurements)) {
  test(`${layout.name} opens as editable parts and persists`, async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Project', exact: true }).click();
    await page.getByRole('button', { name: `Start ${layout.name}`, exact: true }).click();
    const count = layout.keys.length * 2 + layout.keys.filter(key => key.width >= 2).length + (layout.split ? 3 : 2);
    await expect(page.locator('.wb-scene-part')).toHaveCount(count);
    await expect(page.getByRole('button', { name: 'Project', exact: true })).toHaveAttribute('title', `Project menu — ${layout.name}`);
    await expect(page.getByRole('alert')).toHaveCount(0);
    await expect(page.getByLabel('Saved locally', { exact: true })).toBeVisible();
    await page.screenshot({ animations: 'disabled', path: `test-results/demo-${id}.png` });
    if (layout.split) {
      await page.getByRole('combobox', { name: 'Selected board', exact: true }).selectOption('right');
      await expect(page.locator('.wb-scene-part')).toHaveCount(count);
    }
    await page.reload();
    await expect(page.locator('.wb-scene-part')).toHaveCount(count);
    await expect(page.getByRole('button', { name: 'Project', exact: true })).toHaveAttribute('title', `Project menu — ${layout.name}`);
  });
}
