import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { readStepModel } from '../src/index.ts';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const ledger = JSON.parse(readFileSync(resolve(root, 'catalogue/modules/asset-ledger.json'), 'utf8'));
const tightMeasurements = JSON.parse(readFileSync(resolve(root, 'cad/test/fixtures/vik-model-tight-bounds.json'), 'utf8')).models;
const sourceBounds = asset => {
  const b = asset.nativeBoundsMm;
  return { min: [b.XMin, b.YMin, b.ZMin], max: [b.XMax, b.YMax, b.ZMax] };
};
const meshBounds = positions => {
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (let index = 0; index < positions.length; index++) {
    const axis = index % 3;
    const value = positions[index];
    assert.ok(Number.isFinite(value), `mesh coordinate ${index} is finite`);
    min[axis] = Math.min(min[axis], value);
    max[axis] = Math.max(max[axis], value);
  }
  return { min, max };
};
const contains = (outer, inner, label) => {
  for (let axis = 0; axis < 3; axis++) {
    assert.ok(outer.min[axis] <= inner.min[axis] + 0.03, `${label} ledger min axis ${axis}: ${outer.min[axis]} <= ${inner.min[axis]}`);
    assert.ok(outer.max[axis] >= inner.max[axis] - 0.03, `${label} ledger max axis ${axis}: ${outer.max[axis]} >= ${inner.max[axis]}`);
  }
};

for (const asset of ledger.models.filter(model => /\.(?:step|stp)$/iu.test(model.path))) {
  test(`pinned VIK STEP ${asset.path} stays inside independently recorded source bounds`, async () => {
    const bytes = await readFile(resolve(root, asset.bundledFile));
    const imported = await readStepModel(new Uint8Array(bytes));
    assert.ok(imported.mesh.positions.length >= 9, 'STEP produced a non-empty triangle mesh');
    assert.equal(imported.mesh.positions.length, imported.mesh.normals.length);
    assert.ok([...imported.mesh.normals].every(Number.isFinite), 'STEP normals are finite');
    const actual = meshBounds(imported.mesh.positions);
    contains(imported.bounds, actual, `${asset.path} CAD-kernel bounds`);
    contains(sourceBounds(asset), actual, asset.path);
    const tight = tightMeasurements.find(measured => measured.assetId === asset.assetId);
    assert.ok(tight, 'Independent tight-bound measurement exists for this source');
    assert.equal(createHash('sha256').update(bytes).digest('hex'), tight.sha256);
    // Imported preview meshes use 0.1 mm linear deflection. Tight source
    // bounds still detect missing geometry; they are not mesh-derived fit data.
    for (let axis = 0; axis < 3; axis++) {
      assert.ok(Math.abs(actual.min[axis] - tight.min[axis]) <= 0.10001,
        `${asset.path} tight minimum axis ${axis}: ${actual.min[axis]} versus ${tight.min[axis]}`);
      assert.ok(Math.abs(actual.max[axis] - tight.max[axis]) <= 0.10001,
        `${asset.path} tight maximum axis ${axis}: ${actual.max[axis]} versus ${tight.max[axis]}`);
    }
  }, { timeout: 60_000 });
}

for (const asset of ledger.models.filter(model => /\.stl$/iu.test(model.path))) {
  test(`pinned VIK STL ${asset.path} stays inside independently recorded source bounds`, async () => {
    const bytes = await readFile(resolve(root, asset.bundledFile));
    assert.ok(bytes.length >= 84, 'binary STL header and triangle count exist');
    const triangles = bytes.readUInt32LE(80);
    assert.ok(triangles > 0, 'STL has triangles');
    assert.equal(bytes.length, 84 + triangles * 50, 'binary STL is complete');
    const min = [Infinity, Infinity, Infinity];
    const max = [-Infinity, -Infinity, -Infinity];
    for (let face = 0; face < triangles; face++) {
      const start = 84 + face * 50 + 12;
      for (let vertex = 0; vertex < 3; vertex++) {
        for (let axis = 0; axis < 3; axis++) {
          const value = bytes.readFloatLE(start + vertex * 12 + axis * 4);
          assert.ok(Number.isFinite(value), `STL coordinate for face ${face} is finite`);
          min[axis] = Math.min(min[axis], value);
          max[axis] = Math.max(max[axis], value);
        }
      }
    }
    contains(sourceBounds(asset), { min, max }, asset.path);
  }, { timeout: 60_000 });
}
