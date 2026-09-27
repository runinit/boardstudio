import { expect, test, type Locator } from '@playwright/test';

test('sidebars resize, collapse, restore and retain preferences without changing selection', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  await page.getByRole('treeitem', { name: 'Column 1 3 keys', exact: true }).click();
  const canvas = page.locator('.wb-canvas-column');
  const initial = (await canvas.boundingBox())!.width;
  const objects = page.locator('#wb-inventory');
  const inspector = page.locator('#wb-inspector');
  const originalWidth = (await objects.boundingBox())!.width;
  const leftResize = page.getByRole('separator', { name: 'Resize objects', exact: true });
  await leftResize.focus();
  await page.keyboard.press('ArrowRight');
  expect((await objects.boundingBox())!.width).toBeCloseTo(originalWidth + 20, 0);
  await expect(leftResize).toHaveAttribute('aria-valuenow', String(Math.round(originalWidth + 20)));
  const rightResize = page.getByRole('separator', { name: 'Resize inspector', exact: true });
  const rightWidth = (await inspector.boundingBox())!.width;
  const handle = (await rightResize.boundingBox())!;
  await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
  await page.mouse.down();
  await page.mouse.move(handle.x - 35, handle.y + handle.height / 2, { steps: 3 });
  await page.mouse.up();
  expect((await inspector.boundingBox())!.width).toBeGreaterThan(rightWidth + 25);
  await expect(rightResize).toHaveAttribute('aria-valuenow', String(Math.round((await inspector.boundingBox())!.width)));
  await page.getByRole('button', { name: 'Inspector options', exact: true }).click();
  await page.getByRole('button', { name: 'Collapse inspector', exact: true }).click();
  await expect(inspector).toHaveAttribute('aria-hidden', 'true');
  expect((await canvas.boundingBox())!.width).toBeGreaterThan(initial + 200);
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await page.getByRole('button', { name: 'Collapse objects', exact: true }).click();
  await expect(objects).toHaveAttribute('aria-hidden', 'true');
  expect((await canvas.boundingBox())!.width).toBeGreaterThan(1300);
  const inspectorRail = page.getByRole('button', { name: 'Show inspector', exact: true });
  expect((await inspectorRail.boundingBox())!.height).toBeGreaterThan(90);
  await inspectorRail.hover();
  await expect(inspector).toHaveAttribute('aria-hidden', 'false');
  await canvas.hover();
  await expect(inspector).toHaveAttribute('aria-hidden', 'true');
  await page.getByRole('button', { name: 'Show inspector', exact: true }).click();
  await expect(page.getByRole('spinbutton', { name: 'Splay °', exact: true })).toBeVisible();
  await page.reload();
  await expect(objects).toHaveAttribute('aria-hidden', 'true');
  await page.getByRole('button', { name: 'Show objects', exact: true }).click();
  expect((await objects.boundingBox())!.width).toBeCloseTo(originalWidth + 20, 0);
});

test('project actions stay focused and layout tools float over the canvas', async ({ page }) => {
  await page.setViewportSize({ width: 1080, height: 900 });
  await page.goto('/');
  const toolbar = page.getByRole('toolbar', { name: 'Layout commands' });
  const stage = page.locator('.wb-canvas-stage');
  const toolbarBox = (await toolbar.boundingBox())!;
  const stageBox = (await stage.boundingBox())!;
  expect(toolbarBox.y).toBeGreaterThan(stageBox.y);
  expect(toolbarBox.width).toBeLessThan(stageBox.width);
  await expect(page.locator('.wb-stagger-handle').first()).toBeHidden();
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  const project = page.locator('#wb-project-dropdown');
  await expect(project.getByRole('textbox', { name: 'Project name' })).toBeVisible();
  await expect(project.getByRole('button', { name: 'Open project…' })).toBeVisible();
  await expect(project.getByRole('button', { name: 'Geometry scripts…' })).toHaveCount(0);
  await expect(project.getByRole('combobox', { name: 'Selected board' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Project', exact: true }).click();
  await toolbar.getByRole('button', { name: 'Snap' }).click();
  await expect(page.getByRole('dialog', { name: 'Snap' })).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(toolbar).toBeVisible();
  expect((await toolbar.boundingBox())!.width).toBeGreaterThan(350);
});

test('auto-hide panels reveal on demand and protect focused edits', async ({ page }) => {
  await page.goto('/');
  const inspector = page.locator('#wb-inspector');
  await page.getByRole('button', { name: 'Inspector options', exact: true }).click();
  await page.getByRole('button', { name: 'Auto-hide inspector', exact: true }).click();
  const canvas = page.locator('.wb-canvas');
  await canvas.focus();
  await canvas.hover();
  await expect(inspector).toHaveAttribute('aria-hidden', 'true');
  const reveal = page.getByRole('button', { name: 'Show inspector', exact: true });
  await reveal.hover();
  await expect(inspector).toHaveAttribute('aria-hidden', 'false');
  await page.getByRole('textbox', { name: 'Board name', exact: true }).focus();
  await canvas.hover();
  await page.waitForTimeout(400);
  await expect(inspector).toHaveAttribute('aria-hidden', 'false');
  await page.keyboard.press('Escape');
  await expect(inspector).toHaveAttribute('aria-hidden', 'true');
  await expect(reveal).toBeFocused();
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  await page.getByRole('button', { name: 'Auto-hide objects', exact: true }).click();
  await canvas.focus();
  await canvas.hover();
  await expect(page.locator('#wb-inventory')).toHaveAttribute('aria-hidden', 'true');
  await page.reload();
  await expect(page.getByRole('button', { name: 'Show objects', exact: true })).toBeVisible();
  await expect(reveal).toBeVisible();
});

for (const theme of ['dark', 'light'] as const) {
  test(`${theme} sectioned Add and compact drawers fit the workspace`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem('boardstudio:v2:theme', value), theme);
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto('/');
    await page.getByRole('treeitem', { name: 'Column 1 3 keys', exact: true }).click();
    await expect(page.locator('.wb-workspace-navigation')).toHaveCount(0);
    await page.getByRole('button', { name: 'Add object', exact: true }).click();
    const add = page.getByRole('dialog', { name: 'Add', exact: true });
    await expectCompactAddControls(add);
    await expect(add.getByRole('heading', { name: 'Layouts', exact: true })).toBeVisible();
    await expect(add.getByRole('heading', { name: 'Parts', exact: true })).toBeVisible();
    await expect(add.getByRole('heading', { name: 'Board geometry', exact: true })).toBeVisible();
    await page.keyboard.press('Escape');
    await page.locator('.wb-topbar').getByRole('button', { name: 'Export', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Export package', exact: true })).toBeVisible();
    await page.getByRole('tab', { name: 'Design', exact: true }).click();
    await page.setViewportSize({ width: 390, height: 844 });
    const close = page.getByRole('button', { name: 'Close panels', exact: true });
    if (await close.isVisible()) await close.click({ position: { x: 2, y: 100 } });
    await page.locator('.wb-topbar').getByRole('button', { name: 'Objects', exact: true }).click();
    await expect(page.locator('#wb-inventory')).toHaveAttribute('aria-hidden', 'false');
    await page.getByRole('button', { name: 'Inspect', exact: true }).click();
    await expect(page.locator('#wb-inventory')).toHaveAttribute('aria-hidden', 'true');
    await expect(page.locator('#wb-inspector')).toHaveAttribute('aria-hidden', 'false');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await close.click({ position: { x: 2, y: 100 } });
    await page.locator('.wb-topbar').getByRole('button', { name: 'Objects', exact: true }).click();
    await page.getByRole('button', { name: 'Add object', exact: true }).click();
    await expect(add).toBeVisible();
    const bounds = (await add.boundingBox())!;
    expect(bounds.x).toBeGreaterThanOrEqual(0);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(390);
    await expectCompactAddControls(add);
    if (theme === 'dark') {
      for (const width of [1440, 600]) {
        await page.keyboard.press('Escape');
        await page.setViewportSize({ width, height: 1000 });
        if (width < 700) {
          const objects = page.locator('.wb-topbar').getByRole('button', { name: 'Objects', exact: true });
          if (await objects.getAttribute('aria-expanded') === 'false') await objects.click();
        }
        await page.getByRole('button', { name: 'Add object', exact: true }).click();
        await expect(add).toBeVisible();
        await expectCompactAddControls(add);
      }
    }
  });
}

 test('panel menu actions fill their menu instead of inheriting icon dimensions', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Objects options', exact: true }).click();
  const menu = page.locator('#wb-objects-options');
  const action = page.getByRole('button', { name: 'Auto-hide objects', exact: true });
  expect((await action.boundingBox())!.width).toBeGreaterThan((await menu.boundingBox())!.width - 20);
});

async function expectCompactAddControls(panel: Locator) {
  const sizes = await panel.locator('button svg').evaluateAll((icons) => icons.map((icon) => ({ width: icon.getBoundingClientRect().width, height: icon.getBoundingClientRect().height })));
  expect(sizes.length).toBeGreaterThan(4);
  for (const size of sizes) {
    expect(size.width).toBeGreaterThan(0);
    expect(size.width).toBeLessThanOrEqual(20);
    expect(size.height).toBeLessThanOrEqual(20);
  }
  for (const name of ['Matrix…', 'Add row', 'Add column', 'Browse all parts']) {
    const button = panel.getByRole('button', { name, exact: true });
    expect((await button.boundingBox())!.height).toBeLessThanOrEqual(80);
  }
}
