// Controlled checks of the worker protocol shared by both capture drivers.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { webcrypto } = require('node:crypto');
const prepare = require('./measure-cad-worker.cjs');
const root = path.resolve(__dirname, '../..');

async function check(workload, fault = null, sequence = 'latency') {
  const [fixture, operation] = workload.split(':');
  const requests = [], workers = [];
  const timeout = global.setTimeout;
  global.setTimeout = (fn, ms) => timeout(fn, ms === 180000 ? (['correlation', 'null', 'missing'].includes(fault) ? 100 : 2000) : ms);
  global.window = {};
  Object.defineProperty(global, 'crypto', { value: webcrypto, configurable: true });
  global.fetch = async url => {
    const file = path.join(root, url.replace(/^\/__(?:perf|source)\//, ''));
    return { ok: fs.existsSync(file), async json() { return JSON.parse(fs.readFileSync(file, 'utf8')); },
      async arrayBuffer() { const data = fs.readFileSync(file); return data.buffer.slice(data.byteOffset, data.byteOffset + data.length); } };
  };
  const mesh = id => ({ id, positions: new Float32Array([0,0,0, 1,0,0, 0,1,0]),
    normals: new Float32Array([0,0,1, 0,0,1, 0,0,1]) });
  global.Worker = class {
    constructor() { workers.push(this); queueMicrotask(() => this.onmessage({ data: { kind: 'ready' } })); }
    terminate() { this.terminated = true; }
    postMessage(message) {
      const frame = JSON.parse(message.frame);
      requests.push(frame);
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
    const prepared = await prepare({ backend: 'monstertruck', fixture, operation, sequence, isStartup: false });
    assert.equal(prepared.startup.length, 1);
    const errors = { malformed: /malformed worker reply/, null: /malformed worker reply/, missing: /malformed worker reply/,
      identity: /captured identity drift/, epoch: /captured identity drift/, token: /captured identity drift/,
      revision: /stale result revision/, mesh: /invalid mesh/, bodies: /missing\/changed generated bodies/,
      step: /invalid STEP-only output/, vendor: /incomplete vendor source\/occurrence coverage/,
      failure: /controlled worker failure/, correlation: /preview watchdog/ };
    if (fault) {
      await assert.rejects(window.__measure.run(), errors[fault]);
    } else {
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
    global.setTimeout = timeout;
  }
}

(async () => {
  for (const workload of ['C1plate:preview', 'C1plate:step', 'V1switch:import']) await check(workload);
  await check('C1plate:preview', null, 'validation');
  for (const fault of ['malformed', 'null', 'missing', 'identity', 'epoch', 'token', 'revision', 'mesh', 'bodies', 'failure', 'correlation'])
    await check('C1plate:preview', fault);
  await check('C1plate:step', 'step');
  await check('V1switch:import', 'vendor');
  const { stl, oracle } = require('./check-cad-page-downloads.cjs');
  assert.throws(() => stl(Buffer.alloc(83)), /truncated/);
  const truncated = Buffer.alloc(84); truncated.writeUInt32LE(1,80);
  assert.throws(() => stl(truncated), /complete binary STL/);
  assert.equal(typeof oracle().roofAndCavity, 'function');
  console.log('shared CAD worker protocol and STL controls passed; no browser/provider executed');
})().catch(error => { console.error(error); process.exitCode = 1; });
