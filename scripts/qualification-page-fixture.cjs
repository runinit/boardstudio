// Authored product input, restored through normal project storage. No provider/report doubles.
const assert = require('node:assert/strict');
const contract = require('../cad/wasm/tests/fixtures/generated-analysis-contract.json');

module.exports = async function qualificationPageScenario(context, baseUrl, unavailablePcb = false) {
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    // Observe requests at the real worker boundary, without replacing delivery.
    window.collisionPreparations = [];
    const postMessage = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message, ...args) {
      if (message?.frame) {
        const frame = JSON.parse(message.frame);
        if (frame.operation === 'read-step') window.collisionPreparations.push({
          sha: frame.sourceSha256, options: frame.stepImportOptions,
        });
      }
      return postMessage.call(this, message, ...args);
    };
  });
  try {
    await page.goto(baseUrl, { waitUntil: 'domcontentloaded' });
    await page.getByRole('button', { name: 'Create new keyboard', exact: true }).click({ timeout: 60000 });
    await page.getByRole('button', { name: 'Continue to layout', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('.m1-save-state')?.dataset.state === 'saved');
    await page.evaluate(unavailable => new Promise((resolve, reject) => {
      const request = indexedDB.open('boardstudio-m1-boardstudio');
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const db = request.result;
        const transaction = db.transaction('projects', 'readwrite');
        const store = transaction.objectStore('projects');
        const query = store.getAll();
        query.onsuccess = () => {
          if (query.result.length !== 1) { transaction.abort(); return; }
          const document = query.result[0], board = document.boards[0];
          document.name = 'Qualification rectangle and original 0805';
          document.outline = [{ id: board.outlineIds[0], kind: 'rect', center: { x: 10, y: 5 },
            size: { x: 20, y: 10 }, radius: 0, operation: 'add' }];
          document.definitions = [{ id: 'qualification-cap', name: 'Original 0805', kind: 'passive',
            courtyard: [{ x: -.625, y: -1 }, { x: .625, y: -1 }, { x: .625, y: 1 }, { x: -.625, y: 1 }], pads: [],
            models: [{ assetId: 'bundled-model:infused-kim/SMD_0805_Capacitor.step', offset: { x: 0, y: 0, z: 0 },
              rotation: { x: 0, y: 0, z: 0 }, scale: { x: 1, y: 1, z: 1 } }] }];
          // Common X/Z placement; first occurrence is translated -3 along world Y from the second.
          document.parts = [3, 6].map((y, index) => ({ id: `qualification-cap-${index}`, definitionId: 'qualification-cap',
            reference: `C${index + 1}`, pose: { at: { x: 10, y }, rotation: 0 }, side: 'front', outline: { excluded: true } }));
          board.partIds = document.parts.map(part => part.id);
          document.mechanical = { boardId: board.id, method: 'printed', mount: 'rigid', plateThickness: 2,
            plateFoamThickness: .5, pcbThickness: 1.6, bottomFoamThickness: .4, batteryHeight: 4,
            bottomThickness: 3, plateToPcb: 3.5, wallThickness: 2, clearance: 0, bottomStyle: 'sheet',
            middleFrame: false, openingAllowance: 0, mounts: [], closureMounts: [],
            profiles: [] };
          if (unavailable) {
            delete document.mechanical;
            document.caseBodies = [0, 3].map((z, index) => ({ id: index ? 'bottom' : 'plate', name: index ? 'Upper plate' : 'Plate',
              boardId: board.id, kind: 'plate', thickness: 2, clearance: 0, z }));
          }
          document.revision += 1;
          store.put(document);
        };
        transaction.oncomplete = () => { db.close(); resolve(); };
        transaction.onabort = transaction.onerror = () => { db.close(); reject(transaction.error || Error('fixture transaction aborted')); };
      };
    }), unavailablePcb);
    await page.reload();
    await page.getByRole('button', { name: 'Open Case workspace', exact: true }).click({ timeout: 60000 });
    await page.getByRole('status').filter({ hasText: 'Exact case geometry ready.' }).waitFor({ timeout: 30000 });
    async function run(button) {
      await page.evaluate(() => { window.collisionPreparations = []; });
      await page.getByRole('button', { name: button, exact: true }).click();
      await page.waitForFunction(() => {
        const panel = document.querySelector('.m1-case-collision-checks');
        return panel && !panel.querySelector('[role="status"]') && panel.textContent.includes('↔');
      }, null, { timeout: 120000 });
      const panel = page.locator('.m1-case-collision-checks');
      const summaries = await panel.locator('p').evaluateAll(nodes => nodes.filter(node => !node.closest('.m1-case-collision-provenance')
        && node.textContent.includes('↔')).map(node => node.textContent));
      const alerts = await panel.locator('[role="alert"]').allTextContents();
      const provenance = await panel.locator('.m1-case-collision-provenance').allTextContents();
      const preparations = await page.evaluate(() => window.collisionPreparations);
      return { summaries, alerts, provenance, preparations };
    }
    const defaults = await run('Check collisions');
    if (unavailablePcb) {
      assert(defaults.alerts.some(text => text.startsWith('PCB unavailable:')));
      const pcbPairs = defaults.summaries.filter(text => text.includes('[pcb-reference]'));
      assert.equal(pcbPairs.length, 4);
      assert(pcbPairs.every(text => text.includes(': Uncertain (no mesh observation;')));
      assert(defaults.summaries.some(text => text.includes('Clear [1.000, 1.000]')));
      return { unavailablePcb: true, alerts: defaults.alerts, pcbPairs };
    }
    assert.deepEqual(defaults.alerts, [], 'supported product fixture includes an available physical PCB and vendor assets');
    assert(defaults.summaries.some(text => /C1 .*↔ C2 .*: Clear/.test(text)), 'original 0805 pair reaches a supported Clear band');
    assert(defaults.summaries.some(text => /\[(?:plate|bottom)\].*↔.*\[pcb-reference\].*: Clear/.test(text)), 'Case/physical-PCB generated pair reaches a supported Clear band');
    assert(defaults.summaries.some(text => /C[12] .*↔.*\[(?:plate|bottom|pcb-reference)\].*: Clear/.test(text)), 'mixed real vendor/generated pair reaches a supported Clear band');
    assert.equal(defaults.summaries.length, 21, 'all seven real operands contribute every unordered pair');
    assert.equal(new Set(defaults.summaries.map(text => text.split(': ')[0])).size, 21);
    const vendorDetails = defaults.provenance[0];
    const interval = vendorDetails.match(/Source surface distance: \[([^,]+), ([^\]]+)\]/);
    assert(interval, 'structured source interval remains visible separately from rounded summary');
    const expected = contract.qualificationV2.activation.pairCases[0].expectedSourceIntervalMm;
    const numericalWidth = contract.qualificationV2.activation.queryLimits.requestedNumericalWidthMm;
    assert(Number(interval[1]) <= 1 && Number(interval[2]) >= 1);
    assert(Number(interval[1]) >= expected[0] - numericalWidth && Number(interval[2]) <= expected[1] + numericalWidth);
    assert.equal((vendorDetails.match(/declared source precision 0.01 mm once/g) || []).length, 2);
    assert(vendorDetails.includes('STEP representation #') && vendorDetails.includes('source face #') && vendorDetails.includes('conservative box pairs'));
    assert(defaults.provenance.some(text => text.includes('generated body pcb-reference') && text.includes('source face index')));
    assert(defaults.preparations.every(request => request.options?.targetDeflectionMm == null), 'default checking does not request tight refinement');
    const tight = await run('Check at 0.01 mm');
    assert.equal(tight.preparations.length, 1, 'two identical placements share one real provider preparation');
    assert.equal(tight.preparations[0].options.targetDeflectionMm, 0.01);
    assert.deepEqual(tight.alerts, []);
    assert(tight.summaries.some(text => /C1 .*↔ C2 .*: Uncertain/.test(text)), '0805 source precision refuses the tight request');
    assert(tight.summaries[0].includes('exceeds requested accuracy 0.01 mm'));
    assert.equal(tight.summaries.length, 21);
    await page.getByRole('button', { name: 'Check collisions', exact: true }).click();
    await page.getByRole('button', { name: 'Cancel check', exact: true }).click();
    await page.waitForTimeout(500);
    assert.equal(await page.locator('.m1-case-collision-checks').locator('p').filter({ hasText: '↔' }).count(), 0,
      'cancelled real page run cannot publish old findings');
    assert.deepEqual(errors, []);
    return { defaults, tight };
  } catch (error) {
    throw new Error(`${error.message}\nMounted page: ${await page.locator('body').innerText().catch(() => '')}`, { cause: error });
  } finally { await page.close(); }
};
