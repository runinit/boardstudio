// Finite U1 product workflow. Uses Runtime exports; no injected model, report or STL writer.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const ROOT = path.resolve(__dirname, '../..');
const DATABASE = 'boardstudio-m1-boardstudio';

module.exports = async function validatePage(page, origin, output, operation) {
  page.setDefaultTimeout(60000);
  const errors = [];
  const phases = [];
  const mark = name => phases.push({ name, hostUnixMs: Date.now(), hostMonotonicMs: performance.now(), purpose: 'diagnostic-not-benchmark' });
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    window.__u1 = { requests: [], replies: [], printBodies: [], steps: [], drawCalls: 0 };
    const Original = window.Worker;
    window.Worker = class extends Original {
      constructor(...args) {
        super(...args);
        this.addEventListener('message', event => {
          const message = event.data;
          if (message.kind !== 'reply' || !message.frame) return;
          const frame = JSON.parse(message.frame);
          if (!frame.operation) return;
          window.__u1.replies.push({ frame, bodies: message.result?.bodies?.map(body => body.id),
            stepBytes: message.result?.step?.byteLength ?? 0 });
          if (message.result?.step?.byteLength) window.__u1.steps.push({ frame, bytes: Array.from(message.result.step) });
          if (frame.operation === 'print-keycaps' && message.result?.bodies) {
            window.__u1.printBodies.push(...message.result.bodies.map(body => ({ ...body,
              positions: Array.from(body.positions), normals: Array.from(body.normals) })));
          }
        });
      }
      postMessage(message, ...args) {
        if (message.frame) {
          const frame = JSON.parse(message.frame);
          if (frame.operation) window.__u1.requests.push(frame);
        }
        return super.postMessage(message, ...args);
      }
    };
    // The shipped three-d renderer uses WebGL2. Observe real draws, then finish once after the pinned camera action.
    for (const name of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) {
      const draw = WebGL2RenderingContext.prototype[name];
      WebGL2RenderingContext.prototype[name] = function (...args) {
        const result = draw.apply(this, args);
        window.__u1.drawCalls++;
        window.__u1Gl = this;
        return result;
      };
    }
  });
  const saved = () => page.waitForFunction(() => document.querySelector('.m1-save-state')?.dataset.state === 'saved');
  const document = () => page.evaluate(database => new Promise((resolve, reject) => {
    const request = indexedDB.open(database);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      const query = db.transaction('projects').objectStore('projects').getAll();
      query.onerror = () => { db.close(); reject(query.error); };
      query.onsuccess = () => { db.close(); query.result.length === 1 ? resolve(query.result[0]) : reject(Error('expected one accepted project')); };
    };
  }), DATABASE);
  const download = async (button, filename) => {
    mark(`${filename}:action`);
    const pending = page.waitForEvent('download', { timeout: 180000 });
    await button.click();
    const result = await pending;
    assert.equal(await result.failure(), null);
    await result.saveAs(path.join(output, filename));
    mark(`${filename}:saved-download`);
    return { endpoint: button.toString(), file: filename, suggested: result.suggestedFilename(),
      bytes: fs.statSync(path.join(output, filename)).size, purpose: 'validation-only; diagnostic, not benchmark' };
  };
  try {
    mark('navigation');
    await page.goto(`${origin}/boardstudio/`, { waitUntil: 'domcontentloaded' });
    if (operation === 'freeze') {
      await page.getByRole('button', { name: 'Create new keyboard', exact: true }).click();
      await page.getByRole('button', { name: 'Continue to layout', exact: true }).click();
      for (const name of ['Rows', 'Columns']) {
        const slider = page.getByRole('slider', { name, exact: true });
        let value = Number(await slider.getAttribute('aria-valuenow'));
        while (value !== 2) {
          await slider.press(value > 2 ? 'ArrowLeft' : 'ArrowRight');
          value += value > 2 ? -1 : 1;
          await page.waitForFunction(({ name, value }) => Number(document.querySelector(`[role=slider][aria-label="${name}"]`)?.getAttribute('aria-valuenow')) === value,
            { name, value });
        }
      }
      await page.getByRole('button', { name: 'Create matrix', exact: true }).click();
      await page.locator('.m1-scene-part').first().waitFor();
      await saved();
      await page.getByRole('button', { name: 'Open Case workspace', exact: true }).click();
      await page.getByRole('button', { name: /Case \(optional\)/ }).click();
      await page.getByRole('combobox', { name: 'Case mounting style', exact: true }).selectOption('Rigid mount');
      await saved();
      // The guide intentionally replaces the inspector; dismiss it before editing matrix profiles.
      await page.getByRole('button', { name: 'Hide guide', exact: true }).click();
      await page.getByRole('button', { name: 'Open Keycaps workspace', exact: true }).click();
      await page.locator('#m1-workspace-Keycaps[aria-current=page]').waitFor();
      fs.writeFileSync(path.join(output, 'keycaps-controls.log'), await page.evaluate(() => JSON.stringify({
        inspector: document.querySelector('#m1-inspector-slot')?.outerHTML,
        disclosures: [...document.querySelectorAll('details')].map(d => ({ title: d.querySelector('summary')?.textContent, open: d.open })),
      })));
      const reveal = page.getByRole('button', { name: 'Show inspector', exact: true });
      if (await reveal.isVisible()) await reveal.click();
      await page.getByText('Matrix profiles', { exact: true }).click();
      const profile = page.getByRole('combobox', { name: /^Keycap profile for / });
      await profile.selectOption('dcs');
      await saved();
      await page.getByRole('button', { name: 'Open Case workspace', exact: true }).click();
    } else {
      assert.equal(operation, 'page');
      const archive = path.join(ROOT, 'cad/test/fixtures/performance/U1.boardstudio');
      assert(fs.existsSync(archive), 'freeze and commit the ordinary U1 project first');
      mark('frozen-project-import');
      const input = page.locator('input.m1-project-file-input:not(:disabled)').first();
      await input.waitFor({ state: 'attached' });
      await input.setInputFiles(archive);
      await page.getByRole('button', { name: 'Open Case workspace', exact: true }).waitFor();
      await saved();
      mark('accepted-editor-ready');
      mark('enter-Case');
      await page.getByRole('button', { name: 'Open Case workspace', exact: true }).click();
      const guide = page.getByRole('button', { name: 'Hide guide', exact: true });
      if (await guide.isVisible()) await guide.click();
    }
    await page.getByRole('status').filter({ hasText: 'Exact case geometry ready.' }).waitFor();
    await saved();
    const accepted = await document();
    if (operation === 'page') {
      const scope = JSON.parse(fs.readFileSync(path.join(ROOT, 'cad/test/fixtures/performance/U1-scope.json')));
      assert.equal(accepted.id, scope.projectId);
      assert.equal(accepted.formatVersion, scope.formatVersion);
      assert.equal(accepted.revision, scope.revision);
      assert.equal(accepted.boards.length, 1);
      assert.equal(accepted.boards[0].id, scope.boardId);
      assert.deepEqual(accepted.boards[0].outlineIds, [scope.outlineId]);
      assert.deepEqual(accepted.outline.map(outline => outline.id), [scope.outlineId]);
    }
    assert.equal(accepted.matrices.length, 1);
    assert.equal(accepted.matrices[0].rows, 2);
    assert.equal(accepted.matrices[0].columns, 2);
    await page.getByRole('treeitem', { name: /Main board case assembly$/ }).click();
    for (const name of ['Assembled', 'Shaded', 'Isometric']) {
      await page.getByRole('button', { name, exact: true }).click();
    }
    const frameBefore = await page.evaluate(() => window.__u1.drawCalls);
    await page.getByRole('button', { name: 'Fit', exact: true }).click();
    await page.waitForFunction(before => window.__u1.drawCalls > before, frameBefore);
    await page.evaluate(() => window.__u1Gl.finish());
    mark('accepted-Case-frame-after-camera-and-WebGL-finish');
    const viewer = page.getByRole('img', { name: /^Interactive 3D Case preview/ });
    const box = await viewer.boundingBox();
    assert(box && box.width > 0 && box.height > 0);
    await page.screenshot({ path: path.join(output, 'case-rendered.png') });
    const rendering = await page.evaluate(() => ({ drawCalls: window.__u1.drawCalls,
      completion: 'WebGL2.finish after draw following Fit', glVersion: window.__u1Gl.getParameter(window.__u1Gl.VERSION),
      selection: [...document.querySelectorAll('[aria-selected=true]')].map(n => n.getAttribute('aria-label') || n.textContent) }));
    await page.locator('#m1-tab-Export').click();
    await page.locator('.m1-export-workspace').waitFor();
    const downloads = [];
    if (operation === 'freeze') {
      downloads.push(await download(page.getByRole('button', { name: 'Download .boardstudio project', exact: true }), 'U1.boardstudio'));
    }
    downloads.push(await download(page.getByRole('button', { name: 'Export Generated mechanical package', exact: true }), 'mechanical.zip'));
    downloads.push(await download(page.getByRole('button', { name: 'Export Keycap STLs', exact: true }), 'keycaps.zip'));
    const after = await document();
    assert.deepEqual(after, accepted, 'render/export cannot alter the accepted project');
    const workers = await page.evaluate(() => window.__u1);
    const prints = workers.requests.filter(request => request.operation === 'print-keycaps');
    assert.equal(prints.length, 1, 'one full Runtime print request');
    assert.equal(prints[0].keycaps.specs.length, 4);
    assert.equal(workers.printBodies.length, 4);
    assert(prints.every(request => request.identity.revision === accepted.revision));
    assert(workers.replies.some(reply => reply.frame.operation === 'preview' && reply.frame.outcome === 'completed'
      && reply.frame.identity.revision === accepted.revision), 'accepted-revision CAD reply precedes the rendered frame');
    assert(workers.replies.some(reply => ['exact', 'export-step'].includes(reply.frame.operation)
      && reply.frame.outcome === 'completed' && reply.stepBytes > 0), 'real STEP worker completes');
    assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(output, 'accepted-project.json'), JSON.stringify(accepted, null, 2) + '\n');
    fs.writeFileSync(path.join(output, 'page-workers.log'), JSON.stringify(workers) + '\n');
    const receipt = { validationOnly: true, projectId: accepted.id, revision: accepted.revision,
      boardId: accepted.boards[0].id, viewport: [1513,1043], camera: ['Assembled','Shaded','Isometric','Fit'],
      rendering, viewer: box, downloads, comparisonPeer: 'not exercised here; requires its separately built/admitted full page and worker',
      phases, values: 'diagnostic-not-benchmark' };
    fs.writeFileSync(path.join(output, 'page-validation.log'), JSON.stringify(receipt, null, 2) + '\n');
    return receipt;
  } catch (error) {
    fs.writeFileSync(path.join(output, 'page-failure.log'), `${error.stack}\n${await page.locator('body').innerText().catch(() => '')}\n`);
    throw error;
  } finally { await page.close(); }
};
