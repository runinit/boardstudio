import assert from 'node:assert/strict';
import test from 'node:test';
import { buildKeycaps, readStepModel } from '../src/index.ts';
const spec = { id: 'key-a', reference: 'SW1', profile: 'dsa', mount: 'mx', row: 3, size: { x: 18.2, y: 18.2 }, topSize: { x: 13.2, y: 13.2 }, height: 7.5, tilt: 0, dishDepth: .65, spherical: true, wallThickness: 1.2, pose: { at: { x: 30, y: 20 }, rotation: 15 }, side: 'front', z: 10, travel: 4, legend: 'O', color: '#eeeeee', legendColor: '#222222' };
test('WASM keycaps preserve independent meshes, STEP, legends and cache ownership', async () => {
  const preview = await buildKeycaps(5, [spec]);
  assert.equal(preview.revision, 5);
  assert.equal(preview.step.length, 0);
  assert.equal(preview.bodies.length, 2);
  assert.equal(preview.bodies[1].id, 'keycap-legend:key-a');
  const original = preview.bodies[0].positions[0];
  structuredClone(preview.bodies[0].positions, { transfer: [preview.bodies[0].positions.buffer] });
  const next = await buildKeycaps(6, [spec]);
  assert.equal(next.bodies[0].positions[0], original);
  const exported = await buildKeycaps(6, [spec], true);
  assert.ok(exported.step.length > 1000);
  const model = await readStepModel(exported.step);
  assert.ok(model.bounds.min[0] > 15 && model.bounds.max[0] < 45);
  assert.ok(model.mesh.positions.length > 100);
  await assert.rejects(buildKeycaps(7, [spec], false, () => true), /superseded/);
});
test('keycap placement and row geometry participate in cache dependency keys', async () => {
  const a = await buildKeycaps(1, [spec]);
  const b = await buildKeycaps(2, [{ ...spec, pose: { ...spec.pose, at: { x: 40, y: 20 } } }]);
  assert.ok(Math.abs(b.bodies[0].positions[0] - a.bodies[0].positions[0] - 10) < 1e-4);
  const c = await buildKeycaps(3, [{ ...spec, height: 9.5, tilt: 6 }]);
  assert.notDeepEqual(c.bodies[0].positions, a.bodies[0].positions);
});
