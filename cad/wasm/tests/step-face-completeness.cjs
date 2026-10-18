// Run against a wasm-pack --target nodejs build: node this-file /absolute/path/to/package.js
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const cad = require(path.resolve(process.argv[2]));
const repoRoot = path.resolve(__dirname, '../../..');
const fixturePath = path.join(repoRoot, 'cad/monstertruck/tests/fixtures/circular-segment-sliver-prism.step');

const bytes = fs.readFileSync(fixturePath);
const digest = crypto.createHash('sha256').update(bytes).digest('hex');

const result = cad.read_step_model(new Uint8Array(bytes), { sourceSha256: digest });

// Display mesh must be the f32 image of the retained f64 analysis positions.
assert(result.mesh.positions instanceof Float32Array, 'mesh.positions must be Float32Array');
assert(result.mesh.normals instanceof Float32Array, 'mesh.normals must be Float32Array');
assert(result.mesh.positions.length > 0, 'display mesh must be nonempty');
assert.equal(result.mesh.positions.length, result.mesh.normals.length, 'positions/normals align');

const vs = result.vendorStep;
assert(vs.surfacePositionsMmF64 instanceof Float64Array, 'surfacePositionsMmF64 must be Float64Array');
assert(vs.triangleFaceOccurrenceIndices instanceof Uint32Array, 'triangleFaceOccurrenceIndices must be Uint32Array');
assert.equal(vs.surfacePositionsMmF64.length, vs.triangleFaceOccurrenceIndices.length * 9, 'tag count matches triangles');
assert.equal(vs.surfacePositionsMmF64.length, result.mesh.positions.length, 'f64 analysis retains every display triangle');

// Exact f64 -> f32 cast correspondence (no precision laundering).
for (let i = 0; i < vs.surfacePositionsMmF64.length; i++) {
  assert.equal(result.mesh.positions[i], Math.fround(vs.surfacePositionsMmF64[i]),
    `display mesh must be f32 cast at index ${i}`);
  assert(Number.isFinite(vs.surfacePositionsMmF64[i]), 'f64 positions finite');
}

// Source face / occurrence provenance.
assert(Array.isArray(vs.sourceFaces), 'sourceFaces must be array');
assert(Array.isArray(vs.faceOccurrences), 'faceOccurrences must be array');
assert.equal(vs.sourceFaces.length, 4, 'sliver fixture has 4 source faces');
assert.equal(vs.faceOccurrences.length, 4, 'sliver fixture has 4 face occurrences');

const referencedOccurrences = new Set(Array.from(vs.triangleFaceOccurrenceIndices));
const referencedFaces = new Set();
for (const occ of vs.faceOccurrences) {
  assert(Number.isInteger(occ.sourceFaceIndex) && occ.sourceFaceIndex >= 0 && occ.sourceFaceIndex < vs.sourceFaces.length,
    'face occurrence sourceFaceIndex must be valid');
  assert(Number.isInteger(occ.solidOccurrenceIndex) && occ.solidOccurrenceIndex >= 0);
  referencedFaces.add(occ.sourceFaceIndex);
}
// Every retained face occurrence has triangles.
for (let i = 0; i < vs.faceOccurrences.length; i++) {
  assert(referencedOccurrences.has(i), `face occurrence ${i} must retain triangles`);
}
assert.equal(referencedFaces.size, vs.sourceFaces.length, 'every source face keeps analysis coverage');

// Every triangle tag references a valid face occurrence.
for (const tag of vs.triangleFaceOccurrenceIndices) {
  assert(Number.isInteger(tag) && tag >= 0 && tag < vs.faceOccurrences.length, 'triangle tag must be valid occurrence index');
}

// Accuracy: fallback meshed faces retained (2 for sliver fixture).
assert.equal(vs.accuracy.fallbackMeshedFaces, 2, 'fallback meshed faces count');

// Finite display output.
for (let i = 0; i < result.mesh.normals.length; i++) {
  assert(Number.isFinite(result.mesh.normals[i]), 'display normals finite');
}

// Bounds finite and ordered.
assert(Array.isArray(result.bounds.min) && Array.isArray(result.bounds.max));
for (let a = 0; a < 3; a++) {
  assert(Number.isFinite(result.bounds.min[a]));
  assert(Number.isFinite(result.bounds.max[a]));
  assert(result.bounds.min[a] <= result.bounds.max[a]);
}

console.log('PASS step-face-completeness: sliver fixture retains', vs.surfacePositionsMmF64.length / 9, 'f64 triangles across', vs.sourceFaces.length, 'source faces');
