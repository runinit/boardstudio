// Actual provider compatibility; the private operation proof is tested natively.
import { assert, readText, toBytes, sha256Hex, runCases } from './provider-support.mjs';

export async function run(cad, memory) {
return await runCases('step-planar-seam-repair', [['main', async () => {
const source = await readText('cad/monstertruck/tests/fixtures/repair-planar-seam.step');

function replace(text, before, after) {
  assert.equal(text.split(before).length - 1, 1);
  return text.replace(before, after);
}

const closed = replace(replace(replace(source,
  "#10=EDGE_LOOP('',(#9));", "#10=EDGE_LOOP('',(#9,#74));"),
  "#13=CLOSED_SHELL('',(#12));", "#13=CLOSED_SHELL('',(#12,#75));"),
  'ENDSEC;END-ISO-10303-21;',
  "#70=DIRECTION('',(0.,-1.,0.));#71=VECTOR('',#70,1.);#72=LINE('',#34,#71);#73=EDGE_CURVE('',#32,#2,#72,.T.);#74=ORIENTED_EDGE('',*,*,#73,.T.);#75=ADVANCED_FACE('',(#11),#6,.F.);ENDSEC;END-ISO-10303-21;");

async function options(bytes, selected) {
  return {sourceSha256: await sha256Hex(bytes), ...selected};
}

const openBytes = toBytes(source);
const openSettings = await options(openBytes, {});
assert.throws(() => cad.read_step_model(new Uint8Array(openBytes), openSettings), /open boundary/);

async function check(text, selected = {}, scale = 1) {
  const bytes = toBytes(text), input = new Uint8Array(bytes), settings = await options(bytes, selected);
  const result = cad.read_step_model(input, settings);
  assert.deepEqual(new Uint8Array(input), bytes);
  const vendor = result.vendorStep;
  assert.equal(vendor.sourceSha256, settings.sourceSha256);
  assert.equal(vendor.accuracy.maxRepairDisplacementMm, null);
  assert.equal(vendor.accuracy.analysisErrorBoundMm, null);
  assert.deepEqual(vendor.sourceFaces.map(face => face.faceId).sort(), ['12', '75']);
  assert(vendor.sourceFaces.every(face => face.solidId === '14'));
  assert.equal(vendor.faceOccurrences.length, 2);
  assert.deepEqual([...new Set(vendor.triangleFaceOccurrenceIndices)].sort(), [0, 1]);
  assert(vendor.surfacePositionsMmF64.length > 0);
  assert.equal(vendor.surfacePositionsMmF64.length, vendor.triangleFaceOccurrenceIndices.length * 9);
  assert(vendor.surfacePositionsMmF64.every(Number.isFinite));
  assert(vendor.faceOccurrences.every(face => face.repairDisplacementMm === null));
  assert.deepEqual(vendor.faceOccurrences.map(face => face.sourceFaceIndex).sort(), [0, 1]);
  for (const axis of [0, 1, 2]) {
    const expected = [[0, -scale, 0], [scale, scale, 0]];
    for (const [i, key] of ['min', 'max'].entries()) {
      assert(Math.abs(result.bounds[key][axis] - expected[i][axis]) < scale * 1e-6);
    }
  }
  vendor.analysisHandle.free();
}

await check(closed);
await check(closed, {previewRepairBudgetMm: 0.02});
await check(closed, {previewRepairBudgetMm: 0.001});
await check(replace(closed, 'SI_UNIT(.MILLI.,.METRE.)', 'SI_UNIT($,.METRE.)'), {}, 1000);
await check(replace(replace(closed, "#8=EDGE_CURVE('',#2,#32,#7,.T.);", "#8=EDGE_CURVE('',#32,#2,#7,.F.);"),
  "#9=ORIENTED_EDGE('',*,*,#8,.T.);", "#9=ORIENTED_EDGE('',*,*,#8,.F.);"));
await check(replace(closed, '(0.,1.,2.,3.)', '(0.1,0.4,1.1,1.4)'));
await check(replace(closed, 'ENDSEC;END-ISO-10303-21;', `/*${'padding'.repeat(40000)}*/ENDSEC;END-ISO-10303-21;`));
console.log('PASS planar seam provider: default/explicit, metre, sense, numeric, capped, original bytes and two source faces; asset bounds remain Unknown');
}]]);
}
