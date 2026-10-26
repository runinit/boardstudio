// Small real-page regression for trusted pointer drags and durable undo.
const { chromium } = require('playwright-core');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const STORAGE_DATABASE = 'boardstudio-m1-boardstudio';

const stable = value => Array.isArray(value)
  ? value.map(stable)
  : value && typeof value === 'object'
    ? Object.fromEntries(Object.keys(value).sort().map(key => [key, stable(value[key])]))
    : value;
const startedAt = new Date().toISOString();

// Case collision findings read `<left> [id] ↔ <right> [id]: <status> (<observation>; <reasons>)`. A pair only
// proves the physical PCB reached a real Parry query when the witness is a raw surface observation: an
// unavailable PCB renders the same pair label with "no mesh observation".
const CASE_BODY_PCB_PAIR = /\[(?:plate|bottom)[^\]]*\] ↔ PCB \[pcb-reference\]/;
const RAW_SURFACE_OBSERVATION = /\((?:surface intersection (?:near|observed)|surface separation \d)/;
const observedCasePcbPair = text => CASE_BODY_PCB_PAIR.test(text) && RAW_SURFACE_OBSERVATION.test(text);

// The exact summaries Runtime renders for an unavailable PCB or an uncertain raw observation never count.
function assertUnavailablePcbPairsDoNotCount() {
  for (const text of [
    'plate [plate] ↔ PCB [pcb-reference]: Uncertain (no mesh observation; left generated body has no certified analysisErrorBoundMm, right generated body has no certified analysisErrorBoundMm, Parry query has no certified numerical error bound)',
    'bottom [bottom] ↔ PCB [pcb-reference]: Uncertain (uncertain observation (query unsupported); raw mesh observation is uncertain: query unsupported)',
    'SW1 [SW1:0] ↔ PCB [pcb-reference]: Uncertain (surface intersection near (2.33, 5.08, 2.00) mm; left asset has no certified analysisErrorBoundMm)',
    'plate [plate] ↔ bottom [bottom]: Uncertain (surface separation 3.500 mm between (0.00, 0.00, 0.00) and (0.00, 0.00, 3.50) mm)',
  ]) assert.ok(!observedCasePcbPair(text), `not a PCB observation: ${text}`);
  assert.ok(observedCasePcbPair('plate [plate] ↔ PCB [pcb-reference]: Uncertain (surface separation 3.500 mm between (-12.40, -13.00, 3.50) and (-12.40, -13.00, 0.00) mm)'));
  assert.ok(observedCasePcbPair('bottom [bottom] ↔ PCB [pcb-reference]: Uncertain (surface intersection near (24.68, 28.75, -0.65) mm)'));
  assert.ok(observedCasePcbPair('bottom [bottom] ↔ PCB [pcb-reference]: Clear [0.400, 0.400] mm vs 0.000 mm (surface separation 0.400 mm between (0, 0, -2) and (0, 0, -1.6) mm)'));
}

async function setSlider(page, name, target) {
  const slider = page.getByRole('slider', { name, exact: true });
  const initial = Number(await slider.getAttribute('aria-valuenow'));
  for (let value = initial; value !== target; value += Math.sign(target - initial)) {
    await slider.press(initial < target ? 'ArrowRight' : 'ArrowLeft');
    const expected = String(value + Math.sign(target - initial));
    await page.waitForFunction(({ name, expected }) =>
      document.querySelector(`[role="slider"][aria-label="${name}"]`)?.getAttribute('aria-valuenow') === expected,
      { name, expected });
  }
  assert.equal(await slider.getAttribute('aria-valuenow'), String(target));
}


(async () => {
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_BIN || '/usr/bin/chromium',
    args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
  });
  const page = await browser.newPage({ viewport: { width: 1513, height: 1043 } });
  const pageErrors = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  const checks = [];
  const result = (name, details) => checks.push({ name, ...details });
  const readProjects = () => page.evaluate(database => Promise.race([
    new Promise((resolve, reject) => {
      const request = indexedDB.open(database);
      request.onerror = () => reject(request.error);
      request.onblocked = () => reject(Error('IndexedDB project read was blocked'));
      request.onsuccess = () => {
        const db = request.result;
        try {
          const query = db.transaction('projects').objectStore('projects').getAll();
          query.onerror = () => { db.close(); reject(query.error); };
          query.onsuccess = () => { db.close(); resolve(query.result); };
        } catch (error) {
          db.close();
          reject(error);
        }
      };
    }),
    new Promise((_, reject) => setTimeout(() => reject(Error('IndexedDB project read timed out')), 10000)),
  ]), STORAGE_DATABASE);
  const doc = async () => readProjects().then(projects => {
    assert.equal(projects.length, 1, 'expected one local project');
    const project = projects[0];
    const revision = project.revision;
    delete project.revision;
    return { project, revision, signature: JSON.stringify(stable(project)) };
  });
  const scene = () => page.evaluate(() => [...document.querySelectorAll('.m1-scene-part')]
    .map(element => [element.dataset.partId, element.getAttribute('transform')]));
  const saved = () => page.waitForFunction(() =>
    document.querySelector('.m1-save-state')?.dataset.state === 'saved', null, { timeout: 20000 });
  const startSaveTracking = () => page.evaluate(() => {
    window.__saveStates = [document.querySelector('.m1-save-state')?.dataset.state];
    new MutationObserver(() => {
      const state = document.querySelector('.m1-save-state')?.dataset.state;
      if (state && window.__saveStates.at(-1) !== state) window.__saveStates.push(state);
    }).observe(document.body, { attributes: true, childList: true, subtree: true, attributeFilter: ['data-state'] });
  });
  const waitForSaveCycle = () => page.waitForFunction(() => {
    const states = window.__saveStates || [];
    const saving = states.indexOf('saving', 1);
    return saving >= 0 && states.slice(saving + 1).includes('saved');
  }, null, { timeout: 20000 });
  const selectScope = async name => {
    const direct = page.getByRole('button', { name: `Select ${name}`, exact: true });
    if (await direct.isVisible().catch(() => false)) return direct.click();
    await page.locator('#m1-layout-select-trigger').click();
    await page.locator('#m1-layout-select-menu').getByRole('button', { name, exact: true }).click();
  };
  // Poll from Node: a promise-returning pageFunction is evaluated only once by
  // the pinned Playwright version instead of polling to its timeout, so each
  // wait re-evaluates here with the same 20 s bound.
  const pollUntil = async (check, message) => {
    let timer;
    const expired = new Promise((_, reject) => { timer = setTimeout(() => reject(Error(message)), 20000); });
    const pause = () => Promise.race([expired, new Promise(resolve => setTimeout(resolve, 200))]);
    try {
      while (!await Promise.race([expired, check()])) await pause();
      return true;
    } finally {
      clearTimeout(timer);
    }
  };
  const waitDocument = signature => pollUntil(async () => {
    const project = (await readProjects())[0];
    if (!project) return false;
    delete project.revision;
    return JSON.stringify(stable(project)) === signature;
  }, 'timed out waiting for the saved document');
  const matrixState = (project, matrixId, row, column) => {
    const matrix = project.matrices.find(item => item.id === matrixId);
    assert.ok(matrix, `matrix ${matrixId} exists in the saved document`);
    const cells = matrix.cells || [];
    const cell = cells.find(item => item.row === row && item.column === column);
    return {
      offset: matrix.columnLayout?.[column]?.offset || { x: 0, y: 0 },
      stagger: matrix.columnLayout?.[column]?.stagger || 0,
      cellOffset: cell?.offset || { x: 0, y: 0 },
      columnCellOffsets: Array.from({ length: matrix.rows }, (_, cellRow) =>
        cells.find(item => item.row === cellRow && item.column === column)?.offset || { x: 0, y: 0 }),
    };
  };
  const startEventCapture = () => page.evaluate(() => {
    window.__dragEvents = [];
    if (!window.__dragCaptureInstalled) {
      for (const type of ['pointerdown', 'pointermove', 'pointerup', 'pointercancel']) {
        document.addEventListener(type, event => window.__dragEvents.push({
          type,
          time: event.timeStamp,
          trusted: event.isTrusted,
          pointerType: event.pointerType,
          pointerId: event.pointerId,
          label: event.target.closest?.('.m1-transform-handle')?.getAttribute('aria-label'),
          partId: event.target.closest?.('[data-part-id]')?.getAttribute('data-part-id'),
        }), true);
      }
      window.__dragCaptureInstalled = true;
    }
  });

  async function drag({ name, selector, label, dx = 36, dy = 22, control = false, beforeScene }) {
    if (control) await page.keyboard.down('Control');
    let pointerDown = false;
    let closed = false;
    const release = async () => {
      if (closed) return;
      if (pointerDown) {
        await page.mouse.up();
        pointerDown = false;
      }
      if (control) await page.keyboard.up('Control');
      closed = true;
    };
    try {
      const target = page.locator(selector).first();
      await target.waitFor({ state: 'visible' });
      const box = await target.boundingBox();
      assert.ok(box?.width && box?.height, `${name}: target has visible geometry`);
      await startEventCapture();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      pointerDown = true;
      try {
        await page.mouse.move(box.x + box.width / 2 + dx, box.y + box.height / 2 + dy, { steps: 5 });
        await page.waitForFunction(before => JSON.stringify(
          [...document.querySelectorAll('.m1-scene-part')]
            .map(element => [element.dataset.partId, element.getAttribute('transform')]),
        ) !== before, JSON.stringify(beforeScene), { timeout: 5000 });
        const live = await page.evaluate(() => {
          const events = window.__dragEvents;
          const down = events.find(event => event.type === 'pointerdown');
          return {
            events,
            capture: down && [...document.querySelectorAll('svg.m1-canvas,.m1-transform-handle')]
              .some(element => element.hasPointerCapture(down.pointerId)),
            scene: [...document.querySelectorAll('.m1-scene-part')]
              .map(element => [element.dataset.partId, element.getAttribute('transform')]),
          };
        });
        assert.ok(live.events.some(event => event.type === 'pointerdown' && event.trusted), `${name}: trusted pointerdown`);
        assert.ok(live.events.filter(event => event.type === 'pointermove' && event.trusted).length >= 2,
          `${name}: real pointer movement occurred`);
        assert.ok(live.capture, `${name}: canvas retained pointer capture while dragging`);
        if (label) assert.ok(live.events.some(event => event.label === label), `${name}: gesture used ${label}`);
        return { live, release };
      } catch (error) {
        await release().catch(() => {});
        throw error;
      }
    } catch (error) {
      await release().catch(() => {});
      throw error;
    }
  }

  async function runDrag(config, readState, expectedFields, identity, allowedFields = expectedFields) {
    console.log(`drag-smoke: ${config.name} starts`);
    await saved();
    const before = await doc();
    const beforeScene = await scene();
    const expected = readState(before.project);
    const { live, release } = await drag({ ...config, beforeScene });
    assert.notDeepEqual(live.scene, beforeScene, `${config.name}: drag produced a visible preview`);
    await startSaveTracking();
    await release();
    await waitForSaveCycle();
    await pollUntil(async () => {
      const matrix = (await readProjects())[0]?.matrices?.find(item => item.id === identity.matrixId);
      if (!matrix) return false;
      const cells = matrix.cells || [];
      const cell = cells.find(item => item.row === identity.row && item.column === identity.column);
      const state = {
        offset: matrix.columnLayout?.[identity.column]?.offset || { x: 0, y: 0 },
        stagger: matrix.columnLayout?.[identity.column]?.stagger || 0,
        cellOffset: cell?.offset || { x: 0, y: 0 },
        columnCellOffsets: Array.from({ length: matrix.rows }, (_, cellRow) =>
          cells.find(item => item.row === cellRow && item.column === identity.column)?.offset || { x: 0, y: 0 }),
        origin: matrix.origin || { x: 0, y: 0 },
      };
      return expectedFields.every(field => JSON.stringify(state[field]) !== JSON.stringify(expected[field]));
    }, `timed out waiting for the changed matrix cells (${config.name})`);
    const durableAt = await page.evaluate(() => performance.now());
    const after = await doc();
    assert.notEqual(after.signature, before.signature, `${config.name}: edit was durably saved`);
    const actual = readState(after.project);
    for (const field of expectedFields) {
      assert.notDeepEqual(actual[field], expected[field], `${config.name}: changed ${field}`);
    }
    if (config.name === 'Column body drag') {
      const moved = actual.columnCellOffsets.filter((value, index) =>
        JSON.stringify(value) !== JSON.stringify(expected.columnCellOffsets[index])).length;
      assert.ok(moved > 1, `Column body drag: moved multiple keys in its column (${moved})`);
    }
    const untouched = Object.keys(expected).filter(key => !allowedFields.includes(key));
    for (const key of untouched) assert.deepEqual(actual[key], expected[key], `${config.name}: preserved ${key}`);
    const events = await page.evaluate(() => window.__dragEvents);
    const pointerDown = events.find(event => event.type === 'pointerdown');
    const pointerUp = events.findLast(event => event.type === 'pointerup');
    assert.equal(pointerDown?.pointerType, 'mouse', `${config.name}: mouse input was used`);
    assert.ok(pointerUp?.trusted, `${config.name}: trusted pointerup completed the gesture`);
    assert.equal(pointerUp.pointerType, 'mouse', `${config.name}: pointerup used mouse input`);
    const pointerDownToUpMs = Math.round(pointerUp.time - pointerDown.time);
    const releaseToDurableMs = Math.round(durableAt - pointerUp.time);
    assert.ok(Number.isFinite(pointerDownToUpMs) && pointerDownToUpMs >= 0, `${config.name}: valid gesture timing`);
    assert.ok(Number.isFinite(releaseToDurableMs) && releaseToDurableMs >= 0,
      `${config.name}: durable timing follows pointerup`);
    result(config.name, {
      changed: expectedFields,
      input: 'mouse',
      events: events.length,
      pointerDownToUpMs,
      releaseToDurableMs,
      savedRevision: after.revision,
    });
    await page.getByRole('button', { name: 'Undo', exact: true }).click();
    await waitDocument(before.signature);
    await saved();
    const undone = await doc();
    assert.equal(undone.signature, before.signature, `${config.name}: Undo restored the complete saved document`);
    assert.ok(undone.revision > after.revision, `${config.name}: Undo was saved as a new revision`);
    assert.deepEqual(await scene(), beforeScene, `${config.name}: Undo restored the exact canvas scene`);
    result(`${config.name} undo`, { exactDocument: true, exactScene: true });
    console.log(`drag-smoke: ${config.name} and exact Undo passed`);
  }

  // The default collision check runs the real page, worker and Monstertruck provider on a 2 x 2 rigid-mount
  // case. The physical PCB must reach a real query against a generated case body, no collision error may be
  // reported, and the findings panel must scroll internally while the Case viewer stays visible beside it.
  async function collisionScenario(context) {
    assertUnavailablePcbPairsDoNotCount();
    const scenarioStartedMs = Date.now();
    const collisionPage = await context.newPage();
    collisionPage.on('pageerror', error => pageErrors.push(error.message));
    await collisionPage.goto(process.env.BOARDSTUDIO_PREVIEW_URL, { waitUntil: 'domcontentloaded', timeout: 60000 });
    await collisionPage.getByRole('button', { name: 'Create new keyboard', exact: true }).click({ timeout: 60000 });
    await collisionPage.getByRole('button', { name: 'Continue to layout', exact: true }).click({ timeout: 60000 });
    for (const name of ['Rows', 'Columns']) await setSlider(collisionPage, name, 2);
    await collisionPage.getByRole('button', { name: 'Create matrix', exact: true }).click({ timeout: 60000 });
    await collisionPage.getByRole('button', { name: 'Open Case workspace', exact: true }).click();
    await collisionPage.getByRole('button', { name: /Case \(optional\)/ }).click();
    await collisionPage.getByRole('combobox', { name: 'Case mounting style' }).selectOption('Rigid mount');
    await collisionPage.getByRole('status').filter({ hasText: 'Exact case geometry ready.' }).waitFor({ timeout: 120000 });
    const startedMs = Date.now();
    await collisionPage.getByRole('button', { name: 'Check collisions', exact: true }).click();
    // Finished means the pending status is gone and the check produced findings or errors.
    await collisionPage.waitForFunction(() => {
      const panel = document.querySelector('.m1-case-collision-checks');
      return panel && !panel.querySelector('[role="status"]')
        && [...panel.querySelectorAll('p')].some(element => element.textContent.includes('↔') || element.getAttribute('role') === 'alert');
    }, null, { timeout: 120000 });
    const collisionMs = Date.now() - startedMs;
    const layout = await collisionPage.evaluate(() => {
      const panel = document.querySelector('.m1-case-collision-checks');
      const container = panel.closest('.m1-case-panel');
      const viewer = document.querySelector('[aria-label^="Interactive 3D Case preview"]');
      const box = element => { const { x, y, width, height } = element.getBoundingClientRect(); return { x, y, width, height }; };
      const outside = element => !element.closest('.m1-case-collision-provenance');
      return {
        alerts: [...panel.querySelectorAll('p[role="alert"]')].map(element => element.textContent),
        findings: [...panel.querySelectorAll('p:not([role="alert"])')]
          .filter(element => outside(element) && element.textContent.includes('↔')).map(element => element.textContent),
        panel: { ...box(panel), scrollHeight: panel.scrollHeight, clientHeight: panel.clientHeight,
          overflowY: getComputedStyle(panel).overflowY },
        container: container && box(container),
        viewer: { ...box(viewer), display: getComputedStyle(viewer).display, visibility: getComputedStyle(viewer).visibility },
        viewport: { width: window.innerWidth, height: window.innerHeight },
      };
    });
    assert.deepEqual(layout.alerts, [], 'the default collision check reported no collision error, including PCB unavailable');
    const pcbPairs = layout.findings.filter(observedCasePcbPair);
    assert.ok(pcbPairs.length > 0, `a generated case body has a raw mesh observation against the physical PCB (${layout.findings.length} findings)`);
    result('Default collision PCB pair', { findings: layout.findings.length, observedPcbPairs: pcbPairs.length, alerts: 0, collisionMs });

    const { panel, viewer, container, viewport } = layout;
    assert.ok(['auto', 'scroll'].includes(panel.overflowY), `collision findings scroll inside their panel (overflow-y ${panel.overflowY})`);
    assert.ok(panel.scrollHeight > panel.clientHeight && panel.clientHeight > 0,
      `collision findings overflow their panel (scrollHeight ${panel.scrollHeight}, clientHeight ${panel.clientHeight})`);
    assert.ok(viewer.display !== 'none' && viewer.visibility === 'visible' && viewer.width > 0 && viewer.height > 0,
      `the Case viewer has a visible, nonzero rectangle (${viewer.width} x ${viewer.height})`);
    assert.ok(viewer.x >= 0 && viewer.y >= 0 && viewer.x + viewer.width <= viewport.width && viewer.y + viewer.height <= viewport.height,
      'the Case viewer lies inside the desktop viewport');
    assert.ok(container && panel.y >= container.y && panel.y + panel.height <= container.y + container.height,
      'the findings panel stays inside the Case panel instead of pushing past it');
    assert.ok(viewer.y + viewer.height <= panel.y + 1 || panel.y + panel.height <= viewer.y + 1,
      'the findings panel and the Case viewer do not overlap');
    result('Case viewer with overflowing collision findings', {
      scrollHeight: panel.scrollHeight, clientHeight: panel.clientHeight, overflowY: panel.overflowY,
      viewer: [Math.round(viewer.width), Math.round(viewer.height)], panelHeight: Math.round(panel.height),
    });
    console.log(`drag-smoke: default collision check observed ${pcbPairs.length} PCB pairs in ${collisionMs} ms (scenario ${Date.now() - scenarioStartedMs} ms); `
      + `findings scroll ${panel.clientHeight}/${panel.scrollHeight} px beside a ${Math.round(viewer.width)} x ${Math.round(viewer.height)} px viewer`);
  }

  try {
    await page.goto(process.env.BOARDSTUDIO_PREVIEW_URL, { waitUntil: 'domcontentloaded', timeout: 60000 });
    await page.getByRole('button', { name: 'Create new keyboard', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Split keyboard', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Continue to layout', exact: true }).click({ timeout: 60000 });
    for (const name of ['Rows', 'Columns']) await setSlider(page, name, 10);
    await page.getByRole('button', { name: 'Create matrix', exact: true }).click({ timeout: 60000 });
    console.log('drag-smoke: setup created a 10 x 10 matrix');
    const firstKey = page.locator('.m1-scene-part[data-part-id^="matrix/"][data-part-id$="r4c2"] .m1-part-hit-area').first();
    await firstKey.waitFor({ timeout: 60000 });
    await page.getByRole('button', { name: 'Hide guide', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Fit board', exact: true }).click();

    await selectScope('Part');
    await firstKey.click();
    const targetPart = await page.locator('.m1-scene-part[data-part-id^="matrix/"][data-part-id$="r4c2"]').first()
      .getAttribute('data-part-id');
    const [, matrixId, cell] = targetPart.split('/');
    const row = Number(cell.match(/r(\d+)c/)[1]);
    const column = Number(cell.match(/c(\d+)$/)[1]);
    const partRead = project => {
      const { columnCellOffsets, ...state } = matrixState(project, matrixId, row, column);
      return state;
    };
    await runDrag({ name: 'Part drag', selector: `.m1-scene-part[data-part-id="${targetPart}"] .m1-part-hit-area` },
      partRead, ['cellOffset'], { matrixId, row, column });

    await selectScope('Column');
    await firstKey.click();
    const columnRead = project => matrixState(project, matrixId, row, column);
    await runDrag({ name: 'Column body drag', selector: `.m1-scene-part[data-part-id="${targetPart}"] .m1-part-hit-area` },
      columnRead, ['columnCellOffsets'], { matrixId, row, column }, ['columnCellOffsets', 'cellOffset']);

    const stagger = '[aria-label="Drag to stagger"]';
    await saved();
    const cancelBefore = await doc();
    const cancelScene = await scene();
    const canceled = await drag({ name: 'Stagger cancel', selector: stagger, label: 'Drag to stagger', beforeScene: cancelScene });
    assert.notDeepEqual(canceled.live.scene, cancelScene, 'Stagger cancel: a real preview appeared');
    await page.keyboard.press('Escape');
    await canceled.release();
    await saved();
    await waitDocument(cancelBefore.signature);
    const cancelAfter = await doc();
    assert.equal(cancelAfter.revision, cancelBefore.revision, 'Stagger cancel created no saved revision');
    assert.deepEqual(await scene(), cancelScene, 'Stagger cancel restored the exact canvas scene');
    result('Stagger cancel', { previewSeen: true, pointerCapture: true, exactDocument: true, exactScene: true });

    await runDrag({ name: 'Stagger handle', selector: stagger, label: 'Drag to stagger' },
      columnRead, ['stagger'], { matrixId, row, column });
    await runDrag({ name: 'Ctrl Offset handle', selector: '[aria-label="Drag to offset column"]',
      label: 'Drag to offset column', control: true }, columnRead, ['offset'], { matrixId, row, column });

    await selectScope('Matrix');
    await firstKey.click();
    const matrixOriginRead = project => {
      const matrix = project.matrices.find(item => item.id === matrixId);
      assert.ok(matrix, `matrix ${matrixId} exists in the saved document`);
      return { origin: matrix.origin };
    };
    await runDrag({ name: 'Matrix move', selector: '[aria-label="Drag to move matrix"]',
      label: 'Drag to move matrix', dx: 40 }, matrixOriginRead, ['origin'], { matrixId, row, column });

    await collisionScenario(await browser.newContext({ viewport: { width: 1513, height: 1043 } }));

    const qualified = await require('./qualification-page-fixture.cjs')(
      await browser.newContext({ viewport: { width: 1513, height: 1043 } }), process.env.BOARDSTUDIO_PREVIEW_URL);
    result('Qualified collision product fixture', { defaultPairs: qualified.defaults.summaries.length,
      supportedClear: qualified.defaults.summaries.filter(text => text.includes(': Clear')).length,
      tightPairs: qualified.tight.summaries.length, vendorTightUnknown: true, realProviderWorkerRuntime: true });
    const unavailable = await require('./qualification-page-fixture.cjs')(
      await browser.newContext({ viewport: { width: 1513, height: 1043 } }), process.env.BOARDSTUDIO_PREVIEW_URL, true);
    result('Unavailable physical PCB stays uncertain', { alerts: unavailable.alerts.length, pairs: unavailable.pcbPairs.length });

    const browserVersion = browser.version();
    assert.ok(checks.length >= 11, 'smoke completed all drag, cancellation, Undo and collision checks');
    assert.deepEqual(pageErrors, [], 'page raised no uncaught browser errors');
    const receiptPath = process.env.BOARDSTUDIO_DRAG_RECEIPT_PATH;
    assert.ok(receiptPath, 'receipt path supplied by runner');
    fs.mkdirSync(path.dirname(receiptPath), { recursive: true });
    const receipt = {
      schema_version: 1,
      run_id: path.basename(path.dirname(receiptPath)),
      started_at: startedAt,
      completed_at: new Date().toISOString(),
      browser: browserVersion,
      viewport: { width: 1513, height: 1043 },
      source_head: process.env.BOARDSTUDIO_DRAG_SOURCE_HEAD,
      source_sha256: process.env.BOARDSTUDIO_DRAG_SOURCE_SHA256,
      build_sha256: process.env.BOARDSTUDIO_DRAG_BUILD_SHA256,
      checks,
      passed: true,
    };
    receipt.receipt_sha256 = crypto.createHash('sha256').update(JSON.stringify(receipt)).digest('hex');
    fs.writeFileSync(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`, { flag: 'wx' });
    console.log(`PASS drag browser smoke: ${checks.length} checks; receipt ${receiptPath}`);
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
