// 011 capture driver, deliberately separate from product code. Requires a built packaged worker.
const { chromium } = require('playwright-core');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { performance } = require('node:perf_hooks');

const [origin, backend, profile, workload, output, pair, sequence] = process.argv.slice(2);
const [fixture, operation] = workload.split(':');
let activeBrowser;
const csv = values => values.map(value => `"${String(value ?? '').replaceAll('"', '""')}"`).join(',');
const validation = sequence === 'validation';
const metrics = fs.createWriteStream(path.join(output, validation ? 'diagnostic.csv' : 'runtime.csv'));
metrics.write(csv(['backend', 'profile', 'workload', 'pair', 'state', 'ordinal', 'metric', 'value', 'unit', 'status']) + '\n');
const event = (name, data = {}) => fs.appendFileSync(path.join(output, 'events.log'),
  JSON.stringify({ unix_ms: Date.now(), name, ...data }) + '\n');
function metric(state, ordinal, name, value, unit = 'ms', status = 'observed') {
  assert(Number.isFinite(value) && value >= 0);
  metrics.write(csv([backend, profile, workload, pair, state, ordinal, name, value, unit, validation ? 'diagnostic-not-benchmark' : status]) + '\n');
}

(async () => {
  event('purpose', { validationOnly: validation, values: validation ? 'diagnostic-not-benchmark' : 'observation-awaiting-admission' });
  const warmStartup = workload === 'startup:warm';
  const flags = ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'];
  const browser = activeBrowser = await chromium.launch({ executablePath: process.env.CHROME_BIN, args: flags });
  event('browser', { version: browser.version(), executable: process.env.CHROME_BIN, flags,
    viewport: [1513, 1043], gc: 'natural', httpCache: warmStartup, serviceWorkers: warmStartup ? 'allow' : 'block' });
  const context = await browser.newContext({ viewport: { width: 1513, height: 1043 },
    serviceWorkers: warmStartup ? 'allow' : 'block' });
  const page = await context.newPage();
  page.setDefaultTimeout(180000);
  const resources = [];
  context.on('response', response => resources.push({ url: response.url(), status: response.status(),
    serviceWorker: response.fromServiceWorker() }));
  const pageErrors = [];
  page.on('pageerror', error => { pageErrors.push(String(error)); event('page-error', { error: String(error) }); });
  const cdp = await context.newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.setCacheDisabled', { cacheDisabled: !warmStartup });
  let failed = false;
  try {
    if (fixture === 'startup') {
      if (warmStartup) {
        await page.goto(`${origin}/boardstudio/`);
        await page.getByRole('button', { name: 'Create new keyboard', exact: true }).waitFor();
        event('prime-navigation');
        event('service-worker-state', await page.evaluate(async () => {
          const registrations = await navigator.serviceWorker.getRegistrations();
          if (registrations.length) await Promise.race([navigator.serviceWorker.ready,
            new Promise((_, reject) => setTimeout(() => reject(Error('service-worker activation watchdog: 180 seconds')), 180000))]);
          return { registrations: registrations.map(r => ({ scope: r.scope, active: r.active?.state,
            installing: r.installing?.state, waiting: r.waiting?.state })), controller: !!navigator.serviceWorker.controller };
        }));
      }
      event('navigation-start');
      const start = performance.now();
      await page.goto(`${origin}/boardstudio/`);
      await page.getByRole('button', { name: 'Create new keyboard', exact: true }).waitFor();
      metric(operation, 0, 'navigation-to-usable-landing', performance.now() - start);
      event('landing-ready');
      // A landing page is not a frozen project's usable editor or first rendered Case frame.
    }
    await page.goto(`${origin}/__perf/cad/scripts/performance-host.html`);
    const result = await page.evaluate(async ({ backend, fixture, operation, sequence, isStartup }) => {
      const fail = message => { throw new Error(message); };
      const check = (condition, message) => { if (!condition) fail(message); };
      const json = async url => {
        const response = await fetch(url); check(response.ok, `missing ${url}`); return response.json();
      };
      const prepared = ['C1plate', 'C2trayboss', 'C3gasket', 'P1analyticPCB'].includes(fixture)
        ? await json(`/__perf/cad/test/fixtures/performance/${fixture}.json`) : null;
      const keycaps = fixture === 'K1keycap'
        ? await json('/__perf/cad/test/fixtures/performance/K1keycap.json') : null;
      const contract = fixture === 'Qvendor'
        ? await json('/__perf/cad/wasm/tests/fixtures/generated-analysis-contract.json') : null;
      let vendor;
      if (fixture === 'V1switch' || fixture === 'V2vendor') {
        const manifest = await json(`/__perf/cad/test/fixtures/${fixture === 'V1switch' ? 'followup-manifest' : 'manifest'}.json`);
        const entries = Array.isArray(manifest) ? manifest : manifest.fixtures;
        vendor = entries.find(entry => entry.id === (fixture === 'V1switch' ? 'imported-switch' : 'imported-multi-solid'));
        check(vendor, 'missing pinned vendor entry');
      } else if (contract) {
        vendor = { file: contract.qualificationV2.activation.directVendorRecipe.originalAsset,
          sha256: contract.qualificationV2.queryExamples.binding.leftAnalysis.source.sourceSha256 };
      }
      let bytes;
      if (vendor) {
        const response = await fetch(`/__source/${vendor.file}`); check(response.ok, 'missing original STEP');
        bytes = new Uint8Array(await response.arrayBuffer());
        const sha = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)),
          v => v.toString(16).padStart(2, '0')).join('');
        check(sha === vendor.sha256, 'original STEP authentication failed');
      }
      const revision = contract ? contract.qualificationV2.activation.snapshot.revision : (prepared?.revision ?? keycaps?.revision ?? 1);
      const identity = contract ? { ...contract.qualificationV2.activation.snapshot,
        token: Number(contract.qualificationV2.activation.snapshot.token),
        sessionEpoch: Number(contract.qualificationV2.activation.snapshot.sessionEpoch) }
        : { token: 11, sessionEpoch: 2, documentId: 'performance', boardId: prepared?.bodies[0].body.boardId ?? 'performance',
          instanceId: null, revision };
      let worker, sequenceId = 0;
      const waiting = new Map(), startup = [];
      const startWorker = async () => {
        worker?.terminate();
        const start = performance.now();
        worker = new Worker('/boardstudio/assets/cad-worker/entry.js', { type: 'module' });
        await new Promise((resolve, reject) => {
          const timer = setTimeout(() => reject(Error('worker ready watchdog: 180 seconds')), 180000);
          worker.onerror = e => { clearTimeout(timer); reject(Error(e.message)); };
          worker.onmessage = e => {
            const message = e.data;
            if (message.kind === 'ready') { clearTimeout(timer); resolve(); }
            if (message.kind === 'init-failed') { clearTimeout(timer); reject(Error(JSON.stringify(message))); }
            if (message.kind === 'reply') {
              const frame = JSON.parse(message.frame), pending = waiting.get(frame.requestId);
              if (pending) { waiting.delete(frame.requestId); clearTimeout(pending.timer); pending.resolve({ frame, result: message.result }); }
            }
          };
        });
        startup.push(performance.now() - start);
      };
      const request = async (op, fields, payload = {}) => {
        const requestId = `performance-${++sequenceId}`;
        return new Promise((resolve, reject) => {
          const timer = setTimeout(() => { waiting.delete(requestId); reject(Error(`${op} watchdog: 180 seconds`)); }, 180000);
          waiting.set(requestId, { resolve, timer });
          worker.postMessage({ kind: 'request', frame: JSON.stringify({ requestId, jobId: requestId, identity, operation: op, ...fields }),
            ...payload }, Object.values(payload).filter(v => v instanceof ArrayBuffer));
        });
      };
      const completed = async (op, fields, payload) => {
        const reply = await request(op, fields, payload);
        check(reply.frame.outcome === 'completed', reply.frame.error ?? 'worker failed');
        check(JSON.stringify(reply.frame.identity) === JSON.stringify(identity), 'captured identity drift');
        return reply.result;
      };
      await startWorker();
      let queryFields, queryPayload;
      if (contract) {
        // Mint real owned references. Only the authored rows/threshold/limits come from the fixture.
        const imported = await completed('read-step', { sourceSha256: vendor.sha256,
          stepImportOptions: { previewRepairBudgetMm: null, targetDeflectionMm: null } }, { inputBytes: bytes.slice().buffer });
        const asset = imported.vendorStep, example = contract.qualificationV2.queryExamples, b = example.binding;
        check(asset.sourceFaces.length === 30 && asset.faceOccurrences.length === 30, '0805 source census');
        const operand = side => ({ modelId: b[`${side}ModelId`], sourceSha256: vendor.sha256,
          importOptions: asset.importOptions, rowTransformF64: b[`${side}RowTransformF64`],
          analysisRef: asset.analysisRef, analysisMesh: asset.analysisMesh });
        queryFields = { collisionQuery: { left: operand('left'), right: operand('right'),
          clearanceThresholdMm: b.clearanceThresholdMm, requestedAccuracyMm: operation === 'tight' ? .01 : null,
          qualificationLimits: example.limits } };
        queryPayload = () => ({ leftPositions: asset.surfacePositionsMmF64.slice().buffer,
          rightPositions: asset.surfacePositionsMmF64.slice().buffer,
          leftTags: asset.triangleFaceOccurrenceIndices.slice().buffer,
          rightTags: asset.triangleFaceOccurrenceIndices.slice().buffer });
      }
      const input = isStartup ? await json('/__perf/cad/test/fixtures/performance/C1plate.json') : prepared;
      const run = async () => {
        let op, fields, payload;
        if (input) { op = operation === 'step' ? 'export-step' : 'preview'; fields = { prepared: input }; }
        else if (keycaps) { op = operation === 'print' ? 'print-keycaps' : 'preview'; fields = { keycaps }; }
        else if (contract) { op = 'collision-query'; fields = queryFields; payload = queryPayload(); }
        else { op = 'read-step'; fields = { sourceSha256: vendor.sha256,
          stepImportOptions: { previewRepairBudgetMm: null, targetDeflectionMm: operation === 'refine' ? .01 : null } };
          payload = { inputBytes: bytes.slice().buffer }; }
        const start = performance.now();
        const reply = await completed(op, fields, payload);
        const duration = performance.now() - start;
        // All traversal/quality controls are outside the operation timer. Full independent QA is a separate admission.
        const meshes = reply.bodies ?? (reply.mesh ? [reply.mesh] : []);
        if (input || keycaps) check(reply.revision === revision, 'stale result revision');
        let bufferBytes = 0, triangles = 0;
        for (const mesh of meshes) {
          check(mesh.positions instanceof Float32Array && mesh.normals instanceof Float32Array
            && mesh.positions.length > 0 && mesh.positions.length % 9 === 0
            && mesh.positions.length === mesh.normals.length && mesh.positions.every(Number.isFinite)
            && mesh.normals.every(Number.isFinite), 'invalid mesh');
          bufferBytes += mesh.positions.byteLength + mesh.normals.byteLength;
          triangles += mesh.positions.length / 9;
        }
        if (input && op === 'preview') check(JSON.stringify(reply.bodies.map(b => b.id)) ===
          JSON.stringify(input.bodies.map(b => b.body.id)), 'missing/changed generated bodies');
        if (input && op === 'export-step') check(reply.step instanceof Uint8Array && reply.step.length > 0
          && !reply.mesh && !reply.bodies, 'invalid STEP-only output');
        if (keycaps) {
          const expected = keycaps.specs.flatMap(s => operation === 'print' ? [`keycap:${s.id}`]
            : [`keycap:${s.id}`, `keycap-legend:${s.id}`]);
          check(JSON.stringify(reply.bodies.map(b => b.id)) === JSON.stringify(expected), 'incomplete cap/legend bodies');
        }
        let metadata = {};
        if (vendor && !contract) {
          metadata = { bounds: reply.bounds };
          if (backend !== 'cadrum') {
            const asset = reply.vendorStep, count = fixture === 'V1switch' ? 155 : 529;
            check(asset.sourceSha256 === vendor.sha256 && asset.sourceFaces.length === count
              && asset.faceOccurrences.length === count && new Set(asset.triangleFaceOccurrenceIndices).size === count,
            'incomplete vendor source/occurrence coverage');
            metadata = { bounds: reply.bounds, precision: asset.precision, accuracy: asset.accuracy,
              importOptions: asset.importOptions, analysisMesh: asset.analysisMesh,
              sourceQualification: asset.sourceQualification, faces: asset.sourceFaces.length,
              occurrences: asset.faceOccurrences.length, lines: asset.lineOccurrences.length };
            bufferBytes += asset.surfacePositionsMmF64.byteLength + asset.triangleFaceOccurrenceIndices.byteLength
              + asset.lineSegmentsMmF64.byteLength + asset.segmentLineOccurrenceIndices.byteLength
              + (asset.lineRenderPositions?.byteLength ?? 0);
          }
        }
        if (contract) {
          check(reply.queryQualification?.binding?.leftAnalysis?.meshIdentity === queryFields.collisionQuery.left.analysisRef.meshIdentity,
            'query lost owned reference');
          metadata = { collision: reply.collision, queryQualification: reply.queryQualification };
        }
        window.__lastResult = reply;
        return { duration, triangles, bufferBytes, stepBytes: reply.step?.byteLength ?? 0, metadata };
      };
      window.__measure = { run, startWorker, startup, drop: () => { window.__lastResult = null; } };
      return { startup, input: input ?? keycaps ?? { source: vendor, targetDeflectionMm: operation === 'refine' ? .01 : null },
        identity, settings: { linearMm: .1, angularRad: .5 }, mode: sequence };
    }, { backend, fixture, operation, sequence, isStartup: fixture === 'startup' });
    event('prepared', result);
    result.startup.forEach((ms, i) => metric('worker-startup', i, 'worker-create-to-ready', ms));
    // A fixed 100ms dwell supplies idle/retained samples at 50ms; it is never inside latency or a retry.
    const dwell = () => new Promise(resolve => setTimeout(resolve, 100));
    await dwell();
    event('idle-baseline');
    const count = validation || fixture === 'startup' ? 1 : sequence === 'growth20' ? 20 : 6;
    for (let i = 0; i < count; ++i) {
      const state = sequence === 'growth20' ? 'growth' : i === 0 ? 'cold' : 'warm';
      if (operation === 'step' && i > 0) {
        await page.evaluate(() => window.__measure.startWorker());
        event('fresh-export-worker-ready', { ordinal: i });
      }
      event('operation-start', { state, ordinal: i });
      const sample = await page.evaluate(() => window.__measure.run());
      event('reply-retained', { state, ordinal: i, ...sample });
      metric(state, i, 'dispatch-to-validated-reply', sample.duration);
      for (const name of ['triangles', 'bufferBytes', 'stepBytes']) metric(state, i, name, sample[name], name === 'triangles' ? 'count' : 'bytes');
      await dwell();
      await page.evaluate(() => window.__measure.drop());
      event('references-dropped', { state, ordinal: i });
      await dwell();
    }
    event('scope-limit', { print: 'print-keycaps reply only; Runtime binary-STL/ZIP/download remains U1 admission',
      startup: 'landing plus isolated real worker/reply; frozen editor/project and rendered frame remain U1 admission',
      memory: 'process PSS/RSS in memory.csv; worker WASM linear capacity unavailable through production API',
      geometry: 'shape/census checks supplement required independent untimed admission; observed is not accepted' });
    assert.deepEqual(pageErrors, [], 'page errors invalidate the observation');
  } catch (error) {
    failed = true;
    event('failure', { error: String(error.stack ?? error) });
    process.exitCode = 1;
  } finally {
    event('resources', { responses: resources });
    event('result', { failed });
    metrics.end();
    await browser.close();
    activeBrowser = null;
  }
})().catch(async error => {
  event('failure', { error: String(error.stack ?? error) }); metrics.end(); process.exitCode = 1;
  await activeBrowser?.close();
});
