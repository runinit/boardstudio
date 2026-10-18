// Run against a wasm-pack --target nodejs build: node this-file /absolute/path/to/package.js
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const cad = require(path.resolve(process.argv[2]));
const fixture = JSON.parse(fs.readFileSync(path.join(__dirname, '../../test/fixtures/holed-plate.json')));
const selected = fixture.bodies[0];
const body = {
  revision: selected.revision,
  body: selected.body,
  regions: [{
    outer: selected.contours.find(c => !c.hole).points,
    holes: selected.contours.filter(c => c.hole).map(c => c.points),
    cavities: [], gaskets: [], mounts: [],
  }],
};
const result = cad.analyze_generated_body(body, {});
const pcb = JSON.parse(fs.readFileSync(path.join(__dirname, '../../monstertruck/tests/fixtures/pcb-source/circle-tracer.json')));
const pcbDefault = cad.analyze_generated_body(pcb, {});
const pcbRefined = cad.analyze_generated_body(pcb, {targetDeflectionMm: 0.01});
assert.equal(pcbRefined.bodyId, pcb.body.id);
assert.equal(pcbRefined.analysisErrorBoundMm, null);
assert(pcbRefined.surfacePositionsMmF64.length > pcbDefault.surfacePositionsMmF64.length);
assert(pcbRefined.sourceFaces.some(face => face.supportKind === 'REVOLUTION_SURFACE'));
assert.equal(result.bodyId, body.body.id);
assert.equal(result.revision, body.revision);
assert.equal(result.vendorStep, undefined);
assert.equal(result.generatedAnalysis, undefined);
assert.equal(result.geometryIdentity, undefined);
assert(result.surfacePositionsMmF64 instanceof Float64Array);
assert(result.triangleFaceOccurrenceIndices instanceof Uint32Array);
assert.equal(result.surfacePositionsMmF64.length, result.triangleFaceOccurrenceIndices.length * 9);
assert.equal(result.analysisErrorBoundMm, null);
assert.equal(result.deflectionMm, 0.1);
assert.equal(cad.analyze_generated_body(body, {targetDeflectionMm: 0.01}).deflectionMm, 0.01);
for (const targetDeflectionMm of [0, -1, NaN, Infinity]) {
  assert.throws(() => cad.analyze_generated_body(body, {targetDeflectionMm}));
}
for (const tag of result.triangleFaceOccurrenceIndices) {
  assert(tag < result.faceOccurrences.length);
  assert(result.faceOccurrences[tag].sourceFaceIndex < result.sourceFaces.length);
}
const identity = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
const moved = [...identity];
moved[12] = 200;
const operand = transform => ({modelId: 'generated', generatedGeometryIdentity: 'snapshot-1', rowTransformF64: transform});
const query = {left: operand(identity), right: operand(moved), clearanceThresholdMm: 0.5, requestedAccuracyMm: 0.01};
const observe = () => cad.query_collision_mesh_pair(result.surfacePositionsMmF64, result.triangleFaceOccurrenceIndices,
  result.surfacePositionsMmF64, result.triangleFaceOccurrenceIndices, query);
assert.equal(observe().kind, 'surface-separated');
assert.equal(observe().surfaceDistanceMm, 80); // Authored 120 mm plate, translated once by 200 mm.
query.left.sourceSha256 = 'a'.repeat(64);
assert.throws(observe);
delete query.left.sourceSha256;
query.left.importOptions = {targetDeflectionMm: 0.01};
assert.throws(observe);

// Independently authored axis-aligned cubes qualify surface queries, not solid overlap.
function cube(min, max) {
  const [x, y, z] = min, [X, Y, Z] = max;
  const v = [[x,y,z], [X,y,z], [X,Y,z], [x,Y,z], [x,y,Z], [X,y,Z], [X,Y,Z], [x,Y,Z]];
  const triangles = [[0,2,1],[0,3,2],[4,5,6],[4,6,7],[0,1,5],[0,5,4],[2,3,7],[2,7,6],[1,2,6],[1,6,5],[3,0,4],[3,4,7]];
  return {positions: new Float64Array(triangles.flatMap(t => t.flatMap(i => v[i]))), tags: new Uint32Array(triangles.map((_, i) => i))};
}
const unit = cube([0,0,0], [2,2,2]);
function pair(left, right) {
  return cad.query_collision_mesh_pair(left.positions, left.tags, right.positions, right.tags,
    {left: operand(identity), right: operand(identity), clearanceThresholdMm: 0.5});
}
const separated = pair(unit, cube([3,0,0], [5,2,2]));
assert.equal(separated.kind, 'surface-separated');
assert.equal(separated.surfaceDistanceMm, 1);
assert.equal(separated.closestPoints.point1Mm[0], 2);
assert.equal(separated.closestPoints.point2Mm[0], 3);
for (const x of [2, 1.25]) {
  const touchingOrCrossing = pair(unit, cube([x,0,0], [x+2,2,2]));
  assert.equal(touchingOrCrossing.kind, 'surface-intersecting');
  assert(touchingOrCrossing.intersectionTriangles.left.triangleIndex < 12);
  assert(touchingOrCrossing.contactWitness);
}
const nested = pair(cube([0,0,0], [10,10,10]), cube([3,3,3], [7,7,7]));
assert.equal(nested.kind, 'surface-separated');
assert.equal(nested.surfaceDistanceMm, 3);
for (const positions of [new Float64Array(9), new Float64Array([0,0,0,1,1,1,2,2,2])]) {
  assert.throws(() => pair({positions, tags: new Uint32Array([0])}, unit), /degenerate triangle/);
}
const open = {positions: unit.positions.slice(18), tags: unit.tags.slice(2)};
assert.equal(pair(open, cube([3,0,0], [5,2,2])).surfaceDistanceMm, 1);

const tetra = fs.readFileSync(path.join(__dirname, '../../monstertruck/tests/fixtures/tetra-mm.step'));
const imported = cad.read_step_model(new Uint8Array(tetra), {sourceSha256: 'a'.repeat(64), previewRepairBudgetMm: 0.03, targetDeflectionMm: 0.01});
assert.equal(imported.vendorStep.accuracy.selectedRepairBudgetMm, 0.03);
assert.equal(imported.vendorStep.accuracy.maxRepairDisplacementMm, null);
assert.equal(imported.vendorStep.accuracy.analysisErrorBoundMm, null);
assert.equal(imported.vendorStep.importOptions.targetDeflectionMm, 0.01);
assert(imported.vendorStep.faceOccurrences.every(face => face.repairDisplacementMm === null));
console.log('WASM generated analysis, vendor options, typed buffers, provenance and identity validation passed');
