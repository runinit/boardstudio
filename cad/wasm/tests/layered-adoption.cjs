// Focused actual-provider check: node this-file /absolute/path/to/nodejs-package.js
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const cad = require(path.resolve(process.argv[2]));
const directory = path.join(__dirname, '../../monstertruck/tests/fixtures/adoption');
const expectations = JSON.parse(fs.readFileSync(path.join(directory, 'expectations.json')));
const split = JSON.parse(fs.readFileSync(path.join(directory, 'pcb-drill-in-slot.json')));
split.body.id = 'pcb-slot-material-split';
split.body.mounts = [];
split.body.pcb_source.slots[0].geometry.size = {x: 2, y: 20};
expectations.push({
  id: split.body.id, preparedBody: split, expectedRefusal: null,
  volumeMm3: 2*9*16*1.6, boundsMm: [[0,0,0], [20,16,1.6]],
  solidCount: 2, shellCount: 2, coneFaces: 0, cylindricalFaces: 0,
  materialProbes: [
    {pointMm: [1,8,0.8], inside: true}, {pointMm: [19,8,0.8], inside: true},
    {pointMm: [10,8,0.8], inside: false},
  ],
});
const cross = (a, b) => [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]];
const dot = (a, b) => a.reduce((sum, value, i) => sum + value*b[i], 0);
const magnitude = a => Math.hypot(...a);
const results = [];
for (const expected of expectations) {
  const input = expected.preparedBody || JSON.parse(fs.readFileSync(path.join(directory, `${expected.id}.json`)));
  if (expected.expectedRefusal) {
    assert.throws(() => cad.build_case(input), error => {
      assert(String(error).includes(expected.expectedRefusal), `${expected.id}: ${String(error)}`);
      results.push({id: expected.id, state: 'recorded refusal; production scope see adoption note', error: String(error),
        expectedVolumeMm3: expected.volumeMm3, expectedShellCount: expected.shellCount});
      return true;
    }, `${expected.id}: update the inventory when this gap is implemented`);
    continue;
  }
  const built = cad.build_case(input);
  assert.equal(built.revision, input.revision);
  assert(built.step instanceof Uint8Array && built.mesh.positions instanceof Float32Array);
  const step = Buffer.from(built.step).toString();
  assert.equal((step.match(/MANIFOLD_SOLID_BREP\(/g) || []).length, expected.solidCount);
  assert.equal((step.match(/CLOSED_SHELL\(/g) || []).length, expected.shellCount);
  assert.equal((step.match(/RATIONAL_B_SPLINE_SURFACE\(/g) || []).length, expected.coneFaces);
  if (expected.cylindricalFaces !== undefined) {
    assert.equal((step.match(/CYLINDRICAL_SURFACE\(/g) || []).length, expected.cylindricalFaces);
    assert(!step.includes('B_SPLINE_SURFACE'), 'PCB walls remain exact planes/cylinders');
    assert(step.includes('SI_UNIT(.MILLI.,.METRE.)'));
  }
  const analysis = cad.analyze_generated_body(input, {targetDeflectionMm: 1e-4});
  const soup = analysis.surfacePositionsMmF64;
  assert(soup instanceof Float64Array && soup.length > 0 && soup.length % 9 === 0);
  let volume = 0;
  const bounds = [[Infinity, Infinity, Infinity], [-Infinity, -Infinity, -Infinity]];
  const triangles = [];
  for (let index = 0; index < soup.length; index += 9) {
    const triangle = [0, 3, 6].map(offset => [...soup.slice(index+offset, index+offset+3)]);
    triangle.flat().forEach(value => assert(Number.isFinite(value)));
    volume += dot(triangle[0], cross(triangle[1], triangle[2])) / 6;
    triangles.push(triangle);
    for (const point of triangle) point.forEach((value, axis) => {
      bounds[0][axis] = Math.min(bounds[0][axis], value);
      bounds[1][axis] = Math.max(bounds[1][axis], value);
    });
  }
  assert(Math.abs(volume-expected.volumeMm3) < 0.002, `${expected.id}: analytic volume ${volume}`);
  bounds.flat().forEach((value, index) => assert(Math.abs(value-expected.boundsMm.flat()[index]) < 1e-8));
  for (const probe of expected.materialProbes) {
    let angle = 0;
    for (const triangle of triangles) {
      const [a,b,c] = triangle.map(point => point.map((value, axis) => value-probe.pointMm[axis]));
      const denominator = magnitude(a)*magnitude(b)*magnitude(c)
        + dot(a,b)*magnitude(c) + dot(b,c)*magnitude(a) + dot(c,a)*magnitude(b);
      angle += 2*Math.atan2(dot(a,cross(b,c)), denominator);
    }
    assert(Math.abs(angle/(4*Math.PI)-Number(probe.inside)) < 1e-8, `material ${probe.pointMm}`);
  }
  results.push({id: expected.id, state: 'implemented', volumeMm3: volume, boundsMm: bounds,
    meshClosedOriented: analysis.meshClosedOriented, analysisErrorBoundMm: analysis.analysisErrorBoundMm});
}
const erased = JSON.parse(fs.readFileSync(path.join(directory, 'pcb-edge-drill.json')));
erased.body.mounts[0].holeDiameter = 100;
assert.throws(() => cad.build_case(erased), error => String(error).includes('removes all source material'));
const tangent = JSON.parse(fs.readFileSync(path.join(directory, '../pcb-source/circle-tracer.json')));
tangent.body.mounts = [{id: 'boundary-drill', at: {x: 11.5, y: 8}, kind: 'hole', holeDiameter: 1}];
assert.throws(() => cad.build_case(tangent), error => String(error).includes('boundary-drill'));
const badOwner = structuredClone(erased);
badOwner.body.pcb_source.contours[0].outerId = 'missing-owner';
assert.throws(() => cad.build_case(badOwner), error => String(error).includes('missing or non-exterior owner'));
console.log(JSON.stringify(results, null, 2));
