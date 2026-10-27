// Current Graphite feedback acceptance. Requires Playwright on NODE_PATH.
const assert = require('node:assert/strict');
const fs = require('node:fs');
if (process.env.BOARDSTUDIO_GRAPHITE_IDENTITY || process.env.BOARDSTUDIO_GRAPHITE_BUILD_RUN) {
  const result = require('node:child_process').spawnSync('python3', [require('node:path').join(__dirname, 'graphite-identity.py')], { stdio: 'inherit' });
  if (result.status !== 0) throw Error('Graphite identity collection failed');
}
const { chromium } = require('playwright');
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
  const selectScope = async scope => {
    const direct = page.getByRole('button', { name: `Select ${scope}`, exact: true });
    if (await direct.isVisible().catch(() => false)) return direct.click();
    await page.locator('#m1-layout-select-trigger').click();
    await page.locator('#m1-layout-select-menu').getByRole('button', { name: scope, exact: true }).click();
  };
  const shots = process.env.BOARDSTUDIO_SCREENSHOTS;
  if (shots) fs.mkdirSync(shots, { recursive: true });
  try {
    await page.goto(process.env.BOARDSTUDIO_PREVIEW_URL || 'http://127.0.0.1:8182/', { waitUntil: 'domcontentloaded', timeout: 60000 });
    await ready();
    await page.getByRole('button', { name: 'Create new keyboard', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Split keyboard', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Continue to layout', exact: true }).click({ timeout: 60000 });
    for (const name of ['Rows', 'Columns']) {
      const slider = page.getByRole('slider', { name, exact: true });
      await slider.focus();
      const value = Number(await slider.getAttribute('aria-valuenow'));
      assert(Number.isInteger(value));
      for (let i = 0; i < Math.abs(10 - value); i++) await slider.press(value < 10 ? 'ArrowRight' : 'ArrowLeft');
      assert.equal(await slider.getAttribute('aria-valuenow'), '10');
    }
    await page.getByRole('button', { name: 'Create matrix', exact: true }).click({ timeout: 60000 });
    await page.locator('.m1-scene-part[data-part-id^="matrix/"]').first().waitFor({ timeout: 60000 });
    await page.getByRole('button', { name: 'Hide guide', exact: true }).click({ timeout: 60000 });
    await nav('Layout');
    await page.getByRole('button', { name: 'Fit board', exact: true }).click();
    await selectScope('Column');
    check('header workspace navigation exposes six workspaces and current Layout', await page.evaluate(() => (() => { const all = [...document.querySelectorAll('.m1-workflow-navigation')], n = document.querySelector('.m1-topbar .m1-workflow-navigation'); const names = ['Layout', 'PCB', 'Keymap', 'Keycaps', 'Case', 'Parts']; return all.length === 1 && n?.getAttribute('aria-label') === 'Workspaces' && !document.querySelector('#m1-objects-panel .m1-workflow-navigation') && n.querySelectorAll('button').length === 6 && names.every(name => n.querySelectorAll(`button[aria-label="Open ${name} workspace"][aria-controls="m1-workspace-panel"]`).length === 1) && n.querySelectorAll('[aria-current="page"]').length === 1 && n.querySelector('[aria-label="Open Layout workspace"]')?.getAttribute('aria-current') === 'page'; })()));
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
    await undo(); await page.waitForFunction(() => document.querySelector('.m1-key-size-preview output')?.textContent === '1u × 1u');
    const inspector = page.locator('.m1-inspector-body');
    const offset = inspector.getByLabel('Offset X', { exact: true });
    const before = Number(await offset.inputValue());
    check('offset hidden until Ctrl', await page.locator('[aria-label="Drag to offset column"]').count() === 0);
    await page.keyboard.down('Control');
    const handle = page.locator('[aria-label="Drag to offset column"]');
    await handle.waitFor();
    check('Ctrl replaces stagger rather than adding a handle', await page.locator('[aria-label="Drag to stagger"]').count() === 0 && await handle.count() === 1);
    const selected = page.locator('.m1-keycap-overlay.is-selected').first();
    const initial = await selected.boundingBox();
    {
      const box = await handle.boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.evaluate(() => document.addEventListener('pointerdown', e => {
        window.__graphitePointerId = e.pointerId;
        window.__graphiteMoved = false;
        document.addEventListener('pointermove', move => {
          window.__graphiteMoved = move.pointerId === e.pointerId && (move.clientX !== e.clientX || move.clientY !== e.clientY);
        }, { once: true, capture: true });
      }, { once: true, capture: true }));
      await page.mouse.down(); await page.mouse.move(box.x + box.width / 2 + 30, box.y + box.height / 2 - 15, { steps: 5 });
      const captured = await page.evaluate(() => [...document.querySelectorAll('svg.m1-canvas,.m1-transform-handle')].some(e => e.hasPointerCapture(window.__graphitePointerId)));
      const moved = await page.evaluate(() => window.__graphiteMoved === true);
      await page.waitForFunction(({ x, y }) => {
        const r = document.querySelector('.m1-keycap-overlay.is-selected')?.getBoundingClientRect();
        return r && Math.hypot(r.x - x, r.y - y) > 2;
      }, initial);
      await page.keyboard.press('Escape'); await page.mouse.up();
      await waitValue(offset, before);
      await page.waitForFunction(({ x, y }) => {
        const r = document.querySelector('.m1-keycap-overlay.is-selected')?.getBoundingClientRect();
        return r && Math.hypot(r.x - x, r.y - y) < 0.5;
      }, initial);
      check('Ctrl offset Escape restores the moved preview', captured && moved);
    }
    const fresh = await handle.boundingBox();
    await page.mouse.move(fresh.x + fresh.width / 2, fresh.y + fresh.height / 2);
    await page.evaluate(() => {
      window.__graphiteFinalDown = null;
      document.addEventListener('pointerdown', e => {
        window.__graphiteFinalDown = { id: e.pointerId, trusted: e.isTrusted, target: e.target.closest('[aria-label="Drag to offset column"]') !== null, actual: e.target.outerHTML.slice(0, 300) };
      }, { once: true, capture: true });
    });
    await page.mouse.down(); await page.mouse.move(fresh.x + fresh.width / 2 + 30, fresh.y + fresh.height / 2, { steps: 5 });
    const finalProbe = await page.evaluate(() => ({ down: window.__graphiteFinalDown, captured: [...document.querySelectorAll('svg.m1-canvas,.m1-transform-handle')].some(e => e.hasPointerCapture(window.__graphiteFinalDown?.id)) }));
    await page.mouse.up();
    check(`Ctrl offset final pointerdown reaches handle (observed: ${JSON.stringify(finalProbe.down)})`, finalProbe.down?.trusted && finalProbe.down?.target);
    check('Ctrl offset final drag acquires pointer capture', finalProbe.captured);
    for (let i = 0; i < 100 && Math.abs(Number(await offset.inputValue()) - before) < 0.0001; i++) await page.waitForTimeout(50);
    const committed = Math.abs(Number(await offset.inputValue()) - before) > 0.0001;
    check('Ctrl offset commits a real edit', committed);
    if (committed) {
      await undo(); await waitValue(offset, before);
      check('Ctrl offset is one undo step', true);
    } else {
      check('Ctrl offset is one undo step (blocked: edit did not land)', false);
    }
    await page.keyboard.up('Control');
    check('stagger returns after Ctrl release', await page.locator('[aria-label="Drag to stagger"]').count() === 1);
    check('stagger and splay form a centered accessible rail', await page.evaluate(() => (() => { const keys = [...document.querySelectorAll('.m1-keycap-overlay.is-selected')].map(e => e.getBoundingClientRect()); const handles = ['Drag to stagger', 'Drag to splay'].map(label => document.querySelector(`[aria-label="${label}"]`)); if (!keys.length || handles.some(e => !e || e.getAttribute('role') !== 'button' || e.getAttribute('tabindex') !== '0')) return false; const [a, b] = handles.map(e => e.getBoundingClientRect()); const center = (Math.min(...keys.map(r => r.left)) + Math.max(...keys.map(r => r.right))) / 2; return a.width > 0 && b.width > 0 && Math.abs(a.right - b.left) <= 2 && Math.abs(a.top - b.top) <= 2 && Math.abs((a.left + b.right) / 2 - center) <= 3 && a.top < Math.min(...keys.map(r => r.top)); })()));
    check('delete is outside Advanced and available on canvas', await inspector.locator('.m1-selection-delete button').isVisible() && await page.locator('.m1-canvas-delete button').isVisible());
    check('Splay affects is beside Splay, outside origin disclosure', await inspector.getByLabel('Splay affects', { exact: true }).evaluate(e => !e.closest('details[aria-label="Splay and origin"]')));
    const layerTrigger = page.locator('#m1-layers-trigger');
    check('Layers collapsed by default', await layerTrigger.getAttribute('aria-expanded') === 'false');
    const dock = await layerTrigger.boundingBox(), pane = await page.locator('#m1-objects-panel').boundingBox();
    check('Layers stays at bottom independent of tree length', dock.y > pane.y + pane.height - 110);
    await layerTrigger.click();
    check('Layers opens an accessible inline disclosure', await page.evaluate(() => (() => { const t = document.querySelector('#m1-layers-trigger'), l = document.querySelector('#m1-layers-list'), c = document.getElementById(t?.getAttribute('aria-controls')); const r = l?.getBoundingClientRect(); return t?.getAttribute('aria-label') === 'Canvas visibility' && t.getAttribute('aria-expanded') === 'true' && c?.contains(l) && !!r?.width && !!r?.height && !!l.querySelector('button[aria-label="Hide Keycaps"][aria-pressed="true"]'); })()));
    await page.getByRole('button', { name: 'Hide Keycaps', exact: true }).click();
    await page.waitForFunction(() => !document.querySelector('.m1-keycap-overlay') && !!document.querySelector('#m1-layers-list button[aria-label="Show Keycaps"][aria-pressed="false"]'));
    check('Layers visibility control hides keycaps', true);
    await page.getByRole('button', { name: 'Show Keycaps', exact: true }).click();
    await page.waitForFunction(() => !!document.querySelector('.m1-keycap-overlay') && !!document.querySelector('#m1-layers-list button[aria-label="Hide Keycaps"][aria-pressed="true"]'));
    check('Layers visibility control restores keycaps', true);
    await layerTrigger.click();
    check('Layers disclosure closes', await layerTrigger.getAttribute('aria-expanded') === 'false' && !await page.locator('#m1-layers-list').isVisible());
    const snap = page.getByRole('button', { name: 'Snap', exact: true });
    await snap.focus();
    await page.keyboard.press('Space');
    const increment = page.getByLabel('Snap increment', { exact: true });
    check('Snap accordion opens with Space and keeps focus', await snap.getAttribute('aria-expanded') === 'true' && await increment.isVisible() && await snap.evaluate(e => document.activeElement === e));
    await increment.selectOption('0.5');
    await snap.focus();
    await page.keyboard.press('Enter');
    check('Snap accordion closes by keyboard', await snap.getAttribute('aria-expanded') === 'false' && !await increment.isVisible() && await snap.evaluate(e => document.activeElement === e));
    await page.keyboard.press('Enter');
    check('Snap increment persists', await snap.getAttribute('aria-expanded') === 'true' && await increment.inputValue() === '0.5');
    await page.keyboard.press('Enter');
    await selectScope('Key');
    await page.locator('.m1-scene-part[data-part-id^="matrix/"][data-part-id$="r5c5"] .m1-part-hit-area').first().click();
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
    const expandLeft = page.getByRole('button', { name: 'Expand Left half', exact: true });
    if (await expandLeft.count()) await expandLeft.click();
    check('layout grouping restores matrix tree', await page.locator('.m1-component').filter({ hasText: 'keys' }).count() > 0);
    await page.locator('.m1-component').filter({ hasText: 'Main board' }).first().click();
    await page.waitForFunction(() => document.querySelector('.m1-pcb-wiring-breadcrumb')?.textContent.includes('Main board'));
    check('board tree navigation replaces removed board picker', true);
    for (const workspace of ['Layout', 'PCB', 'Keymap', 'Keycaps', 'Case', 'Parts']) {
      await nav(workspace);
      check(`${workspace}: workspace navigation marks current`, await page.locator('.m1-workflow-navigation [aria-current="page"]').count() === 1 && await page.getByRole('button', { name: `Open ${workspace} workspace`, exact: true }).getAttribute('aria-current') === 'page');
      if (workspace === 'Layout') {
        await selectScope('Column');
        await page.locator('.m1-scene-part[data-part-id^="matrix/"][data-part-id$="r0c2"] .m1-part-hit-area').first().click();
      }
      if (workspace === 'Case') {
        await page.locator('.m1-case-view').waitFor();
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
