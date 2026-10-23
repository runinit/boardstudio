// Isolated loader: NODE_PATH is supplied before Node initializes its module search paths.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const { SOURCE } = require('../generate-ergogen-parity-fixtures.cjs');

const checkout = process.argv[2];
const upstream = createRequire(path.join(checkout, 'src/points.js'));
for (const [name, version] of Object.entries(SOURCE.dependencies)) {
  assert.equal(upstream.resolve(name), require.resolve(name), `${name} resolves outside the locked tooling environment`);
  const installed = JSON.parse(fs.readFileSync(path.join(__dirname, 'node_modules', name, 'package.json'), 'utf8'));
  assert.equal(installed.version, version, `${name} version mismatch; run npm ci --ignore-scripts --prefix scripts/ergogen-parity`);
}
const points = upstream('./points.js');
const units = upstream('./units.js');
const configs = JSON.parse(fs.readFileSync(0, 'utf8'));
const result = configs.map(config => Object.fromEntries(Object.entries(
  points.parse(structuredClone(config.points), units.parse(config.units || {})),
).map(([name, point]) => [name, { x: point.x, y: point.y, r: point.r }])));
process.stdout.write(JSON.stringify(result));
