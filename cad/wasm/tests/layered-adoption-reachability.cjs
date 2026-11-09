// Recorded producer/backend dispositions, not acceptance of new product exclusions.
// Replay Core's regression first, then: node this-file /absolute/path/to/provider.js
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const cad = require(path.resolve(process.argv[2]));
const fixture = JSON.parse(fs.readFileSync(path.join(__dirname,
  '../../../core/tests/fixtures/adoption-reachability.json')));
const baseline = cad.build_case(fixture.baselinePreparedBottom);
assert.equal((Buffer.from(baseline.step).toString().match(/MANIFOLD_SOLID_BREP\(/g) || []).length, 1);
const cross = (a, b) => [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]];
const dot = (a, b) => a.reduce((sum, value, i) => sum + value*b[i], 0);
function geometry(input, probes = []) {
  const analysis = cad.analyze_generated_body(input, {targetDeflectionMm: 1e-4});
  const soup = analysis.surfacePositionsMmF64;
  assert(soup instanceof Float64Array && soup.length > 0 && soup.length % 9 === 0);
  let volume = 0;
  const angles = probes.map(() => 0);
  const bounds = [[Infinity, Infinity, Infinity], [-Infinity, -Infinity, -Infinity]];
  for (let index = 0; index < soup.length; index += 9) {
    const triangle = [0, 3, 6].map(offset => [...soup.slice(index+offset, index+offset+3)]);
    volume += dot(triangle[0], cross(triangle[1], triangle[2])) / 6;
    for (const point of triangle) point.forEach((value, axis) => {
      assert(Number.isFinite(value));
      bounds[0][axis] = Math.min(bounds[0][axis], value);
      bounds[1][axis] = Math.max(bounds[1][axis], value);
    });
    probes.forEach((probe, i) => {
      const [a,b,c] = triangle.map(point => point.map((value, axis) => value-probe.pointMm[axis]));
      const magnitude = v => Math.hypot(...v);
      const denominator = magnitude(a)*magnitude(b)*magnitude(c)
        + dot(a,b)*magnitude(c) + dot(b,c)*magnitude(a) + dot(c,a)*magnitude(b);
      angles[i] += 2*Math.atan2(dot(a,cross(b,c)), denominator);
    });
  }
  probes.forEach((probe, i) => assert(Math.abs(angles[i]/(4*Math.PI)-Number(probe.inside)) < 1e-8));
  return {volume, bounds, meshClosedOriented: analysis.meshClosedOriented,
    analysisErrorBoundMm: analysis.analysisErrorBoundMm};
}
for (const item of fixture.cases) {
  const expected = item.expectedBackend;
  if (expected.state === 'named-refusal') {
    assert.throws(() => cad.build_case(item.preparedBody), error => {
      assert(String(error).includes(expected.reason));
      assert(String(error).includes(expected.featureId));
      console.log(JSON.stringify({id: item.prototypeId, state: expected.state, error: String(error)}));
      return true;
    }, 'update the recorded disposition if construction becomes supported');
    continue;
  }
  const built = cad.build_case(item.preparedBody);
  assert.equal(built.revision, item.preparedBody.revision);
  assert(built.mesh.positions instanceof Float32Array && built.mesh.positions.length > 0);
  assert(built.mesh.positions.every(Number.isFinite));
  const step = Buffer.from(built.step).toString();
  assert(step.includes('SI_UNIT(.MILLI.,.METRE.)'));
  for (const [entity, count] of [
    ['MANIFOLD_SOLID_BREP', expected.solidCount], ['CLOSED_SHELL', expected.shellCount],
    ['CYLINDRICAL_SURFACE', expected.cylindricalFaces], ['RATIONAL_B_SPLINE_SURFACE', expected.conicalFaces],
  ]) assert.equal((step.match(new RegExp(`${entity}\\(`, 'g')) || []).length, count);
  if (item.prototypeId === 'opening-ring-island') {
    console.error('Product ring: baseline f64 analysis');
    const before = geometry(fixture.baselinePreparedBottom);
    console.error('Product ring: opened-body f64 analysis');
    const after = geometry(item.preparedBody, expected.materialProbes);
    const removedVolumeMm3 = before.volume-after.volume;
    assert(Math.abs(removedVolumeMm3-expected.removedVolumeMm3) < 0.002);
    after.bounds.flat().forEach((value, i) => assert(Math.abs(value-expected.boundsMm.flat()[i]) < 1e-8));
    assert.deepEqual(after.bounds, before.bounds);
    console.log(JSON.stringify({id: item.prototypeId, state: 'implemented product ring island',
      removedVolumeMm3, ...after}));
  } else {
    console.log(JSON.stringify({id: item.prototypeId, state: 'supported hardware probe; prototype not reproduced',
      solidCount: expected.solidCount, shellCount: expected.shellCount}));
  }
}
