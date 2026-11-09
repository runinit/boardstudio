// Worker protocol shared by both capture drivers: the production measure-cad-worker.cjs, injected as
// cad_capture.rs injects it, against a fake Worker in an isolated browser page. No wasm is executed.
import { assert, decodeText, readBytes, readText, runCases } from './provider-support.mjs';

const errors = { malformed: /malformed worker reply/, null: /malformed worker reply/, missing: /malformed worker reply/,
  identity: /captured identity drift/, epoch: /captured identity drift/, token: /captured identity drift/,
  revision: /stale result revision/, mesh: /invalid mesh/, bodies: /missing\/changed generated bodies/,
  step: /invalid STEP-only output/, vendor: /incomplete vendor source\/occurrence coverage/,
  failure: /controlled worker failure/, correlation: /preview watchdog/ };

async function check(files, prepare, workload, fault = null, sequence = 'latency') {
  const [fixture, operation] = workload.split(':');
  const requests = [], workers = [], timers = new Set();
  const timeout = window.setTimeout, clear = window.clearTimeout, WorkerClass = window.Worker, fetchOriginal = window.fetch;
  const oldMeasure = Object.getOwnPropertyDescriptor(window, '__measure');
  const oldResult = Object.getOwnPropertyDescriptor(window, '__lastResult');
  window.setTimeout = (fn, ms) => {
    const timer = timeout(() => { timers.delete(timer); fn(); }, ms === 180000 ? (['correlation', 'null', 'missing'].includes(fault) ? 100 : 2000) : ms);
    timers.add(timer);
    return timer;
  };
  window.clearTimeout = timer => { timers.delete(timer); clear(timer); };
  window.fetch = async url => {
    const data = files.get(url.replace(/^\/__(?:perf|source)\//, ''));
    return { ok: Boolean(data), async json() { return JSON.parse(decodeText(data)); },
      async arrayBuffer() {
        const copy = data.slice();
        if (fault === 'source-sha' && url.startsWith('/__source/')) copy[0] ^= 1;
        return copy.buffer;
      } };
  };
  const mesh = id => ({ id, positions: new Float32Array([0,0,0, 1,0,0, 0,1,0]),
    normals: new Float32Array([0,0,1, 0,0,1, 0,0,1]) });
  window.Worker = class {
    constructor() { workers.push(this); queueMicrotask(() => this.onmessage({ data: { kind: 'ready' } })); }
    terminate() { this.terminated = true; }
    postMessage(message) {
      const frame = JSON.parse(message.frame);
      requests.push(structuredClone(frame));
      let result;
      if (frame.operation === 'preview') result = { revision: frame.identity.revision,
        bodies: frame.prepared.bodies.map(body => mesh(body.body.id)) };
      else if (frame.operation === 'export-step') result = { revision: frame.identity.revision, step: new Uint8Array([1,2,3]) };
      else if (frame.operation === 'read-step') {
        const count = fault === 'vendor' ? 154 : 155;
        result = { mesh: mesh('vendor'), bounds: {}, vendorStep: {
          sourceSha256: frame.sourceSha256, sourceFaces: Array.from({length: count}, (_, i) => ({ faceId: i })),
          faceOccurrences: Array.from({length: count}, (_, i) => ({ sourceFaceIndex: i })),
          triangleFaceOccurrenceIndices: Uint32Array.from({length: count}, (_, i) => i),
          surfacePositionsMmF64: new Float64Array(count*9), lineSegmentsMmF64: new Float64Array(),
          segmentLineOccurrenceIndices: new Uint32Array(), lineOccurrences: [],
        } };
      } else throw Error(`unexpected operation ${frame.operation}`);
      if (fault === 'identity') frame.identity.revision++;
      if (fault === 'epoch') frame.identity.sessionEpoch++;
      if (fault === 'token') frame.identity.token++;
      if (fault === 'revision') result.revision++;
      if (fault === 'mesh') result.bodies[0].positions[0] = NaN;
      if (fault === 'bodies') result.bodies.pop();
      if (fault === 'step') result.step = new Uint8Array();
      if (fault === 'failure') frame.error = 'controlled worker failure';
      if (fault === 'correlation') frame.requestId = 'unknown';
      const reply = fault === 'malformed' ? '{' : fault === 'null' ? 'null' : fault === 'missing' ? '{}' :
        JSON.stringify({ ...frame, outcome: fault === 'failure' ? 'failed' : 'completed' });
      queueMicrotask(() => this.onmessage({ data: { kind: 'reply', frame: reply, result } }));
    }
  };
  try {
    const args = { backend: 'monstertruck', fixture, operation, sequence, isStartup: false };
    if (fault === 'source-sha') {
      await assert.rejects(() => prepare(args), /original STEP authentication failed/);
      assert.equal(workers.length, 0);
      return;
    }
    const prepared = await prepare(args);
    assert.equal(prepared.startup.length, 1);
    if (fault) await assert.rejects(() => window.__measure.run(), errors[fault]);
    else {
      const count = sequence === 'validation' ? 1 : 6;
      for (let i = 0; i < count; i++) {
        if (operation === 'step' && i) await window.__measure.startWorker();
        const sample = await window.__measure.run();
        assert(Number.isFinite(sample.duration) && sample.duration >= 0);
        assert.equal(sample.stepBytes, operation === 'step' ? 3 : 0);
        assert(window.__lastResult);
        window.__measure.drop();
        assert.equal(window.__lastResult, null);
      }
      assert.equal(requests.length, count);
      assert.equal(new Set(requests.map(r => r.requestId)).size, count);
      assert.equal(workers.length, operation === 'step' ? count : 1);
      assert(workers.slice(0, -1).every(worker => worker.terminated));
    }
  } finally {
    for (const timer of timers) clear(timer);
    for (const worker of workers) worker.terminate();
    window.setTimeout = timeout; window.clearTimeout = clear; window.Worker = WorkerClass; window.fetch = fetchOriginal;
    if (oldMeasure) Object.defineProperty(window, '__measure', oldMeasure); else delete window.__measure;
    if (oldResult) Object.defineProperty(window, '__lastResult', oldResult); else delete window.__lastResult;
  }
}

export async function run() {
  const source = await readText('cad/scripts/measure-cad-worker.cjs');
  const files = new Map([
    ['cad/test/fixtures/performance/C1plate.json', await readBytes('cad/test/fixtures/performance/C1plate.json')],
    ['cad/test/fixtures/followup-manifest.json', await readBytes('cad/test/fixtures/followup-manifest.json')],
    ['ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step',
      await readBytes('ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step')],
  ]);
  // The production injection: cad_capture.rs wraps the source in a function over `module`.
  const module = { exports: null };
  Function('module', source)(module);
  assert.equal(typeof module.exports, 'function');
  const prepare = module.exports;
  return runCases('measure-cad-worker', [
    ['preview', () => check(files, prepare, 'C1plate:preview')],
    ['step', () => check(files, prepare, 'C1plate:step')],
    ['import', () => check(files, prepare, 'V1switch:import')],
    ['validation', () => check(files, prepare, 'C1plate:preview', null, 'validation')],
    ...['malformed', 'null', 'missing', 'identity', 'epoch', 'token', 'revision', 'mesh', 'bodies', 'failure', 'correlation']
      .map(fault => [fault, () => check(files, prepare, 'C1plate:preview', fault)]),
    ['step-fault', () => check(files, prepare, 'C1plate:step', 'step')],
    ['vendor', () => check(files, prepare, 'V1switch:import', 'vendor')],
    ['source-sha', () => check(files, prepare, 'V1switch:import', 'source-sha')],
  ]);
}
