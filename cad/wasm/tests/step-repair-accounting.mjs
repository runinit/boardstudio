// Actual packaged provider, exercised in an isolated browser page.
import { assert, readBytes, decodeText, encodeText, sha256Hex, runCases } from './provider-support.mjs';

export async function run(cad, memory) {
return await runCases('step-repair-accounting', [['main', async () => {
const sha = bytes => sha256Hex(bytes);

async function fixture(path) {
  const bytes = await readBytes(path);
  const text = decodeText(bytes);
  assert.deepEqual(encodeText(text), bytes, 'fixture decoding must retain the original bytes');
  return text;
}

function replaceOnce(text, before, after) {
  assert.equal(text.split(before).length - 1, 1, `unique original fixture entity: ${before}`);
  const changed = text.replace(before, after);
  assert.notEqual(changed, text, 'mutation must change source identity');
  assert.equal(changed.split(before).length - 1, after.split(before).length - 1,
    'only occurrences deliberately retained by the replacement may remain');
  assert(changed.includes(after));
  return changed;
}

async function check(text, options, expectedBounds, occurrences, sourceBounds = true) {
  const bytes = encodeText(text), original = new Uint8Array(bytes);
  const digest = await sha(bytes);
  const input = new Uint8Array(bytes);
  const result = cad.read_step_model(input, {sourceSha256: digest, ...options});
  assert.deepEqual(new Uint8Array(input), original);
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
  result.vendorStep.analysisHandle.free();
  return result;
}

const tetra = await fixture('cad/monstertruck/tests/fixtures/tetra-mm.step');
const tetraBounds = [[0, 0, 0], [2, 3, 4]];
await check(tetra, {}, tetraBounds, 1);
const line = "#22=LINE('',#1,#21);";
const shiftedLine = "#310=CARTESIAN_POINT('',(0.,0.001953125,0.));#22=LINE('',#310,#21);";
await check(replaceOnce(tetra, line, shiftedLine), {}, tetraBounds, 1);
const metreAssembly = replaceOnce(
  replaceOnce(await fixture('cad/monstertruck/tests/fixtures/two-occurrences-mm.step'), line, shiftedLine),
  'SI_UNIT(.MILLI.,.METRE.)', 'SI_UNIT($,.METRE.)');
// Independently authored placements: (10-y,20+x,30+z) and (x-5,y+7,z+11), in metres.
const metreBytes = encodeText(metreAssembly);
const metreDigest = await sha(metreBytes);
assert.throws(() => cad.read_step_model(new Uint8Array(metreBytes),
  {sourceSha256: metreDigest}), error =>
    typeof error === 'string' && error.includes('finite LINE repair') && error.includes('repair budget 0.02'));
await check(metreAssembly, {previewRepairBudgetMm: 2}, [[-5000, 7000, 11000], [10000, 22000, 34000]], 2);
const finite = await fixture('cad/monstertruck/tests/fixtures/repair-finite-domain.step');
const finiteBytes = new Uint8Array(encodeText(finite));
const finiteDigest = await sha(encodeText(finite));
function refusesFinite(options) {
  assert.throws(() => cad.read_step_model(finiteBytes,
    {sourceSha256: finiteDigest, ...options}), error =>
      typeof error === 'string' && error.includes('finite LINE repair') && error.includes('exceeds repair budget'));
}
refusesFinite({});
refusesFinite({previewRepairBudgetMm: 0.031249999999999997});
for (const allowance of [0.03125, 0.03125000000000001]) {
  const accepted = await check(finite, {previewRepairBudgetMm: allowance}, tetraBounds, 1);
  assert.equal(accepted.vendorStep.accuracy.selectedRepairBudgetMm, allowance);
}
assert.deepEqual(new Uint8Array(finiteBytes), new Uint8Array(encodeText(finite)),
  'refused/accepted calls retain immutable original bytes');
await check(replaceOnce(finite, '(0.,0.03125,0.)', '(0.,0.015625,0.)'), {}, tetraBounds, 1);
const unrelatedNumeric = replaceOnce(finite, 'ENDSEC;END-ISO-10303-21;',
  "#4000=DIRECTION('',(1.E-100,1.,0.));#4001=VECTOR('',#4000,1.);#4002=LINE('',#1,#4001);#4003=EDGE_CURVE('',#11,#12,#4002,.T.);ENDSEC;END-ISO-10303-21;");
const unrelatedBytes = encodeText(unrelatedNumeric);
const unrelatedDigest = await sha(unrelatedBytes);
assert.throws(() => cad.read_step_model(new Uint8Array(unrelatedBytes),
  {sourceSha256: unrelatedDigest}), error =>
    typeof error === 'string' && error.includes('finite LINE repair edge #23'));
await check(unrelatedNumeric, {previewRepairBudgetMm: 0.03125}, tetraBounds, 1);

let gap = replaceOnce(tetra,
  "#103=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));",
  "#103=(GEOMETRIC_REPRESENTATION_CONTEXT(3) REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300)));");
gap = replaceOnce(gap, 'ENDSEC;END-ISO-10303-21;',
  "#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#102,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;");
gap = replaceOnce(gap, "#57=AXIS2_PLACEMENT_3D('',#1,#55,#56);",
  "#310=CARTESIAN_POINT('',(0.,0.,0.025));#57=AXIS2_PLACEMENT_3D('',#310,#55,#56);");
const gapBytes = encodeText(gap);
const gapDigest = await sha(gapBytes);
for (const options of [{}, {previewRepairBudgetMm: 0.024}]) {
  assert.throws(() => cad.read_step_model(new Uint8Array(gapBytes),
    {sourceSha256: gapDigest, ...options}), error => {
    assert.equal(typeof error, 'string');
    assert(error.includes('SourceConnectivityRefused'));
    const limit = options.previewRepairBudgetMm ?? 0.02;
    assert(error.includes(`#59: vertex #11: no direct finite face attachment witness within source bound ${limit}`));
    return true;
  });
}
// An admitted off-plane attachment still fails exact source-bound certification.
const admitted = await check(gap, {previewRepairBudgetMm: 0.03}, tetraBounds, 1, false);
assert.equal(admitted.vendorStep.accuracy.selectedRepairBudgetMm, 0.03);
assert.equal(admitted.vendorStep.importOptions.previewRepairBudgetMm, 0.03);
assert(admitted.vendorStep.faceOccurrences.every(face => face.sourceUncertaintyMm === 0.001));
const curved = await check(await fixture('cad/monstertruck/tests/fixtures/unsupported-non-ruled-sphere.step'), {}, [[0, 0, 0], [1, 1, 1]], 1, false);
assert(curved.vendorStep.sourceFaces.some(face => face.supportKind !== 'PLANE'));
// The same literal producer fixture exercises actual native spline restriction
// and the concrete whole-domain certifier inside the rebuilt WASM importer.
const partialBytes = await readBytes('cad/monstertruck-qualification/vendor/monstertruck-io/tests/input/partial_closed_spline/solid32377-source.step');
const partialDigest = await sha(partialBytes);
const partialInput = new Uint8Array(partialBytes);
const partial = cad.read_step_model(partialInput, {sourceSha256: partialDigest});
assert.deepEqual(new Uint8Array(partialInput), partialBytes);
assert.equal(partial.vendorStep.sourceFaces.length, 12);
assert.equal(partial.vendorStep.faceOccurrences.length, 12);
assert.equal(new Set(partial.vendorStep.triangleFaceOccurrenceIndices).size, 12);
assert.equal(partial.vendorStep.accuracy.maxRepairDisplacementMm, null);
assert.equal(partial.vendorStep.accuracy.analysisErrorBoundMm, null);
assert.equal(partial.vendorStep.sourceQualification.state, 'unknown');
partial.vendorStep.analysisHandle.free();
console.log('PASS actual native spline 12-face path and complete finite LINE operation default/equality/nextdown budget enforcement; incomplete asset remains Unknown; repair accounting provider: original/shifted LINE, metre occurrences, default/.024 refusal, .03 admission, curved Unknown, computed SHA/provenance/bounds and null movement/accuracy');

}]]);
}
