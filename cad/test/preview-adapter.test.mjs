import assert from 'node:assert/strict';
import test from 'node:test';
import { combinedPreview, previewBodies } from '../src/preview.ts';
import { beginCadMetrics, endCadMetrics } from '../src/metrics.ts';

test('legacy combined mesh is lazy, memoized, ordered and independent of body buffers', () => {
  const bodies = [1, 2].map(value => ({ id: String(value), name: String(value),
    positions: new Float32Array(9).fill(value), normals: new Float32Array(9).fill(-value) }));
  const metrics = beginCadMetrics();
  try {
    const result = combinedPreview({ revision: 7, bodies });
    assert.equal(result.revision, 7);
    assert.equal(result.bodies, bodies);
    assert.equal(metrics.stages.combinedMeshCopy, undefined);
    const mesh = result.mesh;
    assert.equal(result.mesh, mesh);
    assert.equal(metrics.stages.combinedMeshCopy.calls, 1);
    assert.deepEqual([...mesh.positions], [...bodies[0].positions, ...bodies[1].positions]);
    assert.deepEqual([...mesh.normals], [...bodies[0].normals, ...bodies[1].normals]);
    structuredClone(mesh, { transfer: [mesh.positions.buffer, mesh.normals.buffer] });
    assert.equal(bodies[0].positions.length, 9);
  } finally { endCadMetrics(); }
});

test('supersession is checked between bodies before starting more geometry', async () => {
  const bodies = [0, 1, 2].map(index => ({ revision: 1, body: { id: String(index), name: String(index), kind: 'plate', thickness: 1 },
    regions: [{ outer: [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }, { x: 0, y: 10 }], holes: [], cavities: [], gaskets: [], mounts: [] }] }));
  let cancelled = false;
  const started = new Set();
  await assert.rejects(previewBodies({ revision: 1, bodies }, progress => {
    if (progress.body) started.add(progress.body);
    if (progress.completed === 1) cancelled = true;
  }, () => cancelled), /superseded/);
  assert.deepEqual([...started], ['0']);
  await assert.rejects(previewBodies({ revision: 1, bodies }, () => assert.fail('already cancelled'), () => true), /superseded/);
});

test('frequently reused body previews survive cache pressure', async () => {
  const ir = { revision: 1, bodies: [{ revision: 1,
    body: { id: 'hot', name: 'Hot', kind: 'plate', thickness: 0.67, z: 1000 },
    regions: [{ outer: [{ x: 0, y: 0 }, { x: 2, y: 0 }, { x: 2, y: 2 }, { x: 0, y: 2 }], holes: [], cavities: [], gaskets: [], mounts: [] }],
  }] };
  for (let index = 0; index < 70; index++) {
    const next = structuredClone(ir); next.bodies[0].body.z += index;
    await previewBodies(next, () => {});
    if (index % 4 === 0) {
      const metrics = beginCadMetrics();
      try {
        await previewBodies(ir, () => {});
        assert.equal(metrics.counters.bodyCacheHits, 1, `hot body at edit ${index}`);
      } finally { endCadMetrics(); }
    }
  }
  const metrics = beginCadMetrics();
  try {
    await previewBodies(ir, () => {});
    assert.equal(metrics.counters.bodyCacheHits, 1);
    assert.equal(metrics.counters.bodyCacheMisses, undefined);
  } finally { endCadMetrics(); }
});
