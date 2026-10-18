// Actual rebuilt Node producer; no diagnostic/certificate constructor is exposed.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const cad = require(path.resolve(process.argv[2]));
const base = fs.readFileSync(path.resolve(__dirname,
  '../../monstertruck/tests/fixtures/repair-projected-line.step'), 'utf8');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
function replaceOnce(text, before, after) {
  assert.equal(text.split(before).length - 1, 1);
  return text.replace(before, after);
}
function projected(offset) {
  return replaceOnce(base, '0.015625', offset);
}
function read(source, options = {}, occurrences = 1,
  expectedBounds = [[0, 0, 0], [2, 3, 4]]) {
  const original = Buffer.from(source), input = new Uint8Array(original);
  let result;
  try {
    result = cad.read_step_model(input, {sourceSha256: sha(original), ...options});
  } finally {
    assert.deepEqual(Buffer.from(input), original, 'success and refusal preserve original input');
  }
  const vendor = result.vendorStep;
  assert.equal(vendor.sourceSha256, sha(original));
  assert.equal(vendor.sourceFaces.length, 4);
  assert.equal(vendor.faceOccurrences.length, 4 * occurrences);
  assert.equal(new Set(vendor.triangleFaceOccurrenceIndices).size, 4 * occurrences);
  assert(vendor.surfacePositionsMmF64.every(Number.isFinite));
  assert.deepEqual([result.bounds.min, result.bounds.max], expectedBounds);
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
read(boundary, {previewRepairBudgetMm: 0.020000000000000004});
const exact = projected('0.03125');
for (const previewRepairBudgetMm of [0.03125, 0.03125000000000001]) {
  read(exact, {previewRepairBudgetMm});
}
assert.throws(() => read(exact, {previewRepairBudgetMm: 0.031249999999999997}),
  error => typeof error === 'string' && error.includes('EDGE_CURVE attachment exceeds its source connectivity bound'));
let reversed = projected('0.015625');
for (const [a, b] of [
  ["#23=EDGE_CURVE('',#11,#12,#22,.T.);", "#23=EDGE_CURVE('',#12,#11,#22,.F.);"],
  ["#52=ORIENTED_EDGE('',*,*,#23,.F.);", "#52=ORIENTED_EDGE('',*,*,#23,.T.);"],
  ["#60=ORIENTED_EDGE('',*,*,#23,.T.);", "#60=ORIENTED_EDGE('',*,*,#23,.F.);"]
]) reversed = replaceOnce(reversed, a, b);
read(reversed);
// The source's native interval sense remains unproved; it must not erase
// the independently captured finite attachment operation.
const unresolvedSense = text => replaceOnce(text,
  "#23=EDGE_CURVE('',#11,#12,#22,.T.);", "#23=EDGE_CURVE('',#11,#12,#22,.F.);");
read(unresolvedSense(projected('0.015625')));
assert.throws(() => read(unresolvedSense(boundary)), error => typeof error === 'string'
  && error.includes('finite LINE repair edge #23'));
const unused = "#4000=DIRECTION('',(1.E-100,1.,0.));#4001=VECTOR('',#4000,1.);#4002=LINE('',#1,#4001);#4003=EDGE_CURVE('',#11,#12,#4002,.T.);";
for (const padded of [false, true]) for (const before of [false, true]) {
  const mutate = source => {
    const text = before ? replaceOnce(source, 'DATA;', `DATA;${unused}`)
      : replaceOnce(source, 'ENDSEC;END-ISO-10303-21;', `${unused}ENDSEC;END-ISO-10303-21;`);
    return padded ? text + ' '.repeat(300000) : text;
  };
  for (const unprovedNativeSense of [false, true]) {
    const sense = source => unprovedNativeSense ? unresolvedSense(source) : source;
    assert.throws(() => read(mutate(sense(boundary))), error => typeof error === 'string'
      && error.includes('finite LINE repair edge #23'));
    read(mutate(sense(projected('0.015625'))));
  }
}
const assembly = fs.readFileSync(path.resolve(__dirname,
  '../../monstertruck/tests/fixtures/two-occurrences-mm.step'), 'utf8');
const tail = assembly.slice(assembly.indexOf('#200='), assembly.indexOf('ENDSEC;END-ISO-10303-21;'));
for (const metre of [false, true]) {
  let text = replaceOnce(projected(metre ? '0.000015625' : '0.015625'),
    'ENDSEC;END-ISO-10303-21;', `${tail}ENDSEC;END-ISO-10303-21;`);
  if (metre) text = replaceOnce(text, 'SI_UNIT(.MILLI.,.METRE.)', 'SI_UNIT($,.METRE.)');
  const expected = metre ? [[-5000, 7000, 11000], [10000, 22000, 34000]]
    : [[-5, 7, 11], [10, 22, 34]];
  read(text, {}, 2, expected);
  read(text, {previewRepairBudgetMm: 0.015625}, 2, expected);
}
const scaled = replaceOnce(projected('0.015625'), 'ENDSEC;END-ISO-10303-21;',
  "#500=SHAPE_REPRESENTATION('',(#505),#103);#501=AXIS2_PLACEMENT_3D('',#1,$,$);#502=REPRESENTATION_MAP(#501,#104);#503=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#1,2.,$);#505=MAPPED_ITEM('',#502,#503);ENDSEC;END-ISO-10303-21;");
for (const padded of [false, true]) {
  const text = padded ? scaled + ' '.repeat(300000) : scaled;
  assert.throws(() => read(text), error => typeof error === 'string'
    && error.includes('finite LINE repair edge #23'), 'world .03125 exceeds default .02 after real scale2');
  read(text, {previewRepairBudgetMm: 0.03125}, 1, [[0, 0, 0], [4, 6, 8]]);
}
console.log('PASS actual projected finite LINE attachment: exact default/explicit bands, sense, metre/rotated occurrences, capped/unrelated entity monotonic refusal, immutable bytes and complete face inventory; asset remains Unknown');
