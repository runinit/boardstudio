// Current Graphite feedback acceptance. Requires Playwright on NODE_PATH.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROME_BIN || '/usr/bin/chromium', args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const context = await browser.newContext({ colorScheme: 'dark', viewport: { width: 1513, height: 1043 } });
  const page = await context.newPage();
  const failures = [];
  page.on('pageerror', error => failures.push(error.message));
  const check = (label, value) => { console.log(`${value ? 'PASS' : 'FAIL'} ${label}`); if (!value) failures.push(label); };
  const ready = () => page.waitForFunction(() => {
    const toast = document.querySelector('#__dx-toast');
    if (!toast || toast.getAttribute('aria-hidden') === 'true') return true;
    const box = (toast.querySelector('.dx-toast-inner') || toast).getBoundingClientRect();
    return !box.width || box.right <= 0 || box.x >= innerWidth;
  }, null, { timeout: 180000 });
  const waitValue = async (locator, value) => {
    for (let i = 0; i < 100; i++) { if (Math.abs(Number(await locator.inputValue()) - value) < 0.0001) return; await page.waitForTimeout(50); }
    throw Error('Geometry did not settle');
  };
  const undo = async () => page.getByRole('button', { name: 'Undo', exact: true }).click();
  const nav = async name => { await ready(); await page.getByRole('button', { name: `Open ${name} workspace`, exact: true }).click(); await page.waitForTimeout(300); };
  const shots = process.env.BOARDSTUDIO_SCREENSHOTS;
  if (shots) fs.mkdirSync(shots, { recursive: true });
  try {
    await page.goto(process.env.BOARDSTUDIO_PREVIEW_URL || 'http://127.0.0.1:8182/');
    await ready();
    await page.getByRole('button', { name: 'Start Corne', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Select Column', exact: true }).click({ timeout: 60000 });
    check('workspace navigation is in the Objects dock', await page.locator('#m1-objects-panel .m1-workflow-navigation').count() === 1 && await page.locator('.m1-topbar .m1-workflow-navigation').count() === 0);
    await page.locator('.m1-scene-part[data-part-id^="matrix/"][data-part-id$="r0c2"] .m1-part-hit-area').first().click();
    check('canvas numeric card removed', await page.locator('.m1-canvas-measurements').count() === 0);
    check('board picker removed', await page.locator('#m1-objects-panel select[aria-label="Board"]').count() === 0);
    check('footer grid status removed', await page.locator('.m1-canvas-footer-grid').count() === 0);
    check('key size starts expanded at top', await page.locator('.m1-layout-key-size-section[open]').count() === 1);
    check('key size SVG preview present', await page.locator('.m1-key-size-preview svg').isVisible());
    check('width/height sliders removed', await page.locator('input[aria-label="Key width"],input[aria-label="Key height"]').count() === 0);
    const size = page.locator('.m1-key-size').getByLabel('Key size', { exact: true });
    await size.selectOption('2');
    await page.waitForFunction(() => document.querySelector('.m1-key-size-preview output')?.textContent === '2u × 1u');
    await page.getByLabel('Rotate keycap', { exact: true }).check();
    await page.waitForFunction(() => document.querySelector('.m1-key-size-preview output')?.textContent === '1u × 2u');
    check('size and rotate use accepted key geometry', true);
    await undo(); await page.waitForFunction(() => document.querySelector('.m1-key-size-preview output')?.textContent === '2u × 1u');
    await undo();
    const inspector = page.locator('.m1-inspector-body');
    const offset = inspector.getByLabel('Offset X', { exact: true });
    const before = Number(await offset.inputValue());
    check('offset hidden until Ctrl', await page.locator('[aria-label="Drag to offset column"]').count() === 0);
    await page.keyboard.down('Control');
    const handle = page.locator('[aria-label="Drag to offset column"]');
    await handle.waitFor();
    check('Ctrl replaces stagger rather than adding a handle', await page.locator('[aria-label="Drag to stagger"]').count() === 0 && await handle.count() === 1);
    const box = await handle.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down(); await page.mouse.move(box.x + box.width / 2 + 30, box.y + box.height / 2 - 15, { steps: 5 });
    await page.keyboard.press('Escape'); await page.mouse.up();
    await waitValue(offset, before);
    check('Ctrl offset Escape cancels', true);
    const fresh = await handle.boundingBox();
    await page.mouse.move(fresh.x + fresh.width / 2, fresh.y + fresh.height / 2);
    await page.mouse.down(); await page.mouse.move(fresh.x + fresh.width / 2 + 30, fresh.y + fresh.height / 2, { steps: 5 }); await page.mouse.up();
    for (let i = 0; i < 100 && Math.abs(Number(await offset.inputValue()) - before) < 0.0001; i++) await page.waitForTimeout(50);
    check('Ctrl offset commits a real edit', Math.abs(Number(await offset.inputValue()) - before) > 0.0001);
    await undo(); await waitValue(offset, before);
    check('Ctrl offset is one undo step', true);
    await page.keyboard.up('Control');
    check('stagger returns after Ctrl release', await page.locator('[aria-label="Drag to stagger"]').count() === 1);
    const centerErrors = await page.evaluate(() => {
      const selected = [...document.querySelectorAll('.m1-keycap-overlay.is-selected')].map(e => e.getBoundingClientRect());
      const center = (Math.min(...selected.map(r => r.x)) + Math.max(...selected.map(r => r.right))) / 2;
      return ['Drag to stagger', 'Drag to splay'].map(label => { const r = document.querySelector(`[aria-label="${label}"]`).getBoundingClientRect(); return Math.abs(r.x + r.width / 2 - center); });
    });
    check('stagger and splay align with selection center', centerErrors.every(error => error <= 3));
    check('delete is outside Advanced and available on canvas', await inspector.locator('.m1-selection-delete button').isVisible() && await page.locator('.m1-canvas-delete button').isVisible());
    check('Splay affects is beside Splay, outside origin disclosure', await inspector.getByLabel('Splay affects', { exact: true }).evaluate(e => !e.closest('details[aria-label="Splay and origin"]')));
    const layerTrigger = page.locator('#m1-layers-trigger');
    check('Layers collapsed by default', await layerTrigger.getAttribute('aria-expanded') === 'false');
    const dock = await layerTrigger.boundingBox(), pane = await page.locator('#m1-objects-panel').boundingBox();
    check('Layers stays at bottom independent of tree length', dock.y > pane.y + pane.height - 110);
    await layerTrigger.click();
    const layers = await page.locator('#m1-layers-list').boundingBox();
    check('Layers opens upward', layers && layers.y + layers.height <= dock.y + 2);
    await page.keyboard.press('Escape');
    const snap = page.getByRole('button', { name: 'Snap', exact: true });
    await snap.click();
    const snapBody = await page.locator('#m1-snap-dock-options').boundingBox(), snapButton = await snap.boundingBox();
    check('Snap options open upward and stay in viewport', snapBody && snapBody.y >= 0 && snapBody.y + snapBody.height <= snapButton.y + 2);
    await page.getByLabel('Snap increment', { exact: true }).selectOption('0.5');
    await page.keyboard.press('Escape');
    const beforeDelete = await page.locator('.m1-scene-part').count();
    await page.locator('.m1-canvas-delete button').click();
    await page.waitForFunction(count => document.querySelectorAll('.m1-scene-part').length < count, beforeDelete);
    await undo();
    await page.waitForFunction(count => document.querySelectorAll('.m1-scene-part').length === count, beforeDelete);
    check('canvas delete uses existing typed delete and one Undo restores geometry', true);
    await nav('PCB');
    check('board-wide Routing & nets is default', await page.locator('.m1-pcb-wiring-header h2').textContent() === 'Routing & nets');
    check('Nets overview visible', await page.locator('.m1-pcb-net-overview').isVisible());
    await page.getByLabel('Group objects by', { exact: true }).selectOption('part');
    check('parts grouped by actual categories', await page.locator('.m1-component').filter({ hasText: 'Switches' }).count() === 1 && await page.locator('.m1-component').filter({ hasText: 'Diodes' }).count() === 1);
    await page.getByLabel('Group objects by', { exact: true }).selectOption('layout');
    check('layout grouping restores matrix tree', await page.locator('.m1-component').filter({ hasText: 'keys' }).count() > 0);
    await page.locator('.m1-component').filter({ hasText: 'Right PCB' }).click();
    await page.waitForFunction(() => document.querySelector('.m1-pcb-wiring-breadcrumb')?.textContent.includes('Right PCB'));
    check('split-board tree navigation replaces removed board picker', true);
    await page.locator('.m1-component').filter({ hasText: 'Left PCB' }).click();
    await page.waitForFunction(() => document.querySelector('.m1-pcb-wiring-breadcrumb')?.textContent.includes('Left PCB'));
    for (const workspace of ['Layout', 'PCB', 'Keymap', 'Keycaps', 'Case', 'Parts']) {
      await nav(workspace);
      if (workspace === 'Layout') {
        await page.getByRole('button', { name: 'Select Column', exact: true }).click();
        await page.locator('.m1-scene-part[data-part-id^="matrix/"][data-part-id$="r0c2"] .m1-part-hit-area').first().click();
      }
      if (workspace === 'Case') {
        check('Case stackup visible without configuration click', await page.locator('.m1-stackup-preview').isVisible());
        check('Case generation controls are in footer', await page.locator('footer.m1-case-footer').isVisible());
        check('camera controls joined Case toolbar', await page.locator('.m1-case-view-toolbar .m1-case-camera-controls').count() === 1);
      }
      if (workspace === 'Parts') {
        const create = page.locator('#m1-objects-panel').getByRole('button', { name: 'New assembly', exact: true });
        await create.click(); await page.locator('.m1-parts-assembly-editor').waitFor();
        check('left + opens existing assembly editor', true);
        check('New assembly action removed from Inspector', await page.locator('.m1-inspector-body > .m1-parts-inspector > .m1-parts-assemblies > .m1-parts-create-component').count() === 0);
        await page.getByRole('option', { name: 'MX Hotswap', exact: true }).click();
        await page.getByRole('button', { name: '3D model', exact: true }).click();
        await page.locator('.m1-case-view-toolbar [aria-label="Part preview view"]').waitFor({ timeout: 60000 });
        check('Parts 3D view mode and camera controls share one pill', true);
        await page.getByRole('button', { name: '2D footprint', exact: true }).click();
      }
      check(`${workspace}: routed PCB reference absent`, await page.locator('.m1-board-reference').count() === 0);
      for (const width of [1513, 1024, 390]) {
        await page.setViewportSize({ width, height: width === 390 ? 844 : 1043 }); await page.waitForTimeout(200);
        const centered = await page.evaluate(() => {
          const toolbar = [...document.querySelectorAll('.m1-canvas-toolbar,.m1-case-view-toolbar,.m1-parts-preview > .m1-design-view-group')].find(e => e.getBoundingClientRect().width && getComputedStyle(e).display !== 'none');
          if (!toolbar) return true;
          const view = toolbar.closest('.m1-workspace-content'); if (!view) return true;
          const t = toolbar.getBoundingClientRect(), v = view.getBoundingClientRect();
          return Math.abs(t.x + t.width / 2 - v.x - v.width / 2) <= 2;
        });
        check(`${workspace} ${width}: toolbar remains centered`, centered);
        check(`${workspace} ${width}: no page overflow`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
        if (shots) await page.screenshot({ path: `${shots}/${workspace}-${width}.png` });
      }
      await page.setViewportSize({ width: 1513, height: 1043 });
    }
    assert.deepEqual(failures, [], failures.join('\n'));
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
