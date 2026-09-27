import { expect, test, type Page } from '@playwright/test';
import type { ProjectDoc } from '@boardstudio/v2-contracts';
async function start(page: Page, matrix = false) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  if (matrix) {
    await page.getByRole('button', { name: 'Continue to layout' }).click();
    await page.getByRole('button', { name: 'Add key matrix' }).click();
    await page.getByLabel('New matrix rows').fill('2');
    await page.getByLabel('New matrix columns').fill('3');
  }
}
async function place(page: Page) {
  await page.locator('svg.wb-canvas').click({ position: { x: 250, y: 200 } });
}
test('matrix placement and cancellation return to the guide', async ({ page }) => {
  await start(page, true);
  await page.getByRole('button', { name: 'Create matrix' }).click();
  await expect(page.getByRole('region', { name: 'Project setup guide' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Continue to wiring' })).toBeVisible();
  await page.getByRole('button', { name: 'Add key matrix' }).click();
  await page.getByRole('form', { name: 'New matrix' }).getByRole('button', { name: 'Cancel' }).click();
  await expect(page.getByRole('region', { name: 'Project setup guide' })).toBeVisible();
});
test('project reversible layout applies to matrices at the origin', async ({ page }) => {
  await start(page);
  await page.getByRole('button', { name: 'Split keyboard', exact: true }).click();
  await page.getByLabel('Reversible layout', { exact: true }).click();
  await expect(page.getByLabel('Reversible layout', { exact: true })).toBeChecked();
  await page.getByRole('button', { name: 'Continue to layout' }).click();
  await page.getByRole('button', { name: 'Add key matrix' }).click();
  await page.getByLabel('New matrix rows').fill('2');
  await page.getByLabel('New matrix columns').fill('3');
  await page.getByLabel('New matrix assembly').selectOption('mx-hotswap-rgb');
  await expect(page.getByRole('img', { name: '2 rows by 3 columns matrix preview' })).toBeVisible();
  await expect(page.getByLabel('Selected board')).toHaveCount(0);
  await page.getByRole('button', { name: 'Create matrix' }).click();
  await page.getByRole('button', { name: 'Skip guide' }).click();
  await page.getByRole('treeitem', { name: 'Matrix 1 6 keys', exact: true }).click();
  await expect(page.getByRole('spinbutton', { name: 'Origin X mm', exact: true })).toHaveValue('0');
  await expect(page.getByRole('spinbutton', { name: 'Origin Y mm', exact: true })).toHaveValue('0');
  await page.getByText('Key assembly', { exact: true }).click();
  await expect(page.getByLabel('Apply matrix preset')).toHaveValue('mx-hotswap-rgb');
});
test('empty wiring offers a supported controller and resolves a 35-key matrix', async ({ page }) => {
  await start(page, true);
  await page.getByLabel('New matrix rows').fill('5');
  await page.getByLabel('New matrix columns').fill('7');
  await page.getByRole('button', { name: 'Create matrix' }).click();
  await page.getByRole('button', { name: /Controller & wiring/ }).click();
  await expect(page.getByLabel('Controller', { exact: true })).toHaveCount(0);
  await page.screenshot({ animations: 'disabled', path: '/tmp/boardstudio-wiring-desktop.png' });
  await page.getByRole('button', { name: 'Add controller', exact: true }).click();
  await expect(page.locator('[role="option"][title="infused-kim/nice_nano_pretty"]')).toHaveCount(0);
  await page.locator('[role="option"][title="ceoloide/mcu_nice_nano"]').click();
  await page.getByRole('button', { name: 'Place component', exact: true }).click();
  await place(page);
  await expect(page.getByRole('region', { name: 'Project setup guide' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Electrical wiring' })).toContainText('mcu nice nano');
  await expect(page.getByRole('button', { name: 'Apply wiring', exact: true })).toBeEnabled();
  await expect(page.getByText('This controller needs a reviewed pin profile', { exact: false })).toHaveCount(0);
  await page.getByRole('button', { name: 'Apply wiring', exact: true }).click();
  await expect(page.locator('.wb-scene-footprint text').filter({ hasText: /^ROW_0$/ })).not.toHaveCount(0);
  await expect(page.locator('.wb-scene-footprint text').filter({ hasText: /^user$/ })).toHaveCount(0);
  await page.locator('.wb-wiring-assignment').first().scrollIntoViewIfNeeded();
  await page.screenshot({ path: '/tmp/boardstudio-wiring-assignments.png', animations: 'disabled' });
});
test('individual key assemblies inherit project reversible construction', async ({ page }) => {
  await start(page);
  await page.getByLabel('Reversible layout', { exact: true }).click();
  await page.getByRole('tab', { name: 'Parts', exact: true }).click();
  await page.getByRole('option', { name: 'MX Hotswap RGB', exact: true }).click();
  await page.getByRole('button', { name: 'Place key assembly', exact: true }).click();
  await page.getByRole('button', { name: 'Ghost key, row 1, column 1', exact: true }).click();
  await expect(page.locator('.wb-scene-part')).toHaveCount(3);
  await expect.poll(() => page.evaluate(() => new Promise<boolean[]>((resolve, reject) => {
    const open = indexedDB.open('boardstudio-v2', 1);
    open.onerror = () => reject(open.error);
    open.onsuccess = () => {
      const db = open.result;
      const request = db.transaction('projects').objectStore('projects').get(localStorage.getItem('boardstudio-v2-active-project')!);
      request.onerror = () => { db.close(); reject(request.error); };
      request.onsuccess = () => {
        const document = request.result as ProjectDoc;
        db.close();
        resolve(document.parts.map(part => document.definitions.find(definition => definition.id === part.definitionId)?.generator?.parameters.reversible === true));
      };
    };
  }))).toEqual([true, true, true]);
});
test('export closes with its button, Escape, and toggle', async ({ page }) => {
  await page.goto('/');
  const toggle = page.getByRole('button', { name: 'Export', exact: true }).first();
  await toggle.click();
  await page.getByRole('button', { name: 'Close export', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Export package' })).toHaveCount(0);
  await toggle.click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('heading', { name: 'Export package' })).toHaveCount(0);
  await toggle.click();
  await toggle.click();
  await expect(page.getByRole('heading', { name: 'Export package' })).toHaveCount(0);
});


test('compact construction and export controls stay reachable', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await start(page, true);
  await page.screenshot({ animations: 'disabled', path: '/tmp/boardstudio-construction-mobile.png' });
  await page.getByRole('form', { name: 'New matrix' }).getByRole('button', { name: 'Cancel' }).click();
  await expect(page.getByRole('region', { name: 'Project setup guide' })).toBeVisible();
  await page.getByRole('button', { name: /Review & export/ }).click();
  await page.getByRole('button', { name: 'Open export options', exact: true }).click();
  const close = page.getByRole('button', { name: 'Close export', exact: true });
  await expect(close).toBeInViewport();
  await page.screenshot({ animations: 'disabled', path: '/tmp/boardstudio-export-mobile.png' });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await close.click();
  await expect(page.getByRole('heading', { name: 'Export package' })).toHaveCount(0);
});

test('physical assembly icons stay editable and wired choices are contextual', async ({ page }) => {
  await start(page);
  await page.getByRole('button', { name: 'One keyboard', exact: true }).click();
  await expect(page.getByRole('group', { name: 'Half connection' })).toHaveCount(0);
  await expect(page.getByLabel('Setup board')).toHaveCount(0);
  await expect(page.getByLabel('PCB design')).toHaveCount(0);
  await expect(page.getByLabel('Share construction dimensions')).toHaveCount(0);
  await page.getByRole('button', { name: 'Split keyboard', exact: true }).click();
  await page.getByRole('button', { name: 'Wired', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Wired', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'One keyboard', exact: true }).click();
  await expect(page.getByRole('group', { name: 'Half connection' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Split keyboard', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await page.reload();
  await expect(page.getByRole('button', { name: 'Wired', exact: true })).toHaveAttribute('aria-pressed', 'true');
});

for (const width of [1129, 390]) {
  test(`compact setup visuals at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1017 });
    await start(page);
    await page.getByRole('button', { name: 'Split keyboard', exact: true }).click();
    await page.getByLabel('Reversible layout', { exact: true }).click();
    await expect(page.getByLabel('Reversible layout', { exact: true })).toBeChecked();
    await page.getByRole('button', { name: 'Continue to layout' }).scrollIntoViewIfNeeded();
    await page.screenshot({ animations: 'disabled', path: `/tmp/boardstudio-setup-${width}.png` });
    await page.getByRole('button', { name: 'Continue to layout' }).click();
    await page.getByRole('button', { name: 'Add key matrix' }).click();
    await page.getByLabel('New matrix rows').fill('5');
    await page.getByLabel('New matrix columns').fill('7');
    await expect(page.getByRole('img', { name: '5 rows by 7 columns matrix preview' })).toBeVisible();
    const rows = await page.getByLabel('New matrix rows').boundingBox();
    const columns = await page.getByLabel('New matrix columns').boundingBox();
    expect(rows!.y).toBe(columns!.y);
    await page.screenshot({ animations: 'disabled', path: `/tmp/boardstudio-matrix-${width}.png` });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });
}

test('reversible parts and wireless case defaults carry through setup', async ({ page }) => {
  await start(page);
  await page.getByRole('button', { name: 'Split keyboard', exact: true }).click();
  await page.getByLabel('Reversible layout', { exact: true }).click();
  await expect(page.getByLabel('Reversible layout', { exact: true })).toBeChecked();
  await page.getByRole('tab', { name: 'Parts', exact: true }).click();
  await page.locator('[role="option"][title="ceoloide/mcu_supermini_nrf52840"]').click();
  await expect(page.getByLabel('reversible', { exact: true })).toBeChecked();
  await page.getByRole('tab', { name: 'Design', exact: true }).click();
  await page.getByRole('button', { name: 'Continue to layout' }).click();
  await page.getByRole('button', { name: 'Add key matrix' }).click();
  await page.getByLabel('New matrix rows').fill('5');
  await page.getByLabel('New matrix columns').fill('7');
  await page.getByRole('button', { name: 'Create matrix' }).click();
  await page.getByRole('button', { name: /Case \(optional\)/ }).click();
  const panel = page.locator('.wb-mechanical-panel');
  await expect(panel.getByRole('group', { name: 'Method', exact: true })).toBeVisible();
  await expect(page.locator('#wb-inspector').getByRole('group', { name: 'Keyboard configuration' })).toHaveCount(0);
  await expect(panel.getByLabel('Board for mechanical stack')).toHaveCount(0);
  await expect(panel.getByText('Advanced source geometry', { exact: true })).toHaveCount(0);
  await expect(panel.getByLabel('Resolved mechanical stack').getByRole('button').filter({ hasText: 'Battery' })).toBeVisible();
  await expect.poll(() => panel.locator('.wb-mech-mount').count()).toBe(4);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(panel.locator('.wb-mech-mount')).toHaveCount(0);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(panel.locator('.wb-mech-mount')).toHaveCount(4);
  await panel.getByText('Stabilizer fit', { exact: true }).click();
  await expect(panel.getByText('No keys in this layout need stabilizers.')).toBeVisible();
  await expect(page.locator('.app-error')).toHaveCount(0);
  await panel.getByRole('group', { name: 'Method', exact: true }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: '/tmp/boardstudio-case-defaults-desktop.png', animations: 'disabled' });
  await page.setViewportSize({ width: 390, height: 844 });
  await panel.getByRole('group', { name: 'Method', exact: true }).scrollIntoViewIfNeeded();
  await expect(panel.getByRole('group', { name: 'Method', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: '/tmp/boardstudio-case-defaults-mobile.png', animations: 'disabled' });
});
