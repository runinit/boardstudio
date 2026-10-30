// Controlled protocol self-checks only: no browser/WASM/network/timing acceptance.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { webcrypto } = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const source = fs.readFileSync(path.join(__dirname, 'measure-cad-runtime.cjs'), 'utf8');

async function check(workload, incomplete = false, sequence = 'latency') {
  const directory = fs.mkdtempSync(path.join(root, '.evidence/011-preparation/protocol-'));
  const requests = [], workers = [], metrics = [];
  const fakeProcess = { argv: ['node', 'driver', 'http://controlled.invalid', 'monstertruck',
    'checked', workload, directory, '1', sequence], env: { CHROME_BIN: 'controlled-no-browser' } };
  let closed = false;
  const mesh = id => ({ id, positions: new Float32Array([0,0,0, 1,0,0, 0,1,0]),
    normals: new Float32Array([0,0,1, 0,0,1, 0,0,1]) });
  class ControlledWorker {
    constructor() { workers.push(this); queueMicrotask(() => this.onmessage({ data: { kind: 'ready' } })); }
    terminate() {}
    postMessage(message) {
      const frame = JSON.parse(message.frame);
      requests.push(frame);
      let result;
      if (frame.operation === 'preview') result = { revision: frame.identity.revision,
        bodies: frame.prepared.bodies.map(body => mesh(body.body.id)) };
      else if (frame.operation === 'export-step') result = { revision: frame.identity.revision, step: new Uint8Array([1,2,3]) };
      else if (frame.operation === 'read-step') {
        const count = incomplete ? 154 : 155;
        result = { mesh: mesh('vendor'), bounds: {}, vendorStep: {
          sourceSha256: frame.sourceSha256, sourceFaces: Array.from({length: count}, (_, i) => ({ faceId: i })),
          faceOccurrences: Array.from({length: count}, (_, i) => ({ sourceFaceIndex: i })),
          triangleFaceOccurrenceIndices: Uint32Array.from({length: count}, (_, i) => i),
          surfacePositionsMmF64: new Float64Array(count*9), lineSegmentsMmF64: new Float64Array(),
          segmentLineOccurrenceIndices: new Uint32Array(), lineOccurrences: [],
        } };
      } else throw Error('unexpected controlled operation');
      queueMicrotask(() => this.onmessage({ data: { kind: 'reply', frame: JSON.stringify({ ...frame, outcome: 'completed' }), result } }));
    }
  }
  const page = { on() {}, setDefaultTimeout() {}, async goto() {}, async evaluate(fn, arg) { return fn(arg); } };
  const browser = { version: () => 'controlled', async newContext() { return {
    on() {}, async newPage() { return page; }, async newCDPSession() { return { async send() {} }; },
  }; }, async close() { closed = true; } };
  const context = vm.createContext({
    require: name => name === './measure-cad-worker.cjs' ? vm.runInContext(`(() => { const module = { exports: null }; ${fs.readFileSync(path.join(__dirname, 'measure-cad-worker.cjs'), 'utf8')} return module.exports; })()`, context)
      : name === 'playwright-core' ? { chromium: { async launch() { return browser; } } }
      : name === 'node:fs' ? { ...fs, createWriteStream() { return { write(text) { metrics.push(text); }, end() {} }; } }
        : require(name),
    process: fakeProcess, console, Worker: ControlledWorker, window: {}, crypto: webcrypto,
    Float32Array, Float64Array, Uint8Array, Uint32Array, ArrayBuffer,
    setTimeout: (fn, ms) => ms === 100 ? setTimeout(fn, 0) : setTimeout(fn, ms), clearTimeout,
    fetch: async url => {
      const file = path.join(root, url.replace(/^\/__(?:perf|source)\//, ''));
      return { ok: fs.existsSync(file), async json() { return JSON.parse(fs.readFileSync(file, 'utf8')); },
        async arrayBuffer() { const data = fs.readFileSync(file); return data.buffer.slice(data.byteOffset, data.byteOffset + data.length); } };
    },
  });
  await vm.runInContext(source, context, { filename: 'measure-cad-runtime.cjs' });
  assert(closed, 'owned browser is closed on success and rejection');
  assert.equal(fakeProcess.exitCode ?? 0, incomplete ? 1 : 0);
  if (incomplete) {
    assert(fs.readFileSync(path.join(directory, 'events.log'), 'utf8').includes('incomplete vendor source/occurrence coverage'));
  } else {
    const count = sequence === 'validation' ? 1 : 6;
    assert.equal(requests.length, count, 'validation is one observation; latency has one cold plus five warm');
    assert.equal(new Set(requests.map(r => r.requestId)).size, count, 'unique correlation IDs');
    assert.equal(workers.length, workload.endsWith(':step') ? count : 1, 'fresh module for every STEP export');
    assert(metrics.join('').includes('dispatch-to-validated-reply'));
    if (sequence === 'validation') assert(metrics.join('').includes('diagnostic-not-benchmark'));
  }
}

(async () => {
  fs.mkdirSync(path.join(root, '.evidence/011-preparation'), { recursive: true });
  await check('C1plate:preview');
  await check('C1plate:step');
  await check('V1switch:import');
  await check('V1switch:import', true);
  await check('C1plate:preview', false, 'validation');
  const { stl, oracle } = require('./check-cad-page-downloads.cjs');
  assert.throws(() => stl(Buffer.alloc(83)), /truncated/);
  const truncated = Buffer.alloc(84); truncated.writeUInt32LE(1,80);
  assert.throws(() => stl(truncated), /complete binary STL/);
  assert.equal(typeof oracle().roofAndCavity, 'function');
  console.log('5 controlled protocol checks plus STL decoder/oracle controls passed; no browser/provider executed.');
})().catch(error => { console.error(error); process.exitCode = 1; });
