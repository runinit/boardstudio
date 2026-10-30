// Actual packaged provider, exercised in an isolated browser page.
import { assert, readBytes, readText, runCases } from './provider-support.mjs';

export async function run(cad) {
return runCases('collision-analysis', [['main', async () => {
const contract = JSON.parse(await readText('cad/wasm/tests/fixtures/generated-analysis-contract.json'));
const fieldSet = value => Object.keys(value).sort();
const isU32 = value => Number.isInteger(value) && value >= 0 && value <= 0xffffffff;
function checkContract(payload, deflectionMm) {
  const example = contract.directPayload;
  const qualified = payload.analysisHandle instanceof cad.AnalysisHandle;
  assert.deepEqual(fieldSet(payload), qualified ? [...contract.qualificationV2.activation.directGeneratedRecipe.expectedProviderKeys].sort() : fieldSet(example), 'backend owns only the direct payload fields');
  assert.equal(payload.bodyId, contract.preparedBody.body.id);
  assert.equal(payload.revision, contract.preparedBody.revision);
  assert(payload.surfacePositionsMmF64 instanceof Float64Array);
  assert(payload.triangleFaceOccurrenceIndices instanceof Uint32Array);
  assert(payload.surfacePositionsMmF64.length > 0);
  assert.equal(payload.surfacePositionsMmF64.length, payload.triangleFaceOccurrenceIndices.length * 9);
  assert.equal(payload.analysisErrorBoundMm, payload.sourceQualification?.state === 'bounded' ? payload.sourceQualification.totalBoundMm : null, 'only the sealed source proof supplies the bound');
  assert.equal(payload.deflectionMm, deflectionMm);
  assert.equal(typeof payload.meshClosedOriented, typeof example.meshClosedOriented);
  assert.equal(payload.meshClosedOriented, example.meshClosedOriented);
  assert(Array.isArray(payload.sourceFaces) && payload.sourceFaces.length > 0);
  assert(Array.isArray(payload.faceOccurrences) && payload.faceOccurrences.length > 0);
  const bounds = {min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity]};
  payload.surfacePositionsMmF64.forEach((value, index) => {
    assert(Number.isFinite(value));
    bounds.min[index % 3] = Math.min(bounds.min[index % 3], value === 0 ? 0 : value);
    bounds.max[index % 3] = Math.max(bounds.max[index % 3], value === 0 ? 0 : value);
  });
  assert.deepEqual(bounds, contract.expectedBoundsMm);
  for (const face of payload.sourceFaces) {
    assert.deepEqual(fieldSet(face), fieldSet(example.sourceFaces[0]));
    assert([face.solidIndex, face.shellIndex, face.faceIndex].every(isU32));
    assert.equal(typeof face.supportKind, 'string');
    assert(face.supportKind.trim().length > 0);
  }
  const referencedFaces = new Set();
  const referencedOccurrences = new Set(payload.triangleFaceOccurrenceIndices);
  payload.faceOccurrences.forEach((occurrence, index) => {
    assert.deepEqual(fieldSet(occurrence), fieldSet(example.faceOccurrences[0]));
    assert(isU32(occurrence.sourceFaceIndex) && occurrence.sourceFaceIndex < payload.sourceFaces.length);
    assert(isU32(occurrence.solidOccurrenceIndex));
    assert.equal(occurrence.solidOccurrenceIndex, payload.sourceFaces[occurrence.sourceFaceIndex].solidIndex);
    assert(referencedOccurrences.has(index), 'every retained face occurrence has triangles');
    referencedFaces.add(occurrence.sourceFaceIndex);
  });
  assert.equal(referencedFaces.size, payload.sourceFaces.length);
  assert([...referencedOccurrences].every(index => index < payload.faceOccurrences.length));
  if (qualified) payload.analysisHandle.free();
}
// The same JSON example is consumed by Host; arrays are transport syntax only.
checkContract({...contract.directPayload,
  surfacePositionsMmF64: new Float64Array(contract.directPayload.surfacePositionsMmF64),
  triangleFaceOccurrenceIndices: new Uint32Array(contract.directPayload.triangleFaceOccurrenceIndices),
}, contract.directPayload.deflectionMm);
checkContract(cad.analyze_generated_body(contract.preparedBody, contract.options), contract.directPayload.deflectionMm);
checkContract(cad.analyze_generated_body(contract.preparedBody, contract.refinementOptions), contract.refinementOptions.targetDeflectionMm);
checkContract(cad.analyze_generated_body(contract.preparedBody, {targetDeflectionMm: null}), contract.directPayload.deflectionMm);
for (const key of ['geometryIdentity', 'sourceSha256', 'generatedAnalysis', 'vendorStep']) {
  assert.throws(() => cad.analyze_generated_body(contract.preparedBody, {[key]: contract.workerGeometryIdentityPrefix}));
}
for (const invalid of [null, undefined, [], 0, 'options', true, () => {}]) {
  assert.throws(() => cad.analyze_generated_body(contract.preparedBody, invalid));
}
const fixture = JSON.parse(await readText('cad/test/fixtures/holed-plate.json'));
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
const pcb = JSON.parse(await readText('cad/monstertruck/tests/fixtures/pcb-source/circle-tracer.json'));
const pcbDefault = cad.analyze_generated_body(pcb, {});
const pcbRefined = cad.analyze_generated_body(pcb, {targetDeflectionMm: 0.01});
assert.equal(pcbRefined.bodyId, pcb.body.id);
assert.equal(pcbRefined.analysisErrorBoundMm, pcbRefined.sourceQualification.state === 'bounded' ? pcbRefined.sourceQualification.totalBoundMm : null);
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
assert.equal(result.analysisErrorBoundMm, result.sourceQualification.state === 'bounded' ? result.sourceQualification.totalBoundMm : null);
assert.equal(result.deflectionMm, 0.1);
const refined = cad.analyze_generated_body(body, {targetDeflectionMm: 0.01});
assert.equal(refined.deflectionMm, 0.01);
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

const tetra = await readBytes('cad/monstertruck/tests/fixtures/tetra-mm.step');
const imported = cad.read_step_model(new Uint8Array(tetra), {sourceSha256: 'a'.repeat(64), previewRepairBudgetMm: 0.03, targetDeflectionMm: 0.01});
assert.equal(imported.vendorStep.accuracy.selectedRepairBudgetMm, 0.03);
assert.equal(imported.vendorStep.accuracy.maxRepairDisplacementMm, null);
assert.equal(imported.vendorStep.accuracy.analysisErrorBoundMm, null);
assert.equal(imported.vendorStep.importOptions.targetDeflectionMm, 0.01);
assert(imported.vendorStep.faceOccurrences.every(face => face.repairDisplacementMm === null));
for (const asset of [result, refined, pcbDefault, pcbRefined, imported.vendorStep]) asset.analysisHandle.free();
console.log('WASM shared generated-analysis contract, vendor options, analytical collision fixtures and validation passed');

}]]);
}
