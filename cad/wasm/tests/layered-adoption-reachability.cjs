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
  console.log(JSON.stringify({id: item.prototypeId, state: 'supported hardware probe; prototype not reproduced',
    solidCount: expected.solidCount, shellCount: expected.shellCount}));
}
