import { chooseScope } from './selection';
import { configureMatrix } from './matrix-setup';
import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { unzipSync } from 'fflate';
import { buildCase } from '@boardstudio/v2-cad';
import { prepareCase } from '../../cad/test/native-prepare.mjs';
import { downloadDraftBoard } from './pcb-package';
import { openCustomSwitchProject } from './custom-switch';

const placeGuidedMatrix = async (page: import('@playwright/test').Page) => {
  await configureMatrix(page);
  const ghost = page.getByRole('button', { name: 'Ghost key, row 1, column 1' });
  await expect(ghost).toBeVisible();
  await ghost.click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(60);
};

test('opens the starter workbench and exports a draft KiCad handoff', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Add object', exact: true })).toBeVisible();
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();

  const result = await downloadDraftBoard(page, { reviewExistingConnections: true });
  expect(result.boardName).toBe('Main_board.kicad_pcb');
  expect(result.filename).toBe('Starter keyboard-draft-pcb-handoff.zip');
});

test('reopens offline after the app shell is cached', async ({ page, context }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Add object', exact: true })).toBeVisible();
  await page.evaluate(() => navigator.serviceWorker.ready);
  await context.setOffline(true);
  await page.reload();
  await expect(page.getByRole('button', { name: 'Add object', exact: true })).toBeVisible();
});

test('updates the outline during a part drag and keeps one undo step', async ({ page }) => {
  await page.goto('/');
  const part = page.getByRole('button', { name: /^SW5, MX switch/ });
  const outline = page.locator('.wb-outline-shape').first();

  await expect(outline).toBeVisible();
  const initial = await outline.getAttribute('points');
  const box = await part.boundingBox();
  expect(box).not.toBeNull();
  const x = box!.x + box!.width / 2;
  const y = box!.y + box!.height / 2;

  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + 80, y, { steps: 8 });
  await expect.poll(() => outline.getAttribute('points')).not.toBe(initial);
  await page.mouse.up();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '1');

  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(outline).toHaveAttribute('points', initial!);
});

test('moves a focused part with arrow keys and keeps each nudge undoable', async ({ page }) => {
  await page.goto('/');
  const part = page.getByRole('button', { name: /^SW1, MX switch/ });

  await part.focus();
  await page.keyboard.press('ArrowRight');
  await expect(part).toHaveAttribute('aria-label', /X 0\.1 Y 0/);
  await expect(page.getByRole('status', { name: 'Keyboard movement' })).toContainText('SW1 moved right 0.1 mm');

  await page.keyboard.press('Shift+ArrowUp');
  await expect(part).toHaveAttribute('aria-label', /X 0\.1 Y 1/);

  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(part).toHaveAttribute('aria-label', /X 0\.1 Y 0/);
  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(part).toHaveAttribute('aria-label', /X 0 Y 0/);
});

test('previews and exports the case assembly as STEP', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Case', exact: true }).click();
  await expect(page.getByLabel('Complete PCB assembly preview')).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Generate', exact: true }).click();
  await expect(page.getByText('Preview current')).toBeVisible();
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();

  const download = page.waitForEvent('download');
  await page.locator('.wb-export-row').filter({ hasText: 'Case STEP' }).getByRole('button', { name: 'Export' }).click();
  expect((await download).suggestedFilename()).toBe('Starter keyboard-case.step');
});

test('packages a local model with relative KiCad paths', async ({ page }) => {
  await openCustomSwitchProject(page);
  await page.getByRole('tab', { name: 'Parts' }).click();
  await page.getByRole('option', { name: 'Fixture switch', exact: true }).click();
  await page.locator('.wb-inspector-section > summary').filter({ hasText: '3D model' }).click();
  await page.locator('.wb-model-import input[type=file]').setInputFiles({
    name: 'sample.step',
    mimeType: 'model/step',
    buffer: Buffer.from('ISO-10303-21;\nEND-ISO-10303-21;\n'),
  });
  await expect(page.getByText('Bound asset: sample.step')).toBeVisible();
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();

  const { files } = await downloadDraftBoard(page, { reviewExistingConnections: true });
  expect(Object.keys(files)).toContain('Main_board.kicad_pcb');
  expect(Object.keys(files).some((path) => /^models\/[a-f0-9]{64}\.step$/u.test(path))).toBe(true);

  const footprintsDownload = page.waitForEvent('download');
  await page.locator('.wb-export-row').filter({ hasText: 'KiCad footprints' }).getByRole('button', { name: 'Export' }).click();
  const footprintArchive = unzipSync(new Uint8Array(await readFile(await (await footprintsDownload).path())));
  expect(Object.keys(footprintArchive)).toContain('BoardStudio.pretty/Fixture_switch.kicad_mod');
  expect(Object.keys(footprintArchive)).toContain('fp-lib-table');
  expect(Object.keys(footprintArchive).some((path) => /^models\/[a-f0-9]{64}\.step$/u.test(path))).toBe(true);
});

test('runs a bounded script to add an outline hole', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add object', exact: true }).click();
  await page.getByRole('button', { name: 'Geometry scripts…', exact: true }).click();
  await page.getByRole('button', { name: '+ New script' }).click();
  await page.getByRole('textbox', { name: 'Rhai source' }).fill('rect("extra", "hole", 30.0, -10.0, 3.0, 3.0, 0.0, "subtract");');
  await page.getByRole('checkbox', { name: 'Enable on Apply' }).check();
  await page.getByRole('button', { name: /Apply script/ }).click();
  await page.getByRole('tab', { name: 'Design' }).click();

  await expect(page.locator('.wb-outline-shape.is-hole')).toHaveCount(1);
});

test('builds a new keyboard from a matrix and previews its outline', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await expect(page.getByRole('treeitem', { name: /0 parts/ }).first()).toBeVisible();
  await configureMatrix(page);
  await expect(page.locator('.wb-matrix-cell')).toHaveCount(30);
  await expect(page.getByRole('button', { name: /^Ghost key, row 1, column 1/ })).toBeVisible();
  await page.getByRole('button', { name: 'Ghost key, row 1, column 1' }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(60);
  await expect(page.getByRole('treeitem', { name: /60 parts/ }).first()).toBeVisible();
  await expect(page.getByRole('treeitem', { name: /^Matrix 1/ })).toBeVisible();
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
});

test('edits a guided matrix, scopes the tree, staggers a row, and zooms the canvas', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await placeGuidedMatrix(page);
  await page.getByRole('treeitem', { name: 'PCB', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Pin for Scan row 1', exact: true })).toBeVisible();
  await expect(page.getByRole('combobox', { name: 'Pin for Scan column 1', exact: true })).toBeVisible();
  await page.getByRole('tab', { name: 'Design' }).click();

  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await page.getByRole('combobox', { name: 'Tree grouping' }).selectOption('row');
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Expand Row 1' }).click();
  const row = page.getByRole('treeitem', { name: /^Row 1/ });
  await row.click();
  await expect(page.locator('.wb-scope-outline.is-selected')).toHaveAttribute('data-scope', 'row');
  const firstCell = page.getByRole('button', { name: 'Select key, row 1, column 1' });
  const before = await firstCell.getAttribute('transform');
  await expect(page.locator('.wb-stagger-handle').first()).toBeHidden();
  const offset = page.getByRole('spinbutton', { name: 'Offset X mm', exact: true });
  await offset.fill('4');
  await offset.blur();
  await expect.poll(() => firstCell.getAttribute('transform')).not.toBe(before);

  await page.getByRole('treeitem', { name: /^Key 1\.1/ }).click();
  await page.getByRole('combobox', { name: 'Key Assembly' }).selectOption('ergogen:ceoloide/switch_choc_v1_v2');
  await expect(page.getByRole('combobox', { name: 'Key Assembly' })).toHaveValue('ergogen:ceoloide/switch_choc_v1_v2');
  await chooseScope(page, 'matrix');
  await page.locator('summary').filter({ hasText: 'Key assembly' }).click();
  await page.getByRole('combobox', { name: 'Apply matrix preset' }).selectOption('mx-hotswap-rgb');
  await page.getByRole('button', { name: 'Update assembly preset' }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(90);

  const canvas = page.getByRole('application', { name: /Board layout canvas/ });
  const zoom = page.locator('.wb-footer-zoom span');
  await canvas.hover();
  await page.mouse.wheel(0, -360);
  await expect(zoom).not.toHaveText('100%');
  await page.getByRole('button', { name: 'Fit board' }).click();
  await expect(zoom).toHaveText(/\d+%/);
});

test('canvas drags follow the selected matrix, row, and column scope', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await placeGuidedMatrix(page);

  const drag = async (part: import('@playwright/test').Locator) => {
    const box = await part.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.down();
    await page.mouse.move(box!.x + box!.width / 2 + 24, box!.y + box!.height / 2, { steps: 3 });
    await page.mouse.up();
  };

  const switch1 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(0);
  const switch2 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(1);
  const switch6 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(5);

  const switch2Before = await switch2.getAttribute('aria-label');
  await chooseScope(page, 'matrix');
  await drag(switch1);
  await expect(switch2).not.toHaveAttribute('aria-label', switch2Before!);

  await chooseScope(page, 'row');
  const switch2RowBefore = await switch2.getAttribute('aria-label');
  await drag(switch1);
  await expect(switch2).not.toHaveAttribute('aria-label', switch2RowBefore!);

  await chooseScope(page, 'column');
  const switch6Before = await switch6.getAttribute('aria-label');
  await drag(switch6);
  await expect(switch6).not.toHaveAttribute('aria-label', switch6Before!);
});

test('row and column scope drags follow the key under the pointer', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await placeGuidedMatrix(page);

  const drag = async (part: import('@playwright/test').Locator) => {
    const box = await part.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.down();
    await page.mouse.move(box!.x + box!.width / 2 + 24, box!.y + box!.height / 2, { steps: 3 });
    await page.mouse.up();
  };

  const switch2 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(1);
  const switch16 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(15);
  const switch17 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(16);
  const switch3 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(2);
  const switch8 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(7);
  const row1Before = await switch2.getAttribute('aria-label');

  await chooseScope(page, 'row');
  const row4Before = await switch17.getAttribute('aria-label');
  await drag(switch16);
  await expect(switch17).not.toHaveAttribute('aria-label', row4Before!);
  await expect(switch2).toHaveAttribute('aria-label', row1Before!);
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await page.getByRole('combobox', { name: 'Tree grouping' }).selectOption('row');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('treeitem', { name: /^Row 4/ })).toHaveAttribute('aria-selected', 'true');

  await chooseScope(page, 'column');
  const column3Before = await switch8.getAttribute('aria-label');
  await drag(switch3);
  await expect(switch8).not.toHaveAttribute('aria-label', column3Before!);
  await expect(switch2).toHaveAttribute('aria-label', row1Before!);
});

test('matrix scope drag follows world direction on a rotated mirrored matrix', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await placeGuidedMatrix(page);
  await page.getByRole('spinbutton', { name: 'Rotation' }).fill('90');
  await page.getByRole('spinbutton', { name: 'Rotation' }).blur();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '2');
  await page.getByRole('combobox', { name: 'Mirror matrix' }).selectOption('x');
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '3');

  const switch1 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(0);
  const switch2 = page.getByRole('button', { name: /^SW\d+, switch mx,/ }).nth(1);
  const before = await switch2.getAttribute('aria-label');
  const beforeX = Number(before?.match(/X (-?[\d.]+)/)?.[1]);
  const box = await switch1.boundingBox();
  expect(box).not.toBeNull();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
  await page.mouse.down();
  await page.mouse.move(box!.x + box!.width / 2 + 24, box!.y + box!.height / 2, { steps: 3 });
  await page.mouse.up();
  await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', '4');

  const after = await switch2.getAttribute('aria-label');
  const afterX = Number(after?.match(/X (-?[\d.]+)/)?.[1]);
  expect(afterX).toBeGreaterThan(beforeX);
});

test('matrix rows and columns are nested beneath their matrix in the CAD tree', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await placeGuidedMatrix(page);
  const matrix = page.getByRole('treeitem', { name: /^Matrix 1/ });
  const row = page.getByRole('treeitem', { name: /^Row 1/ });
  const column = page.getByRole('treeitem', { name: /^Column 1/ });
  await expect(matrix).toHaveAttribute('aria-level', '3');
  await expect(column).toHaveAttribute('aria-level', '4');
  const matrixPadding = Number.parseFloat((await matrix.evaluate((element) => getComputedStyle(element.parentElement!).paddingLeft)));
  const columnPadding = Number.parseFloat((await column.evaluate((element) => getComputedStyle(element.parentElement!).paddingLeft)));
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await page.getByRole('combobox', { name: 'Tree grouping' }).selectOption('row');
  await page.keyboard.press('Escape');
  await expect(row).toHaveAttribute('aria-level', '4');
  const rowPadding = Number.parseFloat((await row.evaluate((element) => getComputedStyle(element.parentElement!).paddingLeft)));
  expect(rowPadding).toBeGreaterThan(matrixPadding);
  expect(columnPadding).toBe(rowPadding);
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await page.getByRole('combobox', { name: 'Tree grouping' }).selectOption('column');
  await page.keyboard.press('Escape');
  await column.click();
  await expect(column).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('button', { name: 'Select: Column', exact: true })).toBeVisible();
});

test('matrix diode direction is editable and survives matrix edits', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('treeitem', { name: /^Matrix 1/ }).click();

  await page.locator('summary').filter({ hasText: 'Key assembly' }).click();
  const direction = page.getByRole('combobox', { name: 'Diode direction' });
  await expect(direction).toHaveValue('row2col');
  await direction.selectOption('col2row');
  await expect(direction).toHaveValue('col2row');
  await page.getByRole('combobox', { name: 'Apply matrix preset' }).selectOption('choc-solder');
  await page.getByRole('button', { name: 'Update assembly preset' }).click();
  await expect(direction).toHaveValue('col2row');
});

test('snaps part drags unless Alt is held and pans with Space-drag', async ({ page }) => {
  await page.goto('/');
  const firstPart = page.getByRole('button', { name: /^SW1, MX switch/ });
  const dragThreeMm = async (alt: boolean) => {
    const revision = await page.locator('.wb-root').getAttribute('data-revision');
    const box = await firstPart.boundingBox();
    expect(box).not.toBeNull();
    const dx = await page.evaluate(() => {
      const svg = document.querySelector<SVGSVGElement>('.wb-canvas')!;
      return 3 / svg.getScreenCTM()!.inverse().a;
    });
    const x = box!.x + box!.width / 2;
    const y = box!.y + box!.height / 2;
    await page.mouse.move(x, y);
    await page.mouse.down();
    if (alt) await page.keyboard.down('Alt');
    await page.mouse.move(x + dx, y, { steps: 3 });
    await page.mouse.up();
    if (alt) await page.keyboard.up('Alt');
    await expect(page.locator('.wb-root')).not.toHaveAttribute('data-revision', revision!);
  };

  await dragThreeMm(false);
  await expect(firstPart).toHaveAttribute('aria-label', /X 4\.7625 Y 0/);
  await dragThreeMm(true);
  await expect(firstPart).toHaveAttribute('aria-label', /X 7\.762[0-9]* Y 0/);

  const canvas = page.getByRole('application', { name: /Board layout canvas/ });
  const initialView = await canvas.getAttribute('viewBox');
  const bounds = await canvas.boundingBox();
  expect(bounds).not.toBeNull();
  await page.keyboard.down('Space');
  await page.mouse.move(bounds!.x + bounds!.width / 2, bounds!.y + bounds!.height / 2);
  await page.mouse.down();
  await page.mouse.move(bounds!.x + bounds!.width / 2 + 40, bounds!.y + bounds!.height / 2 + 20, { steps: 3 });
  await page.mouse.up();
  await page.keyboard.up('Space');
  await expect.poll(() => canvas.getAttribute('viewBox')).not.toBe(initialView);
  await page.getByRole('button', { name: 'Fit board' }).click();
  const fittedOutline = await page.locator('.wb-outline-shape').boundingBox();
  const fittedCanvas = await canvas.boundingBox();
  expect(fittedOutline).not.toBeNull();
  expect(fittedCanvas).not.toBeNull();
  expect(fittedOutline!.x).toBeGreaterThanOrEqual(fittedCanvas!.x);
  expect(fittedOutline!.y).toBeGreaterThanOrEqual(fittedCanvas!.y);
  expect(fittedOutline!.x + fittedOutline!.width).toBeLessThanOrEqual(fittedCanvas!.x + fittedCanvas!.width);
  expect(fittedOutline!.y + fittedOutline!.height).toBeLessThanOrEqual(fittedCanvas!.y + fittedCanvas!.height);
});

test('previews a library footprint and keeps generator geometry in sync', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('tab', { name: 'Parts' }).click();
  await page.getByRole('listbox', { name: 'Footprint library' }).getByRole('option', { name: 'MX switch', exact: true }).click();
  const preview = page.getByRole('region', { name: 'Parts canvas' }).getByRole('img', { name: 'Footprint preview' });
  await expect(preview).toBeVisible();
  const before = await preview.innerHTML();
  await page.getByRole('checkbox', { name: 'reversible', exact: true }).check();
  await expect(page.getByRole('checkbox', { name: 'reversible', exact: true })).toBeChecked();
  await expect.poll(() => preview.innerHTML()).not.toBe(before);
  await page.getByRole('button', { name: 'Apply generator settings' }).click();
  await expect(page.getByRole('checkbox', { name: 'reversible', exact: true })).toBeChecked();
});

test('duplicates a matrix as a separate preset project', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await placeGuidedMatrix(page);
  await expect(page.getByText('Saved locally')).toBeVisible();
  const originalId = await page.evaluate(() => localStorage.getItem('boardstudio-v2-active-project'));
  expect(originalId).toBeTruthy();

  await page.locator('summary').filter({ hasText: 'Key assembly' }).click();
  await page.getByRole('combobox', { name: 'Apply matrix preset' }).selectOption('choc-rgb');
  await page.getByRole('button', { name: 'Duplicate design as variant' }).click();
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await expect(page.getByRole('textbox', { name: 'Project name' })).toHaveValue(/choc-rgb/);
  await expect(page.locator('.wb-scene-part')).toHaveCount(90);
  const variantId = await page.evaluate(() => localStorage.getItem('boardstudio-v2-active-project'));
  expect(variantId).not.toBe(originalId);

  await page.reload();
  await expect(page.locator('.wb-scene-part')).toHaveCount(90);
  const originalParts = await page.evaluate(async (id) => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('boardstudio-v2', 1);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    return await new Promise<number>((resolve, reject) => {
      const request = db.transaction('projects').objectStore('projects').get(id);
      request.onsuccess = () => { db.close(); resolve(request.result.parts.length); };
      request.onerror = () => { db.close(); reject(request.error); };
    });
  }, originalId);
  expect(originalParts).toBe(60);
});

test('keeps board outlines and KiCad exports scoped to the selected board', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await placeGuidedMatrix(page);
  await expect(page.getByRole('treeitem', { name: /60 parts/ }).first()).toBeVisible();

  await page.getByRole('button', { name: 'New board', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Selected board' })).toHaveValue(/.+/);
  await expect(page.getByRole('treeitem', { name: /0 parts/ }).first()).toBeVisible();
  await expect(page.locator('.wb-outline-shape')).toHaveCount(0);

  await placeGuidedMatrix(page);
  await expect(page.getByRole('treeitem', { name: /60 parts/ }).first()).toBeVisible();
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const result = await downloadDraftBoard(page);
  expect(result.boardName).toBe('Board_2.kicad_pcb');

  await page.getByRole('combobox', { name: 'Selected board' }).selectOption({ label: 'Main board' });
  await expect(page.getByRole('treeitem', { name: /60 parts/ }).first()).toBeVisible();
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
});

test('creates a case from a new keyboard project', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await placeGuidedMatrix(page);
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
  await page.getByRole('treeitem', { name: 'Case', exact: true }).click();
  await page.getByRole('button', { name: '+ New case body' }).click();
  await page.getByRole('button', { name: 'Generate', exact: true }).click();
  await expect(page.getByText('Preview current')).toBeVisible({ timeout: 30_000 });
});

test('saves and reopens a v2 project archive', async ({ page }) => {
  await page.goto('/');
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: /Save .boardstudio project/ }).click();
  const archive = await download;

  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await expect(page.getByRole('treeitem', { name: /0 parts/ }).first()).toBeVisible();
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.locator('.wb-project-file-input').setInputFiles(await archive.path());
  await expect(page.getByRole('treeitem', { name: /15 parts/ }).first()).toBeVisible();
  await expect(page.locator('.wb-outline-shape')).toHaveCount(1);
});

test('authors a custom component and exports a KiCad footprint library', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project' }).click();
  await page.getByRole('tab', { name: 'Parts' }).click();
  await page.getByRole('button', { name: 'New custom component' }).click();
  const editor = page.getByRole('complementary', { name: 'Parts inspector' });
  await expect(editor.getByRole('heading', { level: 2, name: 'Custom component 1', exact: true })).toBeVisible();
  await page.getByRole('textbox', { name: 'Definition name' }).fill('Controller mount');
  await page.getByRole('textbox', { name: 'Definition name' }).blur();
  await expect(editor.getByRole('heading', { level: 2, name: 'Controller mount', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '+ Add pad' }).click();
  await expect(editor.locator('details').filter({ has: page.getByRole('group', { name: 'Pad 1', includeHidden: true }) })).not.toHaveAttribute('open', '');
  await editor.locator('summary').filter({ hasText: 'Edit footprint' }).click();
  await expect(page.getByRole('group', { name: 'Pad 1' })).toBeVisible();
  await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();

  const download = page.waitForEvent('download');
  await page.locator('.wb-export-row').filter({ hasText: 'KiCad footprints' }).getByRole('button', { name: 'Export' }).click();
  const archive = unzipSync(new Uint8Array(await readFile(await (await download).path())));
  expect(Object.keys(archive)).toContain('BoardStudio.pretty/Controller_mount.kicad_mod');
});

test('shows attached STEP components in the 3D case preview', async ({ page }) => {
  test.setTimeout(60_000);
  const model = await buildCase(prepareCase({
    revision: 0,
    body: { id: 'preview-model', name: 'Preview model', boardId: 'main-board', kind: 'plate', thickness: 2, clearance: 0 },
    contours: [{ hole: false, points: [{ x: -3, y: -3 }, { x: 3, y: -3 }, { x: 3, y: 3 }, { x: -3, y: 3 }] }],
  }));

  await openCustomSwitchProject(page);
  await page.getByRole('tab', { name: 'Parts' }).click();
  await page.getByRole('option', { name: 'Fixture switch', exact: true }).click();
  await page.locator('.wb-inspector-section > summary').filter({ hasText: '3D model' }).click();
  await page.locator('.wb-model-import input[type=file]').setInputFiles({
    name: 'preview.step',
    mimeType: 'model/step',
    buffer: Buffer.from(model.step),
  });
  await expect(page.getByText('Bound asset: preview.step')).toBeVisible();
  await page.getByRole('tab', { name: 'Design', exact: true }).click();
  await page.getByRole('treeitem', { name: 'Case', exact: true }).click();
  await expect(page.getByText('15 / 15 models · 1.6 mm PCB', { exact: true })).toBeVisible({ timeout: 30_000 });
});

test('propagates a linked layout constraint during a source move', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /^SW2, MX switch/ }).click();
  await chooseScope(page, 'component');
  await page.locator('summary').filter({ hasText: 'Layout constraint' }).click();
  await page.getByRole('combobox', { name: 'Constraint source part' }).selectOption({ label: 'SW1' });
  await page.getByRole('spinbutton', { name: 'Offset X (mm)' }).fill('25');
  await page.getByRole('button', { name: 'Add constraint' }).click();
  await expect(page.getByRole('button', { name: /^SW2, MX switch/ })).toHaveAttribute('aria-label', /X 25 Y 0/);

  await page.getByRole('button', { name: /^SW1, MX switch/ }).click();
  await page.getByRole('spinbutton', { name: 'X mm', exact: true }).fill('10');
  await page.getByRole('spinbutton', { name: 'X mm', exact: true }).blur();
  await expect(page.getByRole('button', { name: /^SW2, MX switch/ })).toHaveAttribute('aria-label', /X 35 Y 0/);
});
