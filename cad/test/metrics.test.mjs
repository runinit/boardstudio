import assert from 'node:assert/strict';
import test from 'node:test';
import { previewAssembly, buildAssembly, readStepModel } from '../src/index.ts';
import { beginCadMetrics, cadBody, cadStage, endCadMetrics, sampleCadMemory } from '../src/metrics.ts';

const square = (min, max) => [{ x: min, y: min }, { x: max, y: min }, { x: max, y: max }, { x: min, y: max }];
const ir = { revision: 1, bodies: [{ revision: 1, body: { id: 'metrics', name: 'Metrics', kind: 'plate', thickness: 2 },
  regions: [0, 20].map(x => ({ outer: square(x, x + 10), holes: [], cavities: [], gaskets: [], mounts: [] })),
}] };

async function profile(run) {
  const metrics = beginCadMetrics();
  try { return { result: await run(), metrics }; } finally { endCadMetrics(); }
}

test('metrics distinguish cold construction, cache reuse, local edits and STEP work without changing meshes', async () => {
  const first = await profile(() => previewAssembly(ir, () => {}));
  assert.equal(first.metrics.stages.kernelInitialization.calls, 1);
  assert.equal(first.metrics.counters.bodyCacheMisses, 1);
  assert.equal(first.metrics.counters.regionCacheMisses, 2);
  assert.equal(first.metrics.counters.extrusions, 2);
  assert.equal(first.metrics.counters.exactPlanarMeshes, 2);
  assert.equal(first.metrics.counters.tessellations, undefined, 'eligible exact plates bypass kernel meshing');
  assert.equal(first.metrics.counters.tessellatedTriangles, first.result.mesh.positions.length / 9);
  for (const stage of ['baseExtrusion', 'planarPlateMesh', 'wasmMeshCopy', 'combinedMeshCopy']) {
    assert.ok(first.metrics.stages[stage].durationMs >= 0, stage);
  }
  assert.equal(first.metrics.stages.stepSerialization, undefined);
  assert.equal(first.metrics.stages.cacheSolidCopy, undefined, 'preview assembly must share immutable region solids');
  assert.deepEqual(first.metrics.bodies.map(body => body.id), ['metrics']);
  assert.ok(first.metrics.bodies[0].stages.planarPlateMesh >= 0);
  assert.ok((first.metrics.stages.bodyYield?.calls ?? 0) <= 1);
  assert.ok(first.metrics.wasmAllocatedBytes.start > 0);
  assert.ok(first.metrics.wasmAllocatedBytes.peak >= first.metrics.wasmAllocatedBytes.start);

  // Transferring a response must not detach Rust cache storage.
  const transferred = structuredClone(first.result, { transfer: [first.result.mesh.positions.buffer, first.result.mesh.normals.buffer,
    ...first.result.bodies.flatMap(body => [body.positions.buffer, body.normals.buffer])] });
  assert.equal(first.result.mesh.positions.byteLength, 0);
  const hit = await profile(() => previewAssembly(ir, () => {}));
  assert.deepEqual(hit.result.mesh, transferred.mesh);
  assert.equal(hit.metrics.counters.bodyCacheHits, 1);
  assert.equal(hit.metrics.counters.tessellations, undefined);
  assert.equal(hit.metrics.stages.cacheMeshCopy, undefined, 'cache hits borrow immutable storage before copying the response');
  assert.equal(hit.metrics.stages.kernelInitialization, undefined);

  const edit = structuredClone(ir);
  edit.bodies[0].body.openings = [{ points: square(22, 24), z: 0, height: 2 }];
  const changed = await profile(() => previewAssembly(edit, () => {}));
  assert.equal(changed.metrics.counters.regionCacheHits, 1);
  assert.equal(changed.metrics.counters.regionCacheMisses, 1);
  assert.equal(changed.metrics.counters.booleanEvaluations, 1);
  assert.equal(changed.metrics.stages.openingCuts.calls, 1);

  const exported = await profile(() => buildAssembly(edit));
  assert.equal(exported.metrics.counters.bodyCacheHits, 1);
  assert.equal(exported.metrics.counters.tessellations, undefined);
  assert.equal(exported.metrics.stages.stepSerialization.calls, 1);
  assert.deepEqual(exported.result.mesh, changed.result.mesh);
  const imported = await profile(() => readStepModel(exported.result.step));
  assert.equal(imported.metrics.stages.stepImport.calls, 1);
  assert.equal(imported.metrics.stages.wasmStepImportCopy.calls, 1);
  assert.equal(imported.metrics.counters.tessellations, 1);
  assert.ok(imported.result.mesh.positions.length > 0);

  const disabled = await previewAssembly(edit, () => {});
  assert.deepEqual(disabled, changed.result);
  assert.equal(globalThis.__boardstudioCadMetrics, undefined);
});

test('body diagnostics are opt-in, bounded and reset between requests', () => {
  cadBody('disabled', 'Disabled')();
  assert.equal(globalThis.__boardstudioCadMetrics, undefined);
  const metrics = beginCadMetrics();
  for (let index = 0; index < 140; index++) {
    const finish = cadBody(String(index), 'Body');
    cadStage('test')();
    finish();
  }
  endCadMetrics();
  assert.equal(metrics.bodies.length, 128);
  assert.equal(metrics.counters.omittedBodyTimings, 12);
  const next = beginCadMetrics();
  assert.equal(next.bodies, undefined);
  endCadMetrics();
});

test('direct tray export measures every existing Boolean stage without using preview caches', async () => {
  const tray = { revision: 3, bodies: [{ revision: 3,
    body: { id: 'tray-metrics', name: 'Tray', kind: 'tray', thickness: 2, wallHeight: 5,
      gasket: { depth: 0.5 }, openings: [{ points: [{ x: -1, y: 20 }, { x: 4, y: 20 }, { x: 4, y: 24 }, { x: -1, y: 24 }], z: 2, height: 2 }] },
    regions: [{ outer: square(0, 50), holes: [], cavities: [square(3, 47)],
      gaskets: [{ outer: square(0.5, 49.5), holes: [square(1.5, 48.5)] }],
      mounts: [{ kind: 'boss', at: { x: 10, y: 10 }, height: 3, bossDiameter: 5, holeDiameter: 2 }],
    }],
  }] };
  const { result, metrics } = await profile(() => buildAssembly(tray));
  for (const stage of ['baseExtrusion', 'cavityCuts', 'gasketCuts', 'bossUnions', 'mountHoleCuts', 'openingCuts', 'stepSerialization']) {
    assert.equal(metrics.stages[stage].calls, stage === 'bossUnions' ? 2 : 1, stage);
  }
  assert.equal(metrics.counters.bodyCacheMisses, 1);
  assert.equal(metrics.counters.booleanEvaluations, 5);
  assert.equal(metrics.counters.cylinders, 2);
  assert.ok(result.step.length > 0);
});

test('allocated-memory sampling refreshes the buffer after growth and resets between requests', () => {
  const memory = new WebAssembly.Memory({ initial: 1 });
  const first = beginCadMetrics();
  sampleCadMemory(memory);
  memory.grow(2);
  sampleCadMemory(memory);
  endCadMetrics();
  assert.deepEqual(first.wasmAllocatedBytes, { start: 65536, end: 196608, peak: 196608 });
  const second = beginCadMetrics();
  assert.deepEqual(second.wasmAllocatedBytes, { start: null, end: null, peak: null });
  assert.deepEqual(second.counters, {});
  endCadMetrics();
});

test('opening edits reuse bounded upstream solids while still rebuilding cuts and final meshes', async () => {
  const { readFile } = await import('node:fs/promises');
  const { prepareAssembly } = await import('./native-prepare.mjs');
  const raw = JSON.parse(await readFile(new URL('./fixtures/boss-tray.json', import.meta.url)));
  const prepared = prepareAssembly(raw);
  const first = await profile(() => previewAssembly(prepared, () => {}));
  assert.equal(first.metrics.counters.upstreamStageMisses, 1);
  const edited = structuredClone(prepared);
  edited.bodies[0].body.openings[0].points[0].x += 0.1;
  const changed = await profile(() => previewAssembly(edited, () => {}));
  assert.equal(changed.metrics.counters.upstreamStageHits, 1);
  assert.equal(changed.metrics.stages.baseExtrusion, undefined);
  assert.equal(changed.metrics.stages.bossUnions, undefined);
  assert.equal(changed.metrics.stages.mountHoleCuts, undefined);
  assert.equal(changed.metrics.stages.openingCuts.calls, 1);
  assert.equal(changed.metrics.counters.tessellations, 1);
  const exported = await buildAssembly(edited);
  assert.deepEqual(exported.mesh, changed.result.mesh);
  const geometry = structuredClone(edited);
  geometry.bodies[0].body.thickness += 0.1;
  const invalidated = await profile(() => previewAssembly(geometry, () => {}));
  assert.equal(invalidated.metrics.counters.upstreamStageMisses, 1);
});
