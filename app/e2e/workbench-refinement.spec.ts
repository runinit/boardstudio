import { chooseScope } from './selection';
import { configureMatrix } from './matrix-setup';
import { expect, test } from '@playwright/test';
import { strToU8, zipSync, unzipSync, strFromU8 } from 'fflate';
import { readFile } from 'node:fs/promises';
import { demoProject } from '../src/demo';
import type { ProjectDoc } from '../../contracts/src/index';

test('starter keycap overlays align with their switch members', async ({ page }) => {
  await page.goto('/');
  const overlay = page.getByRole('button', { name: 'Select key, row 2, column 1', exact: true });
  const member = page.getByRole('button', { name: /^SW6, MX switch/ });
  await expect(member).toBeVisible();
  const overlayTransform = await overlay.getAttribute('transform');
  const memberTransform = await member.getAttribute('transform');
  expect(overlayTransform).toBe(memberTransform);
});

async function editNumber(page: import('@playwright/test').Page, name: string, value: string) {
  const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
  const field = page.getByRole('spinbutton', { name, exact: true });
  await field.fill(value);
  await field.blur();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', String(revision + 1));
}

test('selection-specific inspectors, group counts and grouping persistence', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Main board', exact: true })).toBeVisible();
  await expect(page.getByRole('spinbutton', { name: 'Rows keys', exact: true })).toHaveCount(0);
  await expect(page.getByRole('treeitem', { name: 'Column 1 3 keys', exact: true })).toBeVisible();
  await page.getByRole('treeitem', { name: /^Column 1/ }).click();
  await expect(page.getByRole('heading', { name: 'Matrix 1 · Column 1', exact: true })).toBeVisible();
  await expect(page.getByRole('spinbutton', { name: 'Rows keys', exact: true })).toHaveCount(0);
  await page.getByText('Position & rotation', { exact: true }).click();
  await editNumber(page, 'Offset X mm', '3');
  await expect(page.getByRole('button', { name: /^SW1, MX switch/ })).toHaveAttribute('transform', 'translate(3 0) rotate(0)');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.getByRole('spinbutton', { name: 'Offset X mm' })).toHaveValue('0');
  await page.getByRole('button', { name: 'Expand Column 1' }).click();
  await page.getByRole('treeitem', { name: /^Key 1\.2/ }).click();
  await expect(page.getByRole('heading', { name: 'Matrix 1 · Key 1.2', exact: true })).toBeVisible();
  await editNumber(page, 'Local X mm', '2');
  await editNumber(page, 'Key rotation °', '20');
  const overlay = page.getByRole('button', { name: 'Select key, row 2, column 1', exact: true });
  await expect(overlay).toHaveAttribute('transform', 'translate(2 19.05) rotate(20)');
  await expect(page.getByRole('button', { name: /^SW6, MX switch/ })).toHaveAttribute('transform', 'translate(2 19.05) rotate(20)');
  await page.getByRole('checkbox', { name: 'Key enabled' }).click();
  await expect(page.getByRole('checkbox', { name: 'Key enabled' })).not.toBeChecked();
  await expect(page.getByRole('treeitem', { name: 'Column 1 2 keys', exact: true })).toBeVisible();
  await expect(page.getByRole('treeitem', { name: 'Key 1.2 Empty slot', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await page.getByRole('combobox', { name: 'Tree grouping' }).selectOption('row');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('heading', { name: 'Matrix 1 · Key 1.2', exact: true })).toBeVisible();
  await expect(page.getByRole('treeitem', { name: 'Row 2 4 keys', exact: true })).toBeVisible();
  await page.getByRole('checkbox', { name: 'Key enabled' }).click();
  await expect(page.getByRole('checkbox', { name: 'Key enabled' })).toBeChecked();
  await expect(page.getByRole('treeitem', { name: 'Row 2 5 keys', exact: true })).toBeVisible();
  await page.reload();
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Tree grouping' })).toHaveValue('row');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Select key, row 2, column 1', exact: true })).toHaveAttribute('transform', /rotate\(20\)/);
});

test('Add Part searches, cancels, and places a standalone snapped component in one undo step', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /^SW1, MX switch/ }).click();
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('searchbox', { name: 'Search parts' }).fill('choc');
  await page.locator('#wb-add-part').getByRole('button', { name: 'Choc V1 / V2 switch', exact: true }).click();
  await expect(page.locator('#wb-add-part')).toHaveCount(0);
  await expect(page.locator('.wb-placement-preview')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.locator('.wb-placement-preview')).toHaveCount(0);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '0');
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('searchbox', { name: 'Search parts' }).fill('choc');
  await page.locator('#wb-add-part').getByRole('button', { name: 'Choc V1 / V2 switch', exact: true }).click();
  const canvas = page.locator('.wb-canvas');
  await canvas.focus();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('Enter');
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '1');
  await expect(page.locator('.wb-scene-part')).toHaveCount(16);
  await expect(page.getByRole('button', { name: /^SW1, MX switch/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'S1', exact: true })).toBeVisible();
  await expect(page.getByRole('spinbutton', { name: 'Local X mm' })).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Position', exact: false })).toBeVisible();
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(16);
});

test('command controls remain fixed through zoom and pan and both themes persist', async ({ page }) => {
  await page.goto('/');
  const scope = page.getByRole('button', { name: /^Select:/ });
  const snap = page.getByRole('button', { name: 'Snap', exact: true });
  const beforeScope = await scope.boundingBox();
  const beforeSnap = await snap.boundingBox();
  const canvas = page.locator('.wb-canvas');
  const bounds = (await canvas.boundingBox())!;
  await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await page.mouse.wheel(0, -300);
  await expect(page.locator('.wb-footer-zoom span')).not.toHaveText('100%');
  await page.keyboard.down('Space');
  await page.mouse.down();
  await page.mouse.move(bounds.x + bounds.width / 2 + 40, bounds.y + bounds.height / 2 + 20);
  await page.mouse.up();
  await page.keyboard.up('Space');
  expect(await scope.boundingBox()).toEqual(beforeScope);
  expect(await snap.boundingBox()).toEqual(beforeSnap);
  for (const theme of ['dark', 'light']) {
    await page.getByRole('button', { name: 'Project', exact: true }).click();
    await page.getByRole('combobox', { name: 'Color theme' }).selectOption(theme);
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
    expect(await page.locator('.wb-canvas-stage').evaluate((node) => getComputedStyle(node).backgroundColor)).toBe(theme === 'dark' ? 'rgb(16, 26, 34)' : 'rgb(233, 238, 242)');
  }
});

test('reopened saved and transformed sparse matrices retain member alignment and saved identity', async ({ page }) => {
  const doc = demoProject();
  delete doc.matrices[0].mirror;
  doc.parts[6].pose = { at: { x: 22, y: -23 }, rotation: 13 };
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.locator('.wb-project-file-input').setInputFiles({ name: 'saved.boardstudio', mimeType: 'application/zip', buffer: Buffer.from(zipSync({ 'project.json': strToU8(JSON.stringify(doc)) })) });
  const overlay = page.getByRole('button', { name: 'Select key, row 2, column 2', exact: true });
  await expect(overlay).toHaveAttribute('transform', 'translate(22 -23) rotate(13)');
  await page.getByRole('treeitem', { name: /^Matrix 1/ }).click();
  await editNumber(page, 'Rotation °', '30');
  await expect.poll(async () => overlay.getAttribute('transform')).toBe(await page.getByRole('button', { name: /^SW7, MX switch/ }).getAttribute('transform'));
  await chooseScope(page, 'key');
  await page.getByRole('button', { name: /^SW15, MX switch/ }).click();
  await page.keyboard.press('Delete');
  await expect(page.locator('.wb-scene-part')).toHaveCount(14);
  await expect(page.locator('.wb-matrix-cell.is-empty')).toHaveCount(0);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '2');
  await page.reload();
  await expect.poll(async () => page.getByRole('button', { name: 'Select key, row 2, column 2', exact: true }).getAttribute('transform')).toBe(await page.getByRole('button', { name: /^SW7, MX switch/ }).getAttribute('transform'));
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: /Save .boardstudio project/ }).click();
  const data = unzipSync(await readFile((await (await download).path())!));
  const saved = JSON.parse(strFromU8(data['project.json'])) as ProjectDoc;
  expect(saved.parts.map((part) => part.id)).toEqual(doc.parts.slice(0, 14).map((part) => part.id));
  expect(saved.boards[0].partIds).toEqual(doc.boards[0].partIds.slice(0, 14));
  expect(saved.nets[0]).toEqual(doc.nets[0]);
});

test('generated overlays track rotation, mirroring, stagger and disabled slots', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await page.getByRole('button', { name: 'Skip guide', exact: true }).click();
  await configureMatrix(page);
  await expect(page.locator('.wb-scene-part')).toHaveCount(60);
  await editNumber(page, 'Rotation °', '37');
  await page.getByRole('combobox', { name: 'Mirror matrix' }).selectOption('x');
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '3');
  await page.getByRole('treeitem', { name: /^Column 2/ }).click();
  await page.getByText('Position & rotation', { exact: true }).click();
  await editNumber(page, 'Offset Y mm', '4');
  const overlay = page.getByRole('button', { name: 'Select key, row 1, column 2', exact: true });
  const part = page.getByRole('button', { name: /^SW3, switch mx,/ });
  await expect.poll(async () => overlay.getAttribute('transform')).toBe(await part.getAttribute('transform'));
  await chooseScope(page, 'key');
  await part.click();
  await page.getByRole('checkbox', { name: 'Key enabled' }).click();
  await expect(page.getByRole('checkbox', { name: 'Key enabled' })).not.toBeChecked();
  await expect(page.locator('.wb-matrix-cell.is-empty')).toHaveCount(0);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(async () => overlay.getAttribute('transform')).toBe(await part.getAttribute('transform'));
});

for (const theme of ['light', 'dark'] as const) {
  test(`${theme} desktop and narrow controls, flyout and drawers fit the viewport`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem('boardstudio:v2:theme', theme), theme);
    await page.goto('/');
    await page.getByRole('treeitem', { name: /^Matrix 1/ }).click();
    await expect(page.getByRole('heading', { name: 'Matrix 1', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Add object', exact: true }).click();
    await expect(page.getByRole('searchbox', { name: 'Search parts' })).toBeFocused();
    await page.keyboard.press('Escape');
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByRole('button', { name: 'Inspect', exact: true }).click();
    await expect(page.getByRole('button', { name: /^Select:/ })).toBeVisible();
    await page.locator('.wb-topbar').getByRole('button', { name: 'Objects', exact: true }).click();
    await page.getByRole('button', { name: 'Add object', exact: true }).click();
    await expect(page.getByRole('searchbox', { name: 'Search parts' })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog', { name: 'Add', exact: true })).toHaveCount(0);
    await page.locator('.wb-topbar').getByRole('button', { name: 'Objects', exact: true }).click();
    await chooseScope(page, 'key');
    await page.getByRole('button', { name: 'Inspect', exact: true }).click();
    await page.getByRole('button', { name: /^SW1, MX switch/ }).click();
    await expect(page.getByRole('heading', { name: 'Matrix 1 · Key 1.1' })).toBeVisible();
    await expect(page.getByRole('group', { name: 'Selection scope' })).not.toBeVisible();
    const inspector = page.locator('.wb-inspector-content');
    expect(await inspector.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
  });
}

test('mouse placement snaps a standalone part onto only the active board', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'New board', exact: true }).click();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '1');
  await expect(page.locator('.wb-scene-part')).toHaveCount(0);
  await expect(page.locator('.wb-matrix-cell')).toHaveCount(0);
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('searchbox', { name: 'Search parts' }).fill('MX switch');
  await page.locator('#wb-add-part').getByRole('button', { name: 'MX switch', exact: true }).click();
  const point = await page.locator('.wb-canvas').evaluate((node: SVGSVGElement) => {
    const screen = new DOMPoint(20.2, 9.9).matrixTransform(node.getScreenCTM()!);
    return { x: screen.x, y: screen.y };
  });
  await page.mouse.click(point.x, point.y);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '2');
  await expect(page.locator('.wb-scene-part')).toHaveCount(1);
  await expect(page.locator('.wb-scene-part')).toHaveAttribute('transform', 'translate(19.05 -9.525) rotate(0)');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(0);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(1);
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('combobox', { name: 'Selected board' }).selectOption('main-board');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(15);
});

test('column stagger and splay update following keys and support undo, redo and reload', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 2 3 keys', exact: true }).click();
  await page.getByText('Position & rotation', { exact: true }).click();
  await editNumber(page, 'Stagger mm', '5');
  await editNumber(page, 'Splay °', '15');
  const key = page.getByRole('button', { name: /^SW3, MX switch/ });
  await expect(key).toHaveAttribute('transform', /rotate\(15\)/);
  const transformed = await key.getAttribute('transform');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(key).toHaveAttribute('transform', /rotate\(0\)/);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(key).toHaveAttribute('transform', transformed!);
  await page.reload();
  await expect(key).toHaveAttribute('transform', transformed!);
  await page.getByRole('treeitem', { name: 'Column 2 3 keys', exact: true }).click();
  await page.getByText('Position & rotation', { exact: true }).click();
  await expect(page.getByRole('spinbutton', { name: 'Stagger mm', exact: true })).toHaveValue('5');
  await expect(page.getByRole('spinbutton', { name: 'Splay °', exact: true })).toHaveValue('15');
  for (const theme of ['dark', 'light']) {
    await page.setViewportSize({ width: 1420, height: 900 });
    await page.getByRole('button', { name: 'Project', exact: true }).click();
    await page.getByRole('combobox', { name: 'Color theme' }).selectOption(theme);
    await page.getByRole('button', { name: 'Project', exact: true }).click();
    await page.setViewportSize({ width: 390, height: 844 });
    if (!(await page.getByRole('spinbutton', { name: 'Splay °', exact: true }).isVisible())) {
      await page.getByRole('button', { name: 'Inspect', exact: true }).click();
    }
    await expect(page.getByRole('spinbutton', { name: 'Splay °', exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
});

test('dragging a splayed column follows the pointer in board coordinates', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 2 3 keys', exact: true }).click();
  await editNumber(page, 'Splay °', '25');
  // Use the middle key; the base key now hosts the separate splay-origin handle.
  const key = page.getByRole('button', { name: /^SW7, MX switch/ });
  const position = async () => (await key.getAttribute('transform'))!.match(/translate\(([^ ]+) ([^)]+)\)/)!.slice(1).map(Number);
  const before = await position();
  const scale = await page.locator('svg.wb-canvas').evaluate((element) => (element as SVGSVGElement).getScreenCTM()!.inverse().a);
  const box = (await key.boundingBox())!;
  await page.keyboard.down('Alt');
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 30, box.y + box.height / 2, { steps: 4 });
  await page.mouse.up();
  await page.keyboard.up('Alt');
  await expect.poll(async () => (await position())[0] - before[0]).toBeCloseTo(30 * scale, 1);
  expect((await position())[1]).toBeCloseTo(before[1], 1);
});
