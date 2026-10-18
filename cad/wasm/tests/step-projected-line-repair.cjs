// Actual rebuilt Node producer; no diagnostic/certificate constructor is exposed.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const cad = require(path.resolve(process.argv[2]));
const base = fs.readFileSync(path.resolve(__dirname,
  '../../monstertruck/tests/fixtures/repair-finite-domain.step'), 'utf8');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
function replaceOnce(text, before, after) {
  assert.equal(text.split(before).length - 1, 1);
  return text.replace(before, after);
}
function projected(offset) {
  return replaceOnce(replaceOnce(base,
    "#103=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));",
    "#105=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.000001),#102,'distance_accuracy_value','');#103=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#105))REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));"),
  '0.03125', offset);
}
function read(source, options = {}) {
  const original = Buffer.from(source), input = new Uint8Array(original);
  const result = cad.read_step_model(input, {sourceSha256: sha(original), ...options});
  assert.deepEqual(Buffer.from(input), original);
  const vendor = result.vendorStep;
  assert.equal(vendor.sourceSha256, sha(original));
  assert.equal(vendor.sourceFaces.length, 4);
  assert.equal(vendor.faceOccurrences.length, 4);
  assert.equal(new Set(vendor.triangleFaceOccurrenceIndices).size, 4);
  assert.equal(vendor.accuracy.maxRepairDisplacementMm, null);
  assert.equal(vendor.accuracy.analysisErrorBoundMm, null);
  assert.equal(vendor.sourceQualification.state, 'unknown');
  return result;
}
read(projected('0.015625'));
const boundary = projected('0.020000000000000001');
assert.throws(() => read(boundary), error =>
  typeof error === 'string' && error.includes('finite LINE repair edge #23'),
  'exact ideal attachment projection above default .02 must refuse');
console.log('PASS actual projected finite LINE attachment: immutable source, positive4face preview and exact default budget refusal; complete asset remains Unknown');
