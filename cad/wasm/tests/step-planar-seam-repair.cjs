// Actual provider compatibility; the private operation proof is tested natively.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const cad = require(path.resolve(process.argv[2]));
const legacy = process.argv[3] && require(path.resolve(process.argv[3]));
const source = fs.readFileSync(path.resolve(__dirname, '../../monstertruck/tests/fixtures/repair-planar-seam.step'), 'utf8');
function replace(text, before, after) {
  assert.equal(text.split(before).length - 1, 1);
  return text.replace(before, after);
}
const closed = replace(replace(replace(source,
  "#10=EDGE_LOOP('',(#9));", "#10=EDGE_LOOP('',(#9,#74));"),
  "#13=CLOSED_SHELL('',(#12));", "#13=CLOSED_SHELL('',(#12,#75));"),
  'ENDSEC;END-ISO-10303-21;',
  "#70=DIRECTION('',(0.,-1.,0.));#71=VECTOR('',#70,1.);#72=LINE('',#34,#71);#73=EDGE_CURVE('',#32,#2,#72,.T.);#74=ORIENTED_EDGE('',*,*,#73,.T.);#75=ADVANCED_FACE('',(#11),#6,.F.);ENDSEC;END-ISO-10303-21;");
function options(bytes, selected) {
  return {sourceSha256: crypto.createHash('sha256').update(bytes).digest('hex'), ...selected};
}
const openBytes = Buffer.from(source);
assert.throws(() => cad.read_step_model(new Uint8Array(openBytes), options(openBytes, {})), /open boundary/);
function check(text, selected = {}, scale = 1) {
  const bytes = Buffer.from(text), input = new Uint8Array(bytes), settings = options(bytes, selected);
  const result = cad.read_step_model(input, settings);
  assert.deepEqual(Buffer.from(input), bytes);
  const vendor = result.vendorStep;
  assert.equal(vendor.sourceSha256, settings.sourceSha256);
  assert.equal(vendor.accuracy.maxRepairDisplacementMm, null);
  assert.equal(vendor.accuracy.analysisErrorBoundMm, null);
  assert.deepEqual(vendor.sourceFaces.map(face => face.faceId).sort(), ['12', '75']);
  assert.equal(vendor.faceOccurrences.length, 2);
  assert.equal(new Set(vendor.triangleFaceOccurrenceIndices).size, 2);
  assert(vendor.surfacePositionsMmF64.length > 0);
  assert.equal(vendor.surfacePositionsMmF64.length, vendor.triangleFaceOccurrenceIndices.length * 9);
  assert(vendor.surfacePositionsMmF64.every(Number.isFinite));
  assert(vendor.faceOccurrences.every(face => face.repairDisplacementMm === null));
  for (const axis of [0, 1, 2]) {
    const expected = [[0, -scale, 0], [scale, scale, 0]];
    for (const [i, key] of ['min', 'max'].entries()) {
      assert(Math.abs(result.bounds[key][axis] - expected[i][axis]) < scale * 1e-6);
    }
  }
  if (legacy) assert.deepEqual(result, legacy.read_step_model(new Uint8Array(bytes), settings),
    'same bytes/settings keep the actual legacy provider geometry and metadata');
}
check(closed);
check(closed, {previewRepairBudgetMm: 0.02});
check(closed, {previewRepairBudgetMm: 0.001});
check(replace(closed, 'SI_UNIT(.MILLI.,.METRE.)', 'SI_UNIT($,.METRE.)'), {}, 1000);
check(replace(replace(closed, "#8=EDGE_CURVE('',#2,#32,#7,.T.);", "#8=EDGE_CURVE('',#32,#2,#7,.F.);"),
  "#9=ORIENTED_EDGE('',*,*,#8,.T.);", "#9=ORIENTED_EDGE('',*,*,#8,.F.);"));
check(replace(closed, '(0.,1.,2.,3.)', '(0.1,0.4,1.1,1.4)'));
check(replace(closed, 'ENDSEC;END-ISO-10303-21;', `/*${'padding'.repeat(40000)}*/ENDSEC;END-ISO-10303-21;`));
console.log('PASS planar seam provider: default/explicit, metre, sense, numeric, capped, original bytes and two positive faces; asset bounds remain Unknown');
