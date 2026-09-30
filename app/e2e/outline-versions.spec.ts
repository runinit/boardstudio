import { expect, test, type Page } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import { readFile } from 'node:fs/promises';
import { strFromU8, unzipSync } from 'fflate';

type Point = { x: number; y: number };
const retained = fileURLToPath(new URL('../../docs/design/evidence/board-outlines/reviung41-original.boardstudio', import.meta.url));
async function openRetained(page: Page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.locator('.wb-project-file-input').setInputFiles(retained);
  await expect(page.locator('.wb-scene-part')).toHaveCount(85);
}
async function openOutline(page: Page) {
  await page.getByRole('treeitem', { name: /^Outline / }).click();
  await expect(page.getByRole('heading', { name: 'Board outline', exact: true })).toBeVisible();
}
async function paths(page: Page): Promise<Point[][]> {
  return page.locator('.wb-outline-shape').evaluateAll(nodes => nodes.map(node => (node.getAttribute('points') ?? '').split(/\s+/).filter(Boolean).map(pair => {
    const [x, y] = pair.split(',').map(Number); return { x, y };
  })));
}
function contains(paths: Point[][], point: Point): boolean {
  return paths.reduce((inside, path) => {
    let hit = false;
    path.forEach((a, index) => { const b = path[(index + 1) % path.length]; if ((a.y > point.y) !== (b.y > point.y) && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x) hit = !hit; });
    return inside !== hit;
  }, false);
}

test('retained red gaps repair, Keep gap persists, and bridge references highlight the same material', async ({ page }, info) => {
  await openRetained(page);
  for (const point of [{ x: 104.7, y: -80 }, { x: 152.7, y: -80 }, { x: 256, y: -15 }, { x: 280.5, y: -37.4 }]) expect(contains(await paths(page), point)).toBe(true);
  expect(contains(await paths(page), { x: 106, y: 6 })).toBe(false);
  expect(contains(await paths(page), { x: 128.615, y: 5 })).toBe(false);
  await openOutline(page);
  await expect(page.getByRole('checkbox', { name: /^Keep gap \d+$/ })).toHaveCount(5);
  await page.getByRole('button', { name: 'Fit board', exact: true }).click();
  await page.screenshot({ path: info.outputPath('reviung-repaired.png') });
  const generated = await paths(page);
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  for (const kind of ['SVG', 'DXF']) {
    const downloading = page.waitForEvent('download');
    await page.getByRole('button', { name: `Export ${kind} board outline`, exact: true }).click();
    const path = info.outputPath(`reviung-repaired.${kind.toLowerCase()}`);
    await (await downloading).saveAs(path);
    const content = await readFile(path, 'utf8');
    for (const contour of generated) for (const p of contour) expect(content).toContain(kind === 'SVG' ? `${p.x} ${-p.y}` : `10\n${p.x}\n20\n${p.y}\n`);
    if (kind === 'DXF') expect(content).toContain(`0\nLWPOLYLINE\n8\nOUTLINE\n90\n${generated[0].length}\n70\n1\n`);
  }
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  await openOutline(page);
  await page.getByRole('button', { name: 'Show gap 1', exact: true }).click();
  await expect(page.locator('[data-outline-gap]')).toHaveCount(1);
  await page.getByRole('checkbox', { name: 'Keep gap 1', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Keep gap 1', exact: true })).toBeChecked();
  await expect.poll(async () => contains(await paths(page), { x: 256, y: -15 })).toBe(false);
  await expect(page.getByLabel('Saved locally', { exact: true })).toBeVisible();
  await page.reload(); await openOutline(page);
  await expect(page.getByRole('checkbox', { name: 'Keep gap 1', exact: true })).toBeChecked();
  await page.getByRole('button', { name: /^main-U1, mcu nice nano,/ }).click();
  await page.getByRole('spinbutton', { name: 'X mm', exact: true }).fill('274');
  await page.getByRole('spinbutton', { name: 'X mm', exact: true }).blur();
  await expect(page.getByRole('button', { name: /^main-U1, mcu nice nano,/ })).toHaveAttribute('transform', /translate\(274 /);
  await openOutline(page);
  await expect(page.getByRole('checkbox', { name: 'Keep gap 1', exact: true })).toBeChecked();
  await page.getByRole('checkbox', { name: 'Keep gap 1', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Keep gap 1', exact: true })).not.toBeChecked();
  await expect.poll(async () => contains(await paths(page), { x: 256, y: -15 })).toBe(true);
  await page.locator('.wb-tree-row.is-bridge').first().getByRole('treeitem').click();
  await expect(page.getByRole('heading', { name: 'Outline bridge', exact: true })).toBeVisible();
  const highlighted = await page.locator('[data-outline-bridge]').getAttribute('data-outline-bridge');
  await page.getByRole('button', { name: 'Expand right keys', exact: true }).click();
  const reference = page.locator('.wb-tree-row.is-bridge').filter({ has: page.getByRole('treeitem', { name: 'Bridge 1 10 mm', exact: true }) });
  await expect(reference).toHaveCount(2);
  await reference.last().getByRole('treeitem').click();
  await expect(page.locator('[data-outline-bridge]')).toHaveAttribute('data-outline-bridge', highlighted!);
});

test('first edit creates a fixed version, lifecycle supports Undo, and invalid active output stays editable and saveable', async ({ page }, info) => {
  await openRetained(page); await openOutline(page);
  const generated = await paths(page);
  await page.getByRole('button', { name: 'Edit perimeter points', exact: true }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Active outline', exact: true }).locator('option')).toHaveCount(1);
  await page.getByRole('button', { name: 'Edit perimeter points', exact: true }).click();
  const x = page.getByLabel('Point 1 X');
  const exact = String(Number(await x.inputValue()) - .25);
  await x.fill(exact); await x.blur();
  await expect(x).toHaveValue(exact);
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.getByRole('textbox', { name: 'Outline version name', exact: true })).toHaveValue('Edited outline 1');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(() => paths(page)).toEqual(generated);
  await expect(page.getByRole('combobox', { name: 'Active outline', exact: true }).locator('option')).toHaveCount(1);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await page.getByRole('textbox', { name: 'Outline version name', exact: true }).fill('Fixed review');
  await page.getByRole('textbox', { name: 'Outline version name', exact: true }).blur();
  const fixed = await paths(page);
  await page.getByRole('button', { name: 'Copy outline', exact: true }).click();
  await page.getByRole('button', { name: 'Delete outline', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Active outline', exact: true })).toHaveValue('');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.getByRole('textbox', { name: 'Outline version name', exact: true })).toHaveValue('Edited outline 1');
  await page.getByRole('combobox', { name: 'Active outline', exact: true }).selectOption({ label: 'Fixed review' });
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const outlineDownload = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export SVG board outline', exact: true }).click();
  const fixedPath = info.outputPath('reviung-fixed.svg');
  await (await outlineDownload).saveAs(fixedPath);
  const fixedSvg = await readFile(fixedPath, 'utf8');
  for (const contour of fixed) for (const p of contour) expect(fixedSvg).toContain(`${p.x} ${-p.y}`);
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  await page.getByRole('button', { name: /^main-U1, mcu nice nano,/ }).click();
  await page.getByRole('spinbutton', { name: 'X mm', exact: true }).fill('310');
  await page.getByRole('spinbutton', { name: 'X mm', exact: true }).blur();
  await expect(page.getByRole('button', { name: /^main-U1, mcu nice nano,/ })).toHaveAttribute('transform', /translate\(310 /);
  await expect.poll(() => paths(page)).toEqual(fixed);
  await expect(page.locator('.wb-outline-finding')).not.toHaveCount(0);
  await page.getByRole('button', { name: /^Layout findings:/ }).click();
  await page.getByRole('region', { name: 'main-U1 · mcu nice nano', exact: true }).getByRole('button', { name: 'Select affected geometry', exact: true }).first().click();
  await expect(page.locator('.wb-outline-finding.is-focused')).toHaveCount(1);
  await page.screenshot({ path: info.outputPath('fixed-missing-support.png') });
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  for (const name of ['Export SVG board outline', 'Export DXF board outline', 'Export KiCad board', 'Export Draft KiCad board', 'Export Authored Case STEP']) await expect(page.getByRole('button', { name, exact: true })).toBeDisabled();
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save .boardstudio project', exact: true }).click();
  const projectPath = info.outputPath('reviung-fixed-invalid.boardstudio');
  await (await downloading).saveAs(projectPath);
  const archive = unzipSync(await readFile(projectPath));
  const saved = JSON.parse(strFromU8(archive['project.json']));
  expect(saved.boardOutlines[0].versions).toHaveLength(2);
  expect(saved.parts.find((part: { id: string }) => part.id === 'main/U1').pose.at.x).toBe(310);
  await page.reload(); await openOutline(page);
  await expect.poll(() => paths(page)).toEqual(fixed);
  await page.getByRole('combobox', { name: 'Active outline', exact: true }).selectOption({ label: 'Generated' });
  await expect.poll(() => paths(page)).not.toEqual(fixed);
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Export SVG board outline', exact: true })).toBeEnabled();
});
