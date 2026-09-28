import { readFile } from 'node:fs/promises';
import { expect, test, type Page } from '@playwright/test';
import { strFromU8, unzipSync } from 'fflate';
import type { ProjectDoc } from '../../contracts/src/index';
import { downloadDraftBoard } from './pcb-package';

async function newProject(page: Page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  await page.getByRole('button', { name: 'Back to objects', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(0);
}

async function openSetup(page: Page) {
  const objects = page.locator('.wb-topbar').getByRole('button', { name: 'Objects', exact: true });
  if (await objects.isVisible() && await objects.getAttribute('aria-expanded') === 'false') {
    await page.locator('.wb-topbar').getByRole('button', { name: 'Objects', exact: true }).click();
  }
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Mirrored pair…', exact: true }).click();
  return page.getByRole('form', { name: 'New mirrored pair' });
}

async function createPair(page: Page) {
  await openSetup(page);
  await page.getByRole('spinbutton', { name: 'Rows per half', exact: true }).fill('2');
  await page.getByRole('spinbutton', { name: 'Columns per half', exact: true }).fill('3');
  await page.getByRole('button', { name: 'Preview placement', exact: true }).click();
  await expect(page.locator('.wb-matrix-ghost.is-placement-preview')).toHaveCount(2);
  await page.locator('.wb-canvas').press('Enter');
  await expect(page.locator('.wb-scene-part')).toHaveCount(24);
  await expect(page.getByRole('treeitem', { name: 'Left half Linked', exact: true })).toBeVisible();
  await expect(page.getByRole('treeitem', { name: 'Right half Linked', exact: true })).toBeVisible();
}

async function archive(page: Page) {
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: /Save .boardstudio project/ }).click();
  const download = await downloading;
  const path = (await download.path())!;
  const files = unzipSync(new Uint8Array(await readFile(path)));
  return { path, document: JSON.parse(strFromU8(files['project.json'])) as ProjectDoc };
}

test('mirrored creation is cancellable, atomic and edits either half', async ({ page }, testInfo) => {
  await page.addInitScript(() => localStorage.setItem('boardstudio:v2:theme', 'dark'));
  await page.setViewportSize({ width: 1440, height: 900 });
  await newProject(page);
  await openSetup(page);
  await page.screenshot({ path: testInfo.outputPath('dark-pair-setup.png'), animations: 'disabled' });
  const revision = await page.locator('.wb-root').getAttribute('data-revision');
  await page.getByRole('button', { name: 'Preview placement', exact: true }).click();
  await page.locator('.wb-canvas').press('Escape');
  await expect(page.locator('.wb-matrix-ghost.is-placement-preview')).toHaveCount(0);
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', revision!);
  await createPair(page);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(0);
  await expect(page.getByRole('treeitem', { name: /Left half|Right half/ })).toHaveCount(0);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(24);
  await page.getByRole('treeitem', { name: 'Right half Linked', exact: true }).click();
  const rows = page.getByRole('spinbutton', { name: 'Rows keys', exact: true });
  await rows.fill('3');
  await rows.blur();
  await expect(page.locator('.wb-scene-part')).toHaveCount(36);
  await page.getByRole('treeitem', { name: 'Left half Linked', exact: true }).click();
  await expect(rows).toHaveValue('3');
  await expect(page.locator('#wb-inventory')).toContainText('Left half');
  await page.screenshot({ path: testInfo.outputPath('dark-linked-halves.png'), animations: 'disabled' });
  const saved = (await archive(page)).document;
  const { board: pcb } = await downloadDraftBoard(page);
  expect((pcb.match(/\(footprint /gu) ?? []).length).toBe(36);
  const left = saved.layouts!.find((layout) => layout.name === 'Left half')!;
  const right = saved.layouts!.find((layout) => layout.name === 'Right half')!;
  expect(right.mirrorLink?.sourceId).toBe(left.id);
  const leftMatrix = saved.matrices.find((matrix) => matrix.id === left.matrixId)!;
  const rightMatrix = saved.matrices.find((matrix) => matrix.id === right.matrixId)!;
  for (const id of leftMatrix.partIds.filter((id) => id.split('/').length === 3)) {
    const a = saved.parts.find((part) => part.id === id)!;
    const b = saved.parts.find((part) => part.id === id.replace(leftMatrix.id, rightMatrix.id))!;
    expect(a.pose.at.x + b.pose.at.x).toBeCloseTo(2 * right.mirrorLink!.axisX, 8);
    expect(a.pose.at.y).toBeCloseTo(b.pose.at.y, 8);
  }
});

test('linked components keep both halves through save, reopen and geometry edits', async ({ page }) => {
  await newProject(page);
  await createPair(page);
  await page.getByRole('treeitem', { name: 'Left half Linked', exact: true }).click();
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  const destination = page.getByRole('combobox', { name: 'Part placement layout', exact: true });
  await expect(destination.locator('option:checked')).toHaveText('Left half');
  await page.getByRole('searchbox', { name: 'Search parts', exact: true }).fill('encoder');
  await page.getByRole('dialog', { name: 'Add', exact: true }).getByRole('button', { name: 'rotary encoder ec11 ec12', exact: true }).click();
  await page.locator('.wb-canvas').press('Enter');
  await expect(page.locator('.wb-scene-part')).toHaveCount(26);
  await expect(page.getByRole('combobox', { name: 'Component layout', exact: true }).locator('option:checked')).toHaveText('Left half');
  await page.getByRole('treeitem', { name: 'Right half Linked', exact: true }).click();
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Part placement layout', exact: true }).locator('option:checked')).toHaveText('Right half');
  await page.getByRole('searchbox', { name: 'Search parts', exact: true }).fill('RGB LED');
  await page.getByRole('dialog', { name: 'Add', exact: true }).getByRole('button', { name: 'SK6812 MINI-E', exact: true }).click();
  await page.locator('.wb-canvas').press('Enter');
  await expect(page.locator('.wb-scene-part')).toHaveCount(27);
  const saved = await archive(page);
  expect(saved.document.layouts![0].partIds).toHaveLength(1);
  expect(saved.document.layouts![1].partIds).toHaveLength(2);
  const rightComponentId = saved.document.layouts![1].partIds.find(id => saved.document.parts.find(p => p.id === id)?.definitionId === 'ergogen:ceoloide/led_sk6812mini-e')!;
  const rightComponent = saved.document.parts.find((part) => part.id === rightComponentId)!;
  expect(rightComponent.definitionId).toBe('ergogen:ceoloide/led_sk6812mini-e');
  const componentId = saved.document.layouts![0].partIds[0];
  const component = saved.document.parts.find((part) => part.id === componentId)!;
  expect(saved.document.definitions.find((definition) => definition.id === component.definitionId)?.name).toBe('rotary encoder ec11 ec12');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  await page.getByRole('button', { name: 'Back to objects', exact: true }).click();
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.locator('.wb-project-file-input').setInputFiles(saved.path);
  await expect(page.locator('.wb-scene-part')).toHaveCount(27);
  await page.getByRole('treeitem', { name: 'Right half Linked', exact: true }).click();
  await page.getByRole('spinbutton', { name: 'Rows keys', exact: true }).fill('3');
  await page.getByRole('spinbutton', { name: 'Rows keys', exact: true }).blur();
  await expect(page.locator('.wb-scene-part')).toHaveCount(39);
  const changed = (await archive(page)).document;
  expect(changed.parts.find((part) => part.id === componentId)).toEqual(component);
  expect(changed.layouts!.find((layout) => layout.name === 'Right half')!.partIds).toEqual(saved.document.layouts![1].partIds);
  expect(changed.parts.find((part) => part.id === rightComponentId)).toEqual(rightComponent);
});

test('unlinking preserves both halves and permits independent layout changes', async ({ page }) => {
  await newProject(page);
  await createPair(page);
  await page.getByRole('button', { name: 'Unlink halves', exact: true }).click();
  await expect(page.getByRole('treeitem', { name: 'Left half Independent', exact: true })).toBeVisible();
  await expect(page.getByRole('treeitem', { name: 'Right half Independent', exact: true })).toBeVisible();
  await page.getByRole('spinbutton', { name: 'Rows keys', exact: true }).fill('3');
  await page.getByRole('spinbutton', { name: 'Rows keys', exact: true }).blur();
  await expect(page.locator('.wb-scene-part')).toHaveCount(30);
  await page.getByRole('treeitem', { name: 'Right half Independent', exact: true }).click();
  await expect(page.getByRole('spinbutton', { name: 'Rows keys', exact: true })).toHaveValue('2');
});

test('light setup fits a narrow viewport and retains keyboard cancellation', async ({ page }, testInfo) => {
  await page.addInitScript(() => localStorage.setItem('boardstudio:v2:theme', 'light'));
  await page.setViewportSize({ width: 390, height: 844 });
  await newProject(page);
  const setup = await openSetup(page);
  const bounds = (await setup.boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(390);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: testInfo.outputPath('light-narrow-pair-setup.png'), animations: 'disabled' });
  await setup.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(setup).toHaveCount(0);
  await expect(page.locator('.wb-topbar').getByRole('button', { name: 'Objects', exact: true })).toBeFocused();
});


test('linked geometry excludes the moving counterpart from alignment references', async ({ page }) => {
  await newProject(page);
  await createPair(page);
  await page.getByRole('treeitem', { name: 'Left half Linked', exact: true }).click();
  await page.getByRole('button', { name: 'Align', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Alignment reference', exact: true }).locator('option:not([value=""])')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Center X', exact: true })).toBeDisabled();
});
