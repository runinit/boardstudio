// Trusted Node reference and fixture comparison, run separately from the Node-free browser probe.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { cases, boardStudioOnly } = require('./cases.cjs');
const { collectExpected, validateFixture, SOURCE } = require('../generate-ergogen-parity-fixtures.cjs');

if (process.argv[2] === 'reference') {
  const root = path.resolve(process.argv[3]);
  const points = require(path.join(root, 'upstream/src/points.js'));
  const units = require(path.join(root, 'upstream/src/units.js'));
  const reference = cases.map(item => ({ case: item.id, config: item.ergogenConfig,
    result: Object.fromEntries(Object.entries(points.parse(structuredClone(item.ergogenConfig.points),
      units.parse(item.ergogenConfig.units || {}))).map(([name, point]) => [name, { x: point.x, y: point.y, r: point.r }])) }));
  reference.forEach((item, index) => collectExpected(cases[index], item.result));
  process.stdout.write(JSON.stringify(reference));
} else if (process.argv[2] === 'compare') {
  const actual = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
  const reference = JSON.parse(fs.readFileSync(process.argv[4], 'utf8'));
  if (!Array.isArray(actual) || actual.length !== cases.length) throw Error('Oracle case count mismatch');
  assert.equal(reference.length, cases.length);
  cases.forEach((item, index) => {
    assert.equal(reference[index].case, item.id);
    assert.deepEqual(reference[index].config, item.ergogenConfig);
    assert.deepEqual(actual[index], reference[index].result);
  });
  const fixture = { fixtureVersion: 1, targetDocumentFormatVersion: 7, source: SOURCE,
    tolerances: { upstream: 1e-6, boardStudioOnly: 1e-9 },
    cases: cases.map((scenario, index) => ({ ...scenario, expected: collectExpected(scenario, actual[index]) })),
    boardStudioOnly };
  validateFixture(fixture);
  const generated = JSON.stringify(fixture, null, 2) + '\n';
  const committed = fs.readFileSync(path.join(__dirname, '../../core/tests/fixtures/ergogen-layout-parity.json'), 'utf8');
  if (generated !== committed) throw Error('Fixture differs');
  console.log(`Fixture byte-identical: ${cases.length} cases, ${actual.reduce((sum, points) => sum + Object.keys(points).length, 0)} points, ${Buffer.byteLength(generated)} bytes`);
} else throw Error('expected reference or compare');
