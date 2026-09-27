import { navigateWorkspace } from './workspace-navigation';
import { expect, test } from '@playwright/test';

test('Parts saves standard and custom fits without changing the document on cancel', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  const library = page.getByRole('listbox', { name: 'Footprint library' });
  await library.getByRole('option', { name: 'MX switch', exact: true }).click();
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await page.getByRole('button', { name: 'Define profile', exact: true }).click();
  const editor = page.getByRole('region', { name: 'Mechanical fit profile editor' });
  await expect(editor).toBeVisible();
  await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
  await page.getByRole('button', { name: 'Define profile', exact: true }).click();
  await editor.getByRole('button', { name: 'Use standard cutout', exact: true }).click();
  await expect(editor.getByLabel('Plate cutouts contour 1 vertex 1 X', { exact: true })).toBeVisible();
  await editor.getByRole('button', { name: 'Save fit profile', exact: true }).click();
  await page.getByRole('button', { name: 'Edit profile', exact: true }).click();
  await expect(editor).toContainText('3.50 mm');
  await expect(editor.getByLabel('Plate cutouts contour 1 vertex 1 X', { exact: true })).not.toHaveValue('');
  await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
  await library.getByRole('option', { name: /SK6812 MINI-E/ }).click();
  await page.getByRole('button', { name: 'Define profile', exact: true }).click();
  await editor.getByRole('button', { name: 'Add clearance', exact: true }).click();
  await editor.getByLabel('Component clearances contour 1 vertex 1 X', { exact: true }).fill('-4');
  await editor.getByRole('button', { name: 'Save fit profile', exact: true }).click();
  await page.getByRole('button', { name: 'Edit profile', exact: true }).click();
  await expect(editor.getByLabel('Component clearances contour 1 vertex 1 X', { exact: true })).toHaveValue('-4');
});

test('preview layers and part visibility change only the drawing', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('listbox', { name: 'Footprint library' }).getByRole('option', { name: /SK6812 MINI-E/ }).click();

  const preview = page.getByRole('img', { name: 'Footprint preview' });
  const layers = page.getByRole('region', { name: 'Canvas layers' });
  await layers.getByRole('button', { name: 'Layers', exact: true }).click();
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  const padCount = await preview.locator('.wb-preview-pad').count();
  expect(padCount).toBeGreaterThan(0);

  await layers.getByRole('button', { name: 'Hide Pad numbers' }).click();
  await expect(preview.locator('.wb-preview-pad-label')).toHaveCount(0);
  await layers.getByRole('button', { name: 'Show Pad numbers' }).click();
  await expect(preview.locator('.wb-preview-pad-label')).toHaveCount(padCount);

  await layers.getByRole('button', { name: /Hide part SK6812 MINI-E/ }).click();
  await expect(preview.locator('.wb-preview-pad')).toHaveCount(0);
  await layers.getByRole('button', { name: /Show part SK6812 MINI-E/ }).click();
  await expect(preview.locator('.wb-preview-pad')).toHaveCount(padCount);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
});

test('named footprint graphics layers can be hidden independently', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('listbox', { name: 'Footprint library' }).getByRole('option', { name: /Choc V1 \/ V2 switch/ }).click();

  const layers = page.getByRole('region', { name: 'Canvas layers' });
  await layers.getByRole('button', { name: 'Layers', exact: true }).click();
  const preview = page.getByRole('img', { name: 'Footprint preview' });
  const layer = await preview.locator('.wb-ergogen-drawing [data-layer]').first().getAttribute('data-layer');
  expect(layer).toBeTruthy();
  const graphics = preview.locator(`[data-layer="${layer}"]`);
  const before = await graphics.count();
  expect(before).toBeGreaterThan(0);
  const pads = preview.locator('.wb-preview-pad');
  const padCount = await pads.count();
  await layers.getByRole('button', { name: 'Hide B.Cu' }).click();
  await expect(pads).toHaveCount(0);
  await expect(graphics).toHaveCount(before);
  await layers.getByRole('button', { name: 'Show B.Cu' }).click();
  await expect(pads).toHaveCount(padCount);
  await layers.getByRole('button', { name: `Hide ${layer}` }).click();
  await expect(graphics).toHaveCount(0);
  await layers.getByRole('button', { name: `Show ${layer}` }).click();
  await expect(graphics).toHaveCount(before);
});

test('assembly companions can be hidden without hiding the switch', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('button', { name: 'Objects', exact: true }).click();
  await page.getByRole('listbox', { name: 'Key assemblies' }).getByRole('option', { name: 'MX RGB', exact: true }).click();
  await page.getByRole('button', { name: 'Inspect', exact: true }).click();

  const preview = page.getByRole('img', { name: 'Footprint preview' });
  const layers = page.getByRole('region', { name: 'Canvas layers' });
  await layers.getByRole('button', { name: 'Layers', exact: true }).click();
  const before = await preview.locator('.wb-preview-pad').count();
  await layers.getByRole('button', { name: 'Hide part SK6812 MINI-E' }).click();
  const remaining = await preview.locator('.wb-preview-pad').count();
  expect(remaining).toBeGreaterThan(0);
  expect(remaining).toBeLessThan(before);
  await expect(layers.getByRole('button', { name: 'Show part SK6812 MINI-E' })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('shows live generated footprint geometry in the library workspace', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('listbox', { name: 'Footprint library' }).getByRole('option', { name: /SK6812 MINI-E/ }).click();

  const workspace = page.getByRole('region', { name: 'Parts canvas' });
  const preview = workspace.getByRole('img', { name: 'Footprint preview' });
  await expect(preview).toBeVisible();
  await expect(workspace.locator('.wb-canvas')).toBeHidden();
  await expect(preview.locator('.wb-preview-pad')).toHaveCount(4);
  await page.getByRole('checkbox', { name: 'reversible', exact: true }).check();
  await expect(preview.locator('.wb-preview-pad')).toHaveCount(8);
  await page.getByRole('checkbox', { name: 'reversible', exact: true }).uncheck();
  await expect(preview.locator('.wb-preview-pad')).toHaveCount(4);
});

test('standard parts use their footprint defaults without pad tuning', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  const preview = page.getByRole('region', { name: 'Parts canvas' }).getByRole('img', { name: 'Footprint preview' });
  const pad = preview.locator('rect').first();
  const initialWidth = await pad.getAttribute('width');
  await expect(page.getByRole('spinbutton', { name: /Pad (size|spacing|drill)/ })).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'Custom component definition editor' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: '+ New script' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Place component', exact: true })).toBeVisible();
  await navigateWorkspace(page, 'Layout');
  await navigateWorkspace(page, 'Parts');
  await expect(preview.locator('rect').first()).toHaveAttribute('width', initialWidth!);
});

test('RGB assembly companion retains saved model settings and uses the preset mounting configuration', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('listbox', { name: 'Footprint library' }).getByRole('option', { name: 'SK6812 MINI-E', exact: true }).click();
  await page.locator('summary').filter({ hasText: '3D model placement' }).click();
  await page.getByRole('textbox', { name: 'led_3dmodel_xyz_offset', exact: true }).fill('[0, 0, 1]');
  await page.getByRole('button', { name: 'Apply generator settings' }).click();
  await page.getByRole('listbox', { name: 'Key assemblies' }).getByRole('option', { name: 'MX RGB', exact: true }).click();
  const preview = page.getByRole('img', { name: 'Footprint preview' });
  const led = preview.locator('[data-part-source="ceoloide/led_sk6812mini-e"]');
  await expect(led.locator('.wb-preview-pad')).toHaveCount(4);
  await expect(led.locator('.wb-preview-copper')).toHaveCount(0);
  await page.getByRole('listbox', { name: 'Footprint library' }).getByRole('option', { name: 'SK6812 MINI-E', exact: true }).click();
  await page.locator('summary').filter({ hasText: '3D model placement' }).click();
  expect(JSON.parse(await page.getByRole('textbox', { name: 'led_3dmodel_xyz_offset', exact: true }).inputValue())).toEqual([0, 0, 1]);
});
