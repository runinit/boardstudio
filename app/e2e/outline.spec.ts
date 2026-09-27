import { navigateWorkspace } from './workspace-navigation';
import { chooseScope } from './selection';
import { configureMatrix } from './matrix-setup';
import { expect, test, type Page } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { strToU8, zipSync } from 'fflate';
import { emptyProject } from '@boardstudio/v2-contracts';
import { downloadDraftBoard } from './pcb-package';
import { openCustomSwitchProject } from './custom-switch';

type Point = { x: number; y: number };
async function contours(page: Page): Promise<Point[][]> {
  return page.locator('.wb-outline-shape').evaluateAll((nodes) => nodes.map((node) => (node.getAttribute('points') ?? '').trim().split(/\s+/).filter(Boolean).map((pair) => { const [x, y] = pair.split(',').map(Number); return { x, y }; })));
}
function contains(paths: Point[][], p: Point): boolean {
  return paths.reduce((inside, path) => {
    let hit = false;
    for (let i = 0; i < path.length; i++) {
      const a = path[i], b = path[(i + 1) % path.length];
      if ((a.y > p.y) !== (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x) hit = !hit;
    }
    return inside !== hit;
  }, false);
}
async function clickPoint(page: Page, point: Point, double = false) {
  const screen = await page.locator('.wb-canvas').evaluate((node: SVGSVGElement, p) => {
    const at = new DOMPoint(p.x, -p.y).matrixTransform(node.getScreenCTM()!);
    return { x: at.x, y: at.y };
  }, point);
  await page.keyboard.down('Alt');
  if (double) await page.mouse.dblclick(screen.x, screen.y); else await page.mouse.click(screen.x, screen.y);
  await page.keyboard.up('Alt');
}

test('deleting a corner changes the perimeter, finishing and exports follow it, and undo restores it', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  const before = await contours(page);
  expect(contains(before, { x: 0, y: 0 })).toBe(true);
  await page.getByRole('button', { name: /^SW1, MX switch/ }).click();
  await page.keyboard.press('Delete');
  await expect(page.getByRole('button', { name: /^SW1, MX switch/ })).toHaveCount(0);
  await expect.poll(async () => contains(await contours(page), { x: 0, y: 0 })).toBe(false);
  const deleted = await contours(page);
  expect(contains(deleted, { x: 19.05, y: 0 })).toBe(true);
  expect(contains(deleted, { x: 0, y: 19.05 })).toBe(true);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(() => contours(page)).toEqual(before);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect.poll(() => contours(page)).toEqual(deleted);

  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Board outline…', exact: true }).click();
  await page.getByLabel('Outline corners').selectOption('chamfer');
  await expect.poll(() => contours(page)).not.toEqual(deleted);
  await expect(page.getByLabel('Chamfer size')).toBeVisible();
  const finished = await contours(page);
  expect(contains(finished, { x: 0, y: 0 })).toBe(false);
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export SVG board outline', exact: true }).click();
  const svg = await readFile((await (await download).path())!, 'utf8');
  for (const path of finished) for (const p of path) expect(svg).toContain(`${p.x} ${-p.y}`);
  for (const [label, kind] of [['DXF board outline', 'dxf'], ['KiCad board', 'pcb']]) {
    let content: string;
    if (kind === 'pcb') {
      content = (await downloadDraftBoard(page, { reviewExistingConnections: true })).board;
    } else {
      const downloading = page.waitForEvent('download');
      await page.getByRole('button', { name: `Export ${label}`, exact: true }).click();
      content = await readFile((await (await downloading).path())!, 'utf8');
    }
    for (const path of finished) for (const p of path) {
      expect(content).toContain(kind === 'dxf' ? `10\n${p.x}\n20\n${p.y}\n` : `(start ${p.x} ${-p.y})`);
    }
  }
  await page.reload();
  await expect.poll(() => contours(page)).toEqual(finished);
});

test('draws additions and persistent cutouts without duplicate double-click vertices', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Board outline…', exact: true }).click();
  await page.getByLabel('Outline corners').selectOption('sharp');
  await page.getByRole('button', { name: 'Draw addition' }).click();
  await clickPoint(page, { x: 1, y: -1 });
  await clickPoint(page, { x: 4, y: -1 });
  await clickPoint(page, { x: 4, y: -4 });
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Remove addition 1' })).toBeVisible();

  await page.getByRole('button', { name: 'Draw cutout' }).click();
  await clickPoint(page, { x: 8, y: -8 });
  await clickPoint(page, { x: 11, y: -8 });
  await clickPoint(page, { x: 11, y: -11 }, true);
  await expect(page.locator('.wb-outline-shape.is-hole')).toHaveCount(1);
  expect(contains(await contours(page), { x: 10, y: -9 })).toBe(false);
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.getByLabel('Outline margin').fill('5');
  await page.getByLabel('Outline margin').blur();
  await expect(page.locator('.wb-outline-shape.is-hole')).toHaveCount(1);
  await page.getByRole('button', { name: 'Remove cutout 2' }).click();
  await expect(page.locator('.wb-outline-shape.is-hole')).toHaveCount(0);
});

test('part inclusion and margin controls survive save and affect the contour', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  const key = page.getByRole('button', { name: /^SW1, MX switch/ });
  await key.click();
  await chooseScope(page, 'component');
  await page.locator('summary').filter({ hasText: 'Board outline' }).click();
  await page.getByLabel('Use board margin', { exact: true }).click();
  await expect(page.getByLabel('Use board margin', { exact: true })).not.toBeChecked();
  await expect(page.getByLabel('Part edge margin', { exact: true })).toHaveValue('0');
  await expect.poll(async () => contains(await contours(page), { x: -11, y: 0 })).toBe(false);
  await page.getByLabel('Include in outline', { exact: true }).click();
  await expect(page.getByLabel('Include in outline', { exact: true })).not.toBeChecked();
  await expect(key).toBeVisible();
  await expect.poll(async () => contains(await contours(page), { x: 0, y: 0 })).toBe(false);
  await page.getByLabel('Include in outline', { exact: true }).click();
  await expect(page.getByLabel('Include in outline', { exact: true })).toBeChecked();
  await expect.poll(async () => contains(await contours(page), { x: 0, y: 0 })).toBe(true);
  await page.getByLabel('Keycap width', { exact: true }).fill('30');
  await page.getByLabel('Keycap width', { exact: true }).blur();
  await expect.poll(async () => contains(await contours(page), { x: -14, y: 0 })).toBe(true);
  const expected = await contours(page);
  await page.reload();
  await expect.poll(() => contours(page)).toEqual(expected);
});

test('outline settings fit desktop and mobile and invalid drawings block exports', async ({ page }, info) => {
  await page.goto('/');
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Board outline…', exact: true }).click();
  await page.screenshot({ path: info.outputPath('outline-desktop.png') });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole('heading', { name: 'Board outline', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath('outline-mobile.png') });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByRole('button', { name: 'Draw cutout' }).click();
  for (const p of [{x:5,y:-5},{x:15,y:-15},{x:5,y:-15},{x:15,y:-5}]) await clickPoint(page,p);
  await page.keyboard.press('Enter');
  await expect(page.getByText(/non-self-intersecting polygon/).first()).toBeVisible();
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  await expect(page.locator('.wb-export-row').filter({ hasText: 'SVG board outline' }).getByRole('button')).toBeDisabled();
});

test('deleted matrix corners remain empty after resizing and rotation follows the contour frame', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await page.getByRole('button', { name: 'Back to objects', exact: true }).click();
  await configureMatrix(page);
  await expect(page.locator('.wb-scene-part')).toHaveCount(60);
  await chooseScope(page, 'key');
  await page.getByRole('button', { name: /^SW1, switch mx,/ }).click();
  await page.keyboard.press('Delete');
  await expect(page.locator('.wb-scene-part')).toHaveCount(58);
  await expect(page.locator('.wb-matrix-cell.is-empty')).toHaveCount(0);
  await chooseScope(page, 'matrix');
  const columns = page.getByRole('spinbutton', { name: /Columns/ });
  await columns.fill('6');
  await columns.blur();
  await expect(page.locator('.wb-scene-part')).toHaveCount(70);
  await expect(page.locator('.wb-matrix-cell.is-empty')).toHaveCount(0);
  const rotation = page.getByRole('spinbutton', { name: 'Rotation °', exact: true });
  await rotation.fill('30');
  await rotation.blur();
  await expect(page.getByRole('button', { name: /^SW3, switch mx,/ })).toHaveAttribute('transform', /rotate\(30\)/);
  await expect(page.getByRole('button', { name: 'Select key, row 1, column 2' })).toHaveAttribute('transform', /rotate\(30\)/);
});

test('definition keycaps update the outline and per-instance overrides can be reset', async ({ page }) => {
  await openCustomSwitchProject(page);
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('listbox', { name: 'Footprint library', exact: true }).getByRole('option', { name: 'Fixture switch', exact: true }).click();
  await page.getByLabel('Definition keycap width', { exact: true }).fill('30');
  await page.getByLabel('Definition keycap width', { exact: true }).blur();
  await navigateWorkspace(page, 'Layout');
  await expect.poll(async () => contains(await contours(page), { x: -18, y: 0 })).toBe(true);
  await page.getByRole('button', { name: /^SW1, Fixture switch,/ }).click();
  await chooseScope(page, 'component');
  await page.locator('summary').filter({ hasText: 'Board outline' }).click();
  await page.getByLabel('Keycap width', { exact: true }).fill('18');
  await page.getByLabel('Keycap width', { exact: true }).blur();
  await expect.poll(async () => contains(await contours(page), { x: -18, y: 0 })).toBe(false);
  await page.getByRole('button', { name: 'Use definition keycap' }).click();
  await expect.poll(async () => contains(await contours(page), { x: -18, y: 0 })).toBe(true);
});

test('a newly introduced library definition joins the board automatic envelope', async ({ page }) => {
  const doc = emptyProject('empty-import', 'Empty import');
  doc.boards = [{ id: 'board', name: 'Main board', partIds: [], outlineIds: ['edge'], netIds: [], thickness: 1.6 }];
  doc.outline = [{ id: 'edge', kind: 'part-envelope', partIds: [], margin: 4, operation: 'add' }];
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.locator('.wb-project-file-input').setInputFiles({ name: 'empty.boardstudio', mimeType: 'application/zip', buffer: Buffer.from(zipSync({ 'project.json': strToU8(JSON.stringify(doc)) })) });
  await expect(page.getByRole('button', { name: 'Project', exact: true })).toHaveAttribute('title', 'Project menu — Empty import');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(page.getByRole('textbox', { name: 'Project name' })).toHaveValue('Empty import');
  await navigateWorkspace(page, 'Parts');
  await page.getByRole('option', { name: 'Choc V1 / V2 switch', exact: true }).click();
  await page.getByRole('button', { name: 'Place component', exact: true }).click();
  await page.keyboard.press('Enter');
  await expect(page.locator('.wb-scene-part')).toHaveCount(1);
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  expect(contains(await contours(page), { x: 0, y: 0 })).toBe(true);
});
