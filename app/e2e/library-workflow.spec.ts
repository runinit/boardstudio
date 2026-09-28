import { navigateWorkspace } from './workspace-navigation';
import { zipSync, strToU8 } from 'fflate';
import { demoProject } from '../src/demo';
import { chooseScope } from './selection';
import { expect, test } from '@playwright/test';
import { configureMatrix } from './matrix-setup';
import { buildCase } from '../../cad/src/index';
import { prepareCase } from '../../cad/test/native-prepare.mjs';

test('Parts uses a searchable categorized catalogue and a selected component inspector', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  const catalog = page.getByRole('complementary', { name: 'Part inventory' });
  await expect(catalog.getByRole('tree')).toHaveCount(0);
  await expect(catalog.getByRole('listbox', { name: 'Key assemblies' })).toBeVisible();
  await catalog.getByRole('searchbox').fill('RGB');
  await catalog.getByRole('option', { name: /SK6812 MINI-E/ }).click();
  await expect(page.getByRole('complementary', { name: 'Parts inspector' }).getByRole('heading', { name: 'SK6812 MINI-E', exact: true })).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Definition name' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Place component', exact: true })).toBeVisible();
  await expect(page.locator('.wb-canvas-footer')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Snap', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: '3D model', exact: true }).click();
  await expect(page.getByText('1 / 1 models · 1.6 mm PCB', { exact: true })).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: '2D footprint', exact: true }).click();
  await expect(page.getByRole('img', { name: 'Footprint preview' })).toBeVisible();
  const pad = page.locator('.wb-preview-pad').first();
  expect(await pad.evaluate((node) => getComputedStyle(node).stroke)).toBe('none');
  await catalog.getByRole('searchbox').fill('');
  await catalog.getByRole('option', { name: 'MX Hotswap RGB', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'MX Hotswap RGB', exact: true }).first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Place key assembly' })).toBeVisible();
});

test('matrix creation asks for dimensions and deletion removes its container with undo', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Matrix…' }).click();
  await expect(page.getByRole('spinbutton', { name: 'New matrix rows' })).toHaveValue('');
  await expect(page.getByRole('button', { name: 'Continue to placement' })).toBeDisabled();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await configureMatrix(page, 2, 3);
  await page.getByRole('button', { name: 'Ghost key, row 1, column 1', exact: true }).click();
  await expect(page.getByRole('treeitem', { name: 'Matrix 2 6 keys', exact: true })).toBeVisible();
  const name = page.getByRole('textbox', { name: 'Matrix name' });
  await name.fill('Thumb cluster');
  await name.press('Enter');
  await expect(page.getByRole('treeitem', { name: 'Thumb cluster 6 keys', exact: true })).toBeVisible();
  await page.locator('summary').filter({ hasText: 'Matrix actions' }).click();
  await page.getByRole('button', { name: 'Delete matrix', exact: true }).click();
  await expect(page.getByRole('treeitem', { name: /Thumb cluster/ })).toHaveCount(0);
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.getByRole('treeitem', { name: 'Thumb cluster 6 keys', exact: true })).toBeVisible();
  await page.getByRole('treeitem', { name: 'Thumb cluster 6 keys', exact: true }).click();
  await page.keyboard.press('Delete');
  await expect(page.getByRole('treeitem', { name: /Thumb cluster/ })).toHaveCount(0);
  await page.getByRole('treeitem', { name: /^Main board/ }).click();
  await page.getByRole('textbox', { name: 'Board name' }).fill('Left half');
  await page.getByRole('textbox', { name: 'Board name' }).press('Enter');
  await expect(page.getByRole('heading', { name: 'Left half', exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Left half', exact: true })).toBeVisible();
});

test('row and column selections outline their full scope without selection circles', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 2 3 keys', exact: true }).click();
  const outline = page.locator('.wb-scope-outline.is-selected');
  await expect(outline).toHaveAttribute('data-scope', 'column');
  const column = (await outline.boundingBox())!;
  expect(column.height).toBeGreaterThan(column.width * 2);
  await expect(page.locator('circle.wb-part-selection')).toHaveCount(0);
  await chooseScope(page, 'row');
  await expect(outline).toHaveAttribute('data-scope', 'row');
  const row = (await outline.boundingBox())!;
  expect(row.width).toBeGreaterThan(row.height * 3);
  await page.getByRole('button', { name: /^SW6, MX switch/ }).hover();
  await expect(page.locator('.wb-scope-outline.is-hover')).toBeVisible();
});

for (const theme of ['light', 'dark']) {
  test(`${theme} Parts library and inspector fit desktop and narrow screens`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem('boardstudio:v2:theme', theme), theme);
    await page.goto('/');
    await navigateWorkspace(page, 'Parts');
    await expect(page.getByRole('img', { name: 'Footprint preview' })).toBeVisible();
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByRole('button', { name: 'Objects', exact: true }).click();
    await expect(page.getByRole('searchbox', { name: 'Search footprints' })).toBeVisible();
    await page.getByRole('option', { name: 'MX Hotswap RGB', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Place key assembly' })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const inspector = page.locator('.wb-inspector-content');
    expect(await inspector.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    await page.getByRole('button', { name: 'Inspect', exact: true }).click();
    await expect(page.getByRole('button', { name: '3D model', exact: true })).toBeVisible();
  });
}

test('key assembly placement can cancel and commits its companions in one undo step', async ({ page }) => {
  await page.goto('/');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('option', { name: 'MX Hotswap RGB', exact: true }).click();
  await page.getByRole('button', { name: 'Place key assembly' }).click();
  await page.keyboard.press('Escape');
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('button', { name: 'Place key assembly' }).click();
  await page.getByRole('button', { name: 'Ghost key, row 1, column 1', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(18);
  await expect(page.getByRole('treeitem', { name: 'Matrix 2 1 keys', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
  await expect(page.getByRole('treeitem', { name: /^Matrix 2/ })).toHaveCount(0);
});

test('Parts renders an attached model through the visible 3D control', async ({ page }) => {
  test.setTimeout(60_000);
  const model = await buildCase(prepareCase({ revision: 0, body: { id: 'model', name: 'Model', boardId: 'main-board', kind: 'plate', thickness: 2, clearance: 0 }, contours: [{ hole: false, points: [{ x: -3, y: -3 }, { x: 3, y: -3 }, { x: 3, y: 3 }, { x: -3, y: 3 }] }] }));
  await page.goto('/');
  const document = demoProject();
  document.definitions.push({ ...document.definitions[0], generator: undefined, id: 'custom-model-switch', name: 'Custom model switch' });
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.locator('.wb-project-file-input').setInputFiles({ name: 'custom-switch.boardstudio', mimeType: 'application/zip', buffer: Buffer.from(zipSync({ 'project.json': strToU8(JSON.stringify(document)) })) });
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('option', { name: 'Custom model switch', exact: true }).click();
  await page.locator('.wb-inspector-section > summary').filter({ hasText: '3D model' }).click();
  await page.locator('.wb-model-import input[type=file]').setInputFiles({
    name: 'switch.step', mimeType: 'model/step',
    buffer: Buffer.from(model.step),
  });
  await expect(page.getByText('Bound asset: switch.step')).toBeVisible();
  await page.getByRole('button', { name: '3D model', exact: true }).click();
  await expect(page.getByText('1 / 1 models · 1.6 mm PCB', { exact: true })).toBeVisible({ timeout: 30_000 });
  await expect(page.locator('.wb-assembly-scene canvas')).toBeVisible();
  await page.locator('.wb-model-import input[type=file]').setInputFiles({ name: 'export.wrl', mimeType: 'model/vrml', buffer: Buffer.from('#VRML V2.0 utf8\nShape { geometry Box { size 10 10 5 } }') });
  await expect(page.getByText('Bound asset: export.wrl')).toBeVisible();
  await expect(page.getByText('1 / 1 models · 1.6 mm PCB', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: '2D footprint', exact: true }).click();
  await expect(page.getByRole('img', { name: 'Footprint preview' })).toBeVisible();
});
