// Run against a release wasm-pack --target nodejs package; this does not expose the private ledger.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const cad = require(path.resolve(process.argv[2]));
const fixtures = path.resolve(__dirname, '../../monstertruck/tests/fixtures');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
function fixture(name) {
  const bytes = fs.readFileSync(path.join(fixtures, name));
  const text = bytes.toString('utf8');
  assert.deepEqual(Buffer.from(text), bytes, 'fixture decoding must retain the original bytes');
  return text;
}
function replaceOnce(text, before, after) {
  assert.equal(text.split(before).length - 1, 1, `unique original fixture entity: ${before}`);
  const changed = text.replace(before, after);
  assert.notEqual(sha(Buffer.from(changed)), sha(Buffer.from(text)), 'mutation must change source identity');
  assert.equal(changed.split(before).length - 1, after.split(before).length - 1,
    'only occurrences deliberately retained by the replacement may remain');
  assert(changed.includes(after));
  return changed;
}
function check(text, options, expectedBounds, occurrences, sourceBounds = true) {
  const bytes = Buffer.from(text), original = Buffer.from(bytes);
  const digest = sha(bytes);
  const input = new Uint8Array(bytes);
  const result = cad.read_step_model(input, {sourceSha256: digest, ...options});
  assert.deepEqual(Buffer.from(input), original);
  const vendor = result.vendorStep;
  assert.equal(vendor.sourceSha256, digest);
  assert.equal(vendor.accuracy.analysisErrorBoundMm, null);
  assert.equal(vendor.accuracy.maxRepairDisplacementMm, null);
  assert.equal(vendor.accuracy.boundsFromSourceGeometry, sourceBounds);
  assert.deepEqual(vendor.sourceFaces.map(face => face.faceId).sort(), ['59', '69', '79', '89']);
  assert(vendor.sourceFaces.every(face => face.solidId === '101'));
  assert.equal(vendor.faceOccurrences.length, 4 * occurrences);
  assert.equal(new Set(vendor.faceOccurrences.map(face => face.solidOccurrenceIndex)).size, occurrences);
  assert(vendor.surfacePositionsMmF64 instanceof Float64Array);
  assert(vendor.triangleFaceOccurrenceIndices instanceof Uint32Array);
  assert.equal(vendor.surfacePositionsMmF64.length, vendor.triangleFaceOccurrenceIndices.length * 9);
  assert(vendor.surfacePositionsMmF64.length > 0);
  assert(vendor.surfacePositionsMmF64.every(Number.isFinite));
  const tags = new Set(vendor.triangleFaceOccurrenceIndices);
  assert.equal(tags.size, vendor.faceOccurrences.length, 'all original face occurrences retain triangles');
  assert([...tags].every(tag => tag < vendor.faceOccurrences.length));
  for (const face of vendor.faceOccurrences) {
    assert.equal(face.repairDisplacementMm, null);
    assert(Number.isInteger(face.sourceFaceIndex) && face.sourceFaceIndex >= 0 && face.sourceFaceIndex < 4);
  }
  for (const [index, key] of ['min', 'max'].entries()) {
    result.bounds[key].forEach((value, axis) => {
      assert(Number.isFinite(value));
      assert(Math.abs(value - expectedBounds[index][axis]) < 1e-9, `${key}[${axis}]: ${value}`);
    });
  }
  return result;
}
const tetra = fixture('tetra-mm.step');
const tetraBounds = [[0, 0, 0], [2, 3, 4]];
check(tetra, {}, tetraBounds, 1);
const line = "#22=LINE('',#1,#21);";
const shiftedLine = "#310=CARTESIAN_POINT('',(0.,0.001953125,0.));#22=LINE('',#310,#21);";
check(replaceOnce(tetra, line, shiftedLine), {}, tetraBounds, 1);
const metreAssembly = replaceOnce(
  replaceOnce(fixture('two-occurrences-mm.step'), line, shiftedLine),
  'SI_UNIT(.MILLI.,.METRE.)', 'SI_UNIT($,.METRE.)');
// Independently authored placements: (10-y,20+x,30+z) and (x-5,y+7,z+11), in metres.
assert.throws(() => cad.read_step_model(new Uint8Array(Buffer.from(metreAssembly)),
  {sourceSha256: sha(Buffer.from(metreAssembly))}), error =>
    typeof error === 'string' && error.includes('finite LINE repair') && error.includes('repair budget 0.02'));
check(metreAssembly, {previewRepairBudgetMm: 2}, [[-5000, 7000, 11000], [10000, 22000, 34000]], 2);
const finite = fixture('repair-finite-domain.step');
const finiteBytes = new Uint8Array(Buffer.from(finite));
function refusesFinite(options) {
  assert.throws(() => cad.read_step_model(finiteBytes,
    {sourceSha256: sha(Buffer.from(finite)), ...options}), error =>
      typeof error === 'string' && error.includes('finite LINE repair') && error.includes('exceeds repair budget'));
}
refusesFinite({});
refusesFinite({previewRepairBudgetMm: 0.031249999999999997});
for (const allowance of [0.03125, 0.03125000000000001]) {
  const accepted = check(finite, {previewRepairBudgetMm: allowance}, tetraBounds, 1);
  assert.equal(accepted.vendorStep.accuracy.selectedRepairBudgetMm, allowance);
}
assert.deepEqual(Buffer.from(finiteBytes), Buffer.from(finite), 'refused/accepted calls retain immutable original bytes');
check(replaceOnce(finite, '(0.,0.03125,0.)', '(0.,0.015625,0.)'), {}, tetraBounds, 1);
let gap = replaceOnce(tetra,
  "#103=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));",
  "#103=(GEOMETRIC_REPRESENTATION_CONTEXT(3) REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300)));");
gap = replaceOnce(gap, 'ENDSEC;END-ISO-10303-21;',
  "#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#102,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;");
gap = replaceOnce(gap, "#57=AXIS2_PLACEMENT_3D('',#1,#55,#56);",
  "#310=CARTESIAN_POINT('',(0.,0.,0.025));#57=AXIS2_PLACEMENT_3D('',#310,#55,#56);");
for (const options of [{}, {previewRepairBudgetMm: 0.024}]) {
  assert.throws(() => cad.read_step_model(new Uint8Array(Buffer.from(gap)),
    {sourceSha256: sha(Buffer.from(gap)), ...options}), error => {
    assert.equal(typeof error, 'string');
    assert(error.includes('SourceConnectivityRefused'));
    const limit = options.previewRepairBudgetMm ?? 0.02;
    assert(error.includes(`#59: vertex #11: no direct finite face attachment witness within source bound ${limit}`));
    return true;
  });
}
// An admitted off-plane attachment still fails exact source-bound certification.
const admitted = check(gap, {previewRepairBudgetMm: 0.03}, tetraBounds, 1, false);
assert.equal(admitted.vendorStep.accuracy.selectedRepairBudgetMm, 0.03);
assert.equal(admitted.vendorStep.importOptions.previewRepairBudgetMm, 0.03);
assert(admitted.vendorStep.faceOccurrences.every(face => face.sourceUncertaintyMm === 0.001));
const curved = check(fixture('unsupported-non-ruled-sphere.step'), {}, [[0, 0, 0], [1, 1, 1]], 1, false);
assert(curved.vendorStep.sourceFaces.some(face => face.supportKind !== 'PLANE'));
console.log('PASS complete finite LINE operation default/equality/nextdown budget enforcement; incomplete asset remains Unknown; repair accounting provider: original/shifted LINE, metre occurrences, default/.024 refusal, .03 admission, curved Unknown, computed SHA/provenance/bounds and null movement/accuracy');
