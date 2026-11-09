// Offline generator contract checks: no upstream checkout, installed modules or Rust.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { collectExpected, validateFixture, ensureCheckout, writeFixture, SOURCE } = require('./generate-ergogen-parity-fixtures.cjs');

const translated = {
  id: 'translated-and-turned',
  boardStudioMatrices: [{ id: 'main', rows: 1, columns: 1 }],
  mapping: [{ ergogenName: 'm_a_r0', matrix: 'main', row: 0, column: 0 }],
};
assert.deepEqual(collectExpected(translated, { m_a_r0: { x: 10, y: 20, r: 30 } }), [
  { ergogenName: 'm_a_r0', matrix: 'main', row: 0, column: 0,
    pose: { x: 10, y: 20, rotation: 30 } },
]);
console.log('PASS: translated/turned point maps to its explicit matrix coordinate');

assert.throws(() => collectExpected({ ...translated, mapping: [...translated.mapping, ...translated.mapping] },
  { m_a_r0: { x: 10, y: 20, r: 30 } }), /duplicate.*point/i);
console.log('PASS: duplicate upstream point mappings are refused');

assert.throws(() => collectExpected(translated, {
  m_a_r0: { x: 10, y: 20, r: 30 }, extra: { x: 0, y: 0, r: 0 },
}), /unmapped.*extra/i);
console.log('PASS: an upstream point cannot silently disappear from a fixture');

assert.throws(() => collectExpected(translated, {}), /missing.*m_a_r0/i);
assert.throws(() => collectExpected(translated, { m_a_r0: { x: NaN, y: 20, r: 30 } }), /finite/i);
assert.throws(() => collectExpected({ ...translated, mapping: [
  ...translated.mapping, { ergogenName: 'other', matrix: 'main', row: 0, column: 0 },
] }, { m_a_r0: { x: 10, y: 20, r: 30 }, other: { x: 0, y: 0, r: 0 } }), /duplicate.*coordinate/i);
const skipped = { ...translated, mapping: [{ ...translated.mapping[0], absent: true }] };
assert.deepEqual(collectExpected(skipped, {}), skipped.mapping);
assert.throws(() => collectExpected(skipped, { m_a_r0: { x: 0, y: 0, r: 0 } }), /absent/i);
assert.throws(() => collectExpected({ ...translated, mapping: [{ ...translated.mapping[0], absent: 'no' }] }, {}), /boolean/i);
assert.throws(() => collectExpected({ ...translated, boardStudioMatrices: [{ id: 'main', rows: 2, columns: 1 }] },
  { m_a_r0: { x: 0, y: 0, r: 0 } }), /missing matrix coordinate/i);
assert.throws(() => collectExpected({ ...translated, mapping: [{ ...translated.mapping[0], row: 1 }] },
  { m_a_r0: { x: 0, y: 0, r: 0 } }), /invalid mapped coordinate/i);
console.log('PASS: missing/nonfinite/aliased points and explicitly absent output are validated');

const scenarios = require('./ergogen-parity/cases.cjs');
assert.ok(scenarios.cases.find(item => item.id === 'S1-grid'));
const padding = scenarios.cases.find(item => item.id === 'S7-padding');
assert.equal(padding.ergogenConfig.points.zones.m.rows.r1.padding, 22);
assert.equal(padding.boardStudioMatrices[0].rowLayout[2].padding, 22);
console.log('PASS: source catalog includes the agreed padding-attachment translation');

const fixture = JSON.parse(fs.readFileSync(path.join(__dirname, '../core/tests/fixtures/ergogen-layout-parity.json'), 'utf8'));
validateFixture(fixture);
assert.equal(fixture.source.commit, '8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37');
assert.deepEqual(fixture.source.dependencies, { makerjs: '0.18.2', mathjs: '15.2.0' });
assert.equal(fixture.cases.length, scenarios.cases.length);
for (const mask of ['none', 'x', 'y', 'r', 'xy', 'xr', 'yr', 'xyr']) {
  assert.ok(fixture.cases.some(item => item.id === `anchor-follow-${mask}`));
}
const point = (id, name) => fixture.cases.find(item => item.id === id).expected.find(entry => entry.ergogenName === name);
assert.deepEqual(point('translated-and-turned', 'm_a_r0').pose, { x: 10, y: 20, rotation: 30 });
assert.equal(point('full-heading-not-modulo-180', 'mirror_m_a_r0').pose.rotation, -315);
assert.equal(point('skipped-key-carries-and-anchors', 'm_b_r1').absent, true);
assert.deepEqual(point('skipped-key-carries-and-anchors', 'probe_a_r0').pose, { x: 21, y: 19, rotation: 10 });
assert.deepEqual(point('linked-partial-follow-reflects-zero', 'mirror_thumb_a_r0').pose, { x: 100, y: 30, rotation: 0 });
assert.equal(point('S15-additive-offset-divergence', 'm_b_r1').pose.x, 19);
assert.equal(fixture.cases.find(item => item.id === 'S15-additive-offset-divergence').differences[0].pose.x, 23);
const corruptWorked = structuredClone(fixture);
corruptWorked.boardStudioOnly[0].expected[0].pose.x = NaN;
assert.throws(() => validateFixture(corruptWorked), /finite/i);
console.log('PASS: committed upstream and worked fixtures have complete, finite expectations');

const packageRoot = path.join(__dirname, 'ergogen-parity');
assert.deepEqual(JSON.parse(fs.readFileSync(path.join(packageRoot, 'package.json'))).dependencies, SOURCE.dependencies);
const lock = JSON.parse(fs.readFileSync(path.join(packageRoot, 'package-lock.json')));
assert.deepEqual(lock.packages[''].dependencies, SOURCE.dependencies);
assert.equal(Object.keys(lock.packages).some(name => /(?:hull|kle-serial|node_modules\/ergogen)$/.test(name)), false);
assert.match(fs.readFileSync(path.join(packageRoot, '.npmrc'), 'utf8'), /^ignore-scripts=true$/m);
assert.throws(() => ensureCheckout(path.resolve(__dirname, '..')), /outside.*repository/i);
console.log('PASS: pin, minimal dependency lock and install-script/cache policy are explicit');

const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'boardstudio-parity-'));
try {
  const checkout = path.join(temporary, 'checkout');
  fs.mkdirSync(checkout);
  const output = path.join(temporary, 'fixture.json');
  const run = spawnSync(process.execPath, [path.join(__dirname, 'generate-ergogen-parity-fixtures.cjs'),
    '--cache', checkout, '--output', output], { encoding: 'utf8' });
  assert.notEqual(run.status, 0);
  assert.match(run.stderr, /pinned|checkout|git/i);
  assert.equal(fs.existsSync(output), false);
  const help = spawnSync(process.execPath, [path.join(__dirname, 'generate-ergogen-parity-fixtures.cjs'), '--help'],
    { encoding: 'utf8' });
  assert.equal(help.status, 0);
  assert.match(help.stdout, /--cache/);
  console.log('PASS: CLI rejects an invalid existing checkout before writing; help is offline');

  const legacy = path.join(__dirname, '../core/tests/fixtures/matrix-projections.json');
  const original = fs.existsSync(legacy) ? fs.readFileSync(legacy, 'utf8') : null;
  const protectedRun = spawnSync(process.execPath, [path.join(__dirname, 'generate-ergogen-parity-fixtures.cjs'),
    '--cache', checkout, '--output', legacy], { encoding: 'utf8' });
  assert.notEqual(protectedRun.status, 0);
  assert.match(protectedRun.stderr, /current.*oracle/i);
  assert.equal(fs.existsSync(legacy) ? fs.readFileSync(legacy, 'utf8') : null, original);
  console.log('PASS: generation cannot overwrite the still-active legacy oracle');

  const git = args => {
    const result = spawnSync('git', ['-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', ...args],
      { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
  };
  git(['init', '--quiet', checkout]);
  git(['-C', checkout, '-c', 'user.name=Fixture test', '-c', 'user.email=fixture@example.invalid',
    'commit', '--quiet', '--allow-empty', '-m', 'Unrelated fixture commit']);
  const wrongPin = spawnSync(process.execPath, [path.join(__dirname, 'generate-ergogen-parity-fixtures.cjs'),
    '--cache', checkout, '--output', output], { encoding: 'utf8' });
  assert.notEqual(wrongPin.status, 0);
  assert.match(wrongPin.stderr, /pinned Ergogen commit/);
  assert.equal(fs.existsSync(output), false);
  console.log('PASS: an existing checkout at a different HEAD is not moved or used');

  const unrelated = `${output}.tmp-${process.pid}`;
  fs.writeFileSync(unrelated, 'unrelated file\n');
  try {
    writeFixture(output, 'new fixture\n');
  } finally {
    assert.equal(fs.readFileSync(unrelated, 'utf8'), 'unrelated file\n');
  }
  assert.equal(fs.readFileSync(output, 'utf8'), 'new fixture\n');
  console.log('PASS: atomic output never removes an unrelated pre-existing temporary file');
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}

// Opt-in provenance check; normal tooling never needs an upstream checkout.
if (process.argv[2]) {
  const cache = path.resolve(process.argv[2]);
  ensureCheckout(cache);
  const probe = path.join(cache, `boardstudio-parity-probe-${process.pid}.txt`);
  fs.writeFileSync(probe, 'untracked provenance probe\n', { flag: 'wx' });
  try {
    assert.throws(() => ensureCheckout(cache), /modifications|untracked/i);
    console.log('PASS: untracked cache content is refused at the correct pinned commit');
  } finally {
    fs.unlinkSync(probe);
  }
}
