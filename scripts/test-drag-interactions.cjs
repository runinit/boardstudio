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
  const doc = async () => page.evaluate(database => Promise.race([
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
  ]), STORAGE_DATABASE).then(projects => {
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
  const selectScope = async name => {
    const direct = page.getByRole('button', { name: `Select ${name}`, exact: true });
    if (await direct.isVisible().catch(() => false)) return direct.click();
    await page.locator('#m1-layout-select-trigger').click();
    await page.locator('#m1-layout-select-menu').getByRole('button', { name, exact: true }).click();
  };
  const waitDocument = signature => page.waitForFunction(async ({ expected, database }) => {
    const project = await new Promise((resolve, reject) => {
      const request = indexedDB.open(database);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const db = request.result;
        const query = db.transaction('projects').objectStore('projects').getAll();
        query.onerror = () => reject(query.error);
        query.onsuccess = () => { db.close(); resolve(query.result[0]); };
      };
    });
    if (!project) return false;
    delete project.revision;
    const canonical = value => Array.isArray(value) ? value.map(canonical)
      : value && typeof value === 'object'
        ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
        : value;
    return JSON.stringify(canonical(project)) === expected;
  }, { expected: signature, database: STORAGE_DATABASE }, { timeout: 20000 });
  const waitDocumentChanged = async signature => {
    const settled = await page.waitForFunction(async ({ previous, database }) => {
    const project = await new Promise((resolve, reject) => {
      const request = indexedDB.open(database);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const db = request.result;
        const query = db.transaction('projects').objectStore('projects').getAll();
        query.onerror = () => reject(query.error);
        query.onsuccess = () => { db.close(); resolve(query.result[0]); };
      };
    });
    if (!project) return false;
    delete project.revision;
    const canonical = value => Array.isArray(value) ? value.map(canonical)
      : value && typeof value === 'object'
        ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
        : value;
    if (JSON.stringify(canonical(project)) === previous) return false;
    return true;
    }, { previous: signature, database: STORAGE_DATABASE }, { timeout: 20000 });
    try {
      await settled.jsonValue();
    } finally {
      await settled.dispose();
    }
  };

  const vectorAt = (values, index) => values?.[index] || { x: 0, y: 0 };
  const matrixState = (project, matrixId, row, column) => {
    const matrix = project.matrices.find(item => item.id === matrixId);
    assert.ok(matrix, `matrix ${matrixId} exists in the saved document`);
    const cells = matrix.cells || [];
    const cell = cells.find(item => item.row === row && item.column === column);
    return {
      offset: vectorAt(matrix.columnOffsets, column),
      stagger: matrix.columnStaggers?.[column] || 0,
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

  async function runDrag(config, readState, expectedFields, allowedFields = expectedFields) {
    console.log(`drag-smoke: ${config.name} starts`);
    await saved();
    const before = await doc();
    const beforeScene = await scene();
    const expected = readState(before.project);
    const { live, release } = await drag({ ...config, beforeScene });
    assert.notDeepEqual(live.scene, beforeScene, `${config.name}: drag produced a visible preview`);
    await release();
    await waitDocumentChanged(before.signature);
    const durableAt = await page.evaluate(() => performance.now());
    await saved();
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

  try {
    await page.goto(process.env.BOARDSTUDIO_PREVIEW_URL, { waitUntil: 'domcontentloaded', timeout: 60000 });
    await page.getByRole('button', { name: 'Create new keyboard', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Split keyboard', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Continue to layout', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Add key matrix', exact: true }).click({ timeout: 60000 });
    await page.getByLabel('New matrix rows', { exact: true }).fill('10');
    await page.getByLabel('New matrix columns', { exact: true }).fill('10');
    await page.getByRole('button', { name: 'Create matrix', exact: true }).click({ timeout: 60000 });
    console.log('drag-smoke: setup created a 10 x 10 matrix');
    const firstKey = page.locator('.m1-scene-part[data-part-id^="matrix/"][data-part-id$="r4c2"] .m1-part-hit-area').first();
    await firstKey.waitFor({ timeout: 60000 });
    await page.getByRole('button', { name: 'Edit existing objects', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Open Layout workspace', exact: true }).click();
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
      partRead, ['cellOffset']);

    await selectScope('Column');
    await firstKey.click();
    const columnRead = project => matrixState(project, matrixId, row, column);
    await runDrag({ name: 'Column body drag', selector: `.m1-scene-part[data-part-id="${targetPart}"] .m1-part-hit-area` },
      columnRead, ['columnCellOffsets'], ['columnCellOffsets', 'cellOffset']);

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
      columnRead, ['stagger']);
    await runDrag({ name: 'Ctrl Offset handle', selector: '[aria-label="Drag to offset column"]',
      label: 'Drag to offset column', control: true }, columnRead, ['offset']);

    const browserVersion = browser.version();
    assert.ok(checks.length >= 9, 'smoke completed all drag, cancellation and Undo checks');
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
