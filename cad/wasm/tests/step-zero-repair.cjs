// Actual packaged provider: node this-file /absolute/path/to/package.js [--web].
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');

(async () => {
const cad = await require('./load-provider.cjs')(process.argv[2]);
const fixtures = path.resolve(__dirname, '../../monstertruck/tests/fixtures');
const box = fs.readFileSync(path.join(fixtures, 'zero-repair-cuboid-mm.step'));
const offset = fs.readFileSync(path.join(fixtures, 'zero-repair-cuboid-offset-mm.step'));

function imported(bytes, options = {}) {
  bytes = Buffer.from(bytes);
  const original = Buffer.from(bytes);
  const digest = crypto.createHash('sha256').update(bytes).digest('hex');
  let result;
  try {
    result = cad.read_step_model(new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength), {
      sourceSha256: digest, ...options,
    });
  } finally {
    assert.deepEqual(bytes, original, 'caller source bytes remain immutable, including refusal');
  }
  assert.equal(result.vendorStep.sourceSha256, digest);
  assert.equal(result.vendorStep.accuracy.analysisErrorBoundMm, result.vendorStep.sourceQualification.state === 'bounded' ? result.vendorStep.sourceQualification.totalBoundMm : null);
  assert(result.mesh.positions instanceof Float32Array);
  assert(result.mesh.normals instanceof Float32Array);
  const asset = result.vendorStep;
  assert(asset.surfacePositionsMmF64 instanceof Float64Array);
  assert(asset.triangleFaceOccurrenceIndices instanceof Uint32Array);
  assert.equal(asset.surfacePositionsMmF64.length, asset.triangleFaceOccurrenceIndices.length * 9);
  assert.equal(asset.surfacePositionsMmF64.length, result.mesh.positions.length);
  const used = new Set(asset.triangleFaceOccurrenceIndices);
  for (let i = 0; i < asset.faceOccurrences.length; i++) assert(used.has(i));
  for (const tag of used) assert(tag < asset.faceOccurrences.length);
  for (const face of asset.sourceFaces) {
    for (const field of ['solidId', 'faceId', 'supportId']) assert.match(face[field], /^\d+$/);
  }
  assert(asset.faceOccurrences.every(face => face.repairDisplacementMm === null),
    'per-face wire metadata is unchanged');
  for (let i = 0; i < asset.surfacePositionsMmF64.length; i++) {
    assert(Number.isFinite(asset.surfacePositionsMmF64[i]));
    assert.equal(result.mesh.positions[i], Math.fround(asset.surfacePositionsMmF64[i]));
  }
  result.vendorStep.analysisHandle.free();
  return result;
}

test('actual WASM integer cuboid reports complete zero and unchanged default/provenance', () => {
  const result = imported(box);
  assert.deepEqual(result.bounds, { min: [0, 0, 0], max: [2, 4, 8] });
  const { accuracy, sourceFaces, faceOccurrences } = result.vendorStep;
  assert.equal(sourceFaces.length, 6);
  assert.equal(faceOccurrences.length, 6);
  assert.equal(accuracy.maxRepairDisplacementMm, 0);
  assert.equal(accuracy.analysisErrorBoundMm, null, 'zero repair is not a source certificate');
  assert.equal(result.vendorStep.sourceQualification.state, 'unknown');
  assert(result.vendorStep.sourceQualification.reasons.includes('missing source precision'));
  assert.equal(accuracy.previewDeflectionMm, 0.1);
  assert.equal(accuracy.previewFloorMm, 0.02);
  assert.equal(accuracy.previewDeclaredMultiplier, 2);
  assert.equal(accuracy.selectedRepairBudgetMm, undefined);
});

test('independently authored shifted cuboid and reversed native senses qualify', () => {
  const result = imported(offset);
  assert.deepEqual(result.bounds, { min: [-3, 2, -4], max: [5, 8, 2] });
  assert.equal(result.vendorStep.accuracy.maxRepairDisplacementMm, 0);
});

test('explicit precision, budget and refinement remain separate from zero movement', () => {
  const declared = box.toString().replace(
    "#303=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#302)));",
    "#303=(GEOMETRIC_REPRESENTATION_CONTEXT(3)REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#302))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#900)));")
    .replace('ENDSEC;END-ISO-10303-21;', "#900=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#302,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;");
  const result = imported(declared, { previewRepairBudgetMm: 0.03, targetDeflectionMm: 0.01 });
  assert.equal(result.vendorStep.accuracy.maxRepairDisplacementMm, 0);
  assert.equal(result.vendorStep.accuracy.selectedRepairBudgetMm, 0.03);
  assert.equal(result.vendorStep.sourceQualification.state, 'bounded', 'complete independent LINE/plane proof');
  assert.equal(result.vendorStep.sourceQualification.directBoundMm, 0);
  assert.equal(result.vendorStep.sourceQualification.sourcePrecisionMm, 0.001);
  assert.equal(result.vendorStep.sourceQualification.totalBoundMm, 0.001);
  assert.equal(result.vendorStep.accuracy.analysisErrorBoundMm, 0.001);
  assert.equal(result.vendorStep.accuracy.previewDeflectionMm, 0.01);
  assert.equal(result.vendorStep.importOptions.targetDeflectionMm, 0.01);
  assert(result.vendorStep.faceOccurrences.every(face => face.sourceUncertaintyMm === 0.001));
});

test('rounded source lexemes, inconsistent native LINE and selected PCurves stay null', () => {
  const rounded = box.toString().replace("#2=CARTESIAN_POINT('',(2.,0.,0.));", "#2=CARTESIAN_POINT('',(2.0000000000000001,0.,0.));");
  const offLine = box.toString().replace("#32=LINE('',#1,#31);", "#900=CARTESIAN_POINT('',(0.,1.,0.));#32=LINE('',#900,#31);");
  const selected = fs.readFileSync(path.join(fixtures, 'source-pcurve-cube.step'));
  for (const bytes of [rounded, selected]) {
    assert.equal(imported(bytes).vendorStep.accuracy.maxRepairDisplacementMm, null);
  }
  // This actual one-millimetre finite LINE operation used to pass attachment
  // guards. The complete operation bound now enforces the selected allowance;
  // admission with an explicit equality budget still cannot certify the asset.
  assert.throws(() => imported(offLine), error =>
    typeof error === 'string' && error.includes('finite LINE repair') && error.includes('exceeds repair budget 0.02'));
  assert.equal(imported(offLine, {previewRepairBudgetMm: 1}).vendorStep.accuracy.maxRepairDisplacementMm, null);
  const sliver = imported(fs.readFileSync(path.join(fixtures, 'circular-segment-sliver-prism.step')));
  assert.equal(sliver.vendorStep.accuracy.maxRepairDisplacementMm, null);
  assert(sliver.vendorStep.accuracy.fallbackMeshedFaces > 0);
});

test('mixed asset cannot borrow a supported cuboid proof', () => {
  const second = offset.toString().split('ENDSEC;DATA;')[1].replace('ENDSEC;END-ISO-10303-21;', '');
  const mixed = box.toString().replace('ENDSEC;END-ISO-10303-21;', `${second}ENDSEC;END-ISO-10303-21;`);
  const result = imported(mixed);
  assert.equal(result.vendorStep.sourceFaces.length, 12);
  assert.equal(result.vendorStep.accuracy.maxRepairDisplacementMm, null);
});

test('existing tetra plane-gap default and explicit budget policy stays unchanged', () => {
  const source = fs.readFileSync(path.join(fixtures, 'tetra-mm.step'), 'utf8')
    .replace("#103=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));", "#103=(GEOMETRIC_REPRESENTATION_CONTEXT(3)REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300)));")
    .replace("#57=AXIS2_PLACEMENT_3D('',#1,#55,#56);", "#310=CARTESIAN_POINT('',(0.,0.,0.025));#57=AXIS2_PLACEMENT_3D('',#310,#55,#56);")
    .replace('ENDSEC;END-ISO-10303-21;', "#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#102,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;");
  assert.throws(() => imported(source));
  assert.throws(() => imported(source, { previewRepairBudgetMm: 0.024 }));
  const result = imported(source, { previewRepairBudgetMm: 0.03 });
  assert.equal(result.vendorStep.accuracy.selectedRepairBudgetMm, 0.03);
  assert.equal(result.vendorStep.accuracy.maxRepairDisplacementMm, null);
  assert(result.vendorStep.faceOccurrences.every(face => face.sourceUncertaintyMm === 0.001));
});

test('unretained styled finite curve and alternate presentation roots explicitly refuse', () => {
  // #32 has unit +X direction through the origin: [10,11] is outside [0,2].
  const styled = box.toString().replace('ENDSEC;END-ISO-10303-21;',
    "#901=TRIMMED_CURVE('outside-body',#32,(PARAMETER_VALUE(10.)),(PARAMETER_VALUE(11.)),.T.,.PARAMETER.);" +
    "#902=STYLED_ITEM('',(#904),#901);" +
    "#903=MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#902),#303);" +
    "#904=PRESENTATION_STYLE_ASSIGNMENT((#905));" +
    "#905=CURVE_STYLE('',#906,POSITIVE_LENGTH_MEASURE(0.1),#907);" +
    "#906=DRAUGHTING_PRE_DEFINED_CURVE_FONT('continuous');" +
    "#907=COLOUR_RGB('',1.,0.,0.);" +
    'ENDSEC;END-ISO-10303-21;');
  const sources = [styled, ...['DRAUGHTING_MODEL', 'PRESENTATION_REPRESENTATION',
    'GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION'].map(kind =>
    box.toString().replace('ENDSEC;END-ISO-10303-21;',
      `#903=${kind}('',(#32),#303);ENDSEC;END-ISO-10303-21;`))];
  for (const source of sources) {
    assert.throws(() => imported(source), /source completeness/);
  }
});

test('styles of retained finite geometry preserve complete mixed and surface assets', () => {
  const wires = fs.readFileSync(path.join(fixtures, 'tetra-and-finite-wires.step'), 'utf8');
  for (const [source, context, targets] of [[box.toString(), 303, [301, 110, 33, 304]], [wires, 103, [414, 420]]]) {
    for (const target of targets) {
      const styled = source.replace('ENDSEC;END-ISO-10303-21;',
        `#901=STYLED_ITEM('',(#904),#${target});` +
        `#902=MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#901),#${context});` +
        "#904=PRESENTATION_STYLE_ASSIGNMENT((#905));#905=CURVE_STYLE('',#906,POSITIVE_LENGTH_MEASURE(0.1),#907);" +
        "#906=DRAUGHTING_PRE_DEFINED_CURVE_FONT('continuous');#907=COLOUR_RGB('',1.,0.,0.);ENDSEC;END-ISO-10303-21;");
      const result = imported(styled);
      assert.equal(result.vendorStep.accuracy.maxRepairDisplacementMm, null);
      assert.equal(result.vendorStep.lineOccurrences.length, source === wires ? 2 : 0);
    }
  }
});

})().catch(error => { console.error(error); process.exitCode = 1; });
