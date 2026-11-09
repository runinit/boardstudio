const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { execFileSync } = require('node:child_process');
const { parseArgs } = require('node:util');
const assert = require('node:assert/strict');

const SOURCE = Object.freeze({
  repository: 'https://github.com/ergogen/ergogen',
  commit: '8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37',
  version: '4.2.1',
  entry: 'src/points.js:parse',
  license: 'MIT',
  dependencies: { makerjs: '0.18.2', mathjs: '15.2.0' },
});
const ROOT = path.resolve(__dirname, '..');

function matrixIndex(values) {
  const matrices = new Map(values.map(matrix => [matrix.id, matrix]));
  if (matrices.size !== values.length) throw new Error('Duplicate matrix identity');
  let slots = 0;
  for (const matrix of matrices.values()) {
    if (typeof matrix.id !== 'string' || !matrix.id || !Number.isSafeInteger(matrix.rows) || matrix.rows < 1 ||
        !Number.isSafeInteger(matrix.columns) || matrix.columns < 1) throw new Error('Invalid matrix dimensions');
    slots += matrix.rows * matrix.columns;
  }
  if (!Number.isSafeInteger(slots) || slots < 1) throw new Error('Invalid matrix coordinate count');
  return { matrices, slots };
}

function coordinateKey(matrices, entry) {
  const matrix = matrices.get(entry.matrix);
  if (!matrix || !Number.isInteger(entry.row) || !Number.isInteger(entry.column) ||
      entry.row < 0 || entry.row >= matrix.rows || entry.column < 0 || entry.column >= matrix.columns) {
    throw new Error(`Invalid mapped coordinate: ${JSON.stringify(entry)}`);
  }
  return JSON.stringify([entry.matrix, entry.row, entry.column]);
}

function collectExpected(scenario, points) {
  const seen = new Set(), coordinates = new Set();
  const { matrices, slots } = matrixIndex(scenario.boardStudioMatrices);
  const expected = scenario.mapping.map(mapping => {
    if (typeof mapping.ergogenName !== 'string' || !mapping.ergogenName) throw new Error('Missing point name');
    if (seen.has(mapping.ergogenName)) throw new Error(`Duplicate point mapping: ${mapping.ergogenName}`);
    seen.add(mapping.ergogenName);
    const coordinate = coordinateKey(matrices, mapping);
    if (coordinates.has(coordinate)) throw new Error(`Duplicate mapped coordinate: ${coordinate}`);
    coordinates.add(coordinate);
    if (Object.hasOwn(mapping, 'absent') && typeof mapping.absent !== 'boolean') throw new Error('Absent marker must be boolean');
    if (mapping.absent) {
      if (Object.hasOwn(points, mapping.ergogenName)) throw new Error(`Expected absent point: ${mapping.ergogenName}`);
      return { ...mapping };
    }
    if (!Object.hasOwn(points, mapping.ergogenName)) throw new Error(`Missing point: ${mapping.ergogenName}`);
    const point = points[mapping.ergogenName];
    if (![point.x, point.y, point.r].every(Number.isFinite)) throw new Error(`Point must be finite: ${mapping.ergogenName}`);
    return { ...mapping, pose: { x: point.x, y: point.y, rotation: point.r } };
  });
  for (const name of Object.keys(points)) {
    if (!seen.has(name)) throw new Error(`Unmapped upstream point: ${name}`);
  }
  if (coordinates.size !== slots) throw new Error(`Missing matrix coordinate mapping in ${scenario.id}`);
  return expected;
}

function validateFixture(fixture) {
  assert.equal(fixture.fixtureVersion, 1);
  assert.equal(fixture.targetDocumentFormatVersion, 7);
  assert.deepEqual(fixture.source, SOURCE);
  assert.deepEqual(fixture.tolerances, { upstream: 1e-6, boardStudioOnly: 1e-9 });
  const ids = new Set();
  const identity = item => {
    if (typeof item.id !== 'string' || !item.id || ids.has(item.id)) throw new Error('Missing/duplicate case identity');
    ids.add(item.id);
  };
  const poses = (values, expected, complete = true) => {
    const { matrices, slots } = matrixIndex(values), seen = new Set();
    for (const entry of expected) {
      const key = coordinateKey(matrices, entry);
      if (seen.has(key)) throw new Error(`Duplicate expected coordinate: ${key}`);
      seen.add(key);
      if (!entry.pose || ![entry.pose.x, entry.pose.y, entry.pose.rotation].every(Number.isFinite)) {
        throw new Error(`Expected pose must be finite: ${key}`);
      }
    }
    if (complete && seen.size !== slots) throw new Error('Missing worked coordinate');
  };
  for (const item of fixture.cases) {
    identity(item);
    const points = Object.fromEntries(item.expected.filter(entry => !entry.absent).map(entry =>
      [entry.ergogenName, { x: entry.pose.x, y: entry.pose.y, r: entry.pose.rotation }]));
    assert.deepEqual(collectExpected(item, points), item.expected);
    if (item.comparison === 'intentional-divergence') {
      if (!item.differences?.length) throw new Error('Divergence needs independent BoardStudio expectations');
      poses(item.boardStudioMatrices, item.differences, false);
      for (const entry of item.differences) {
        const counterpart = item.expected.find(point => point.matrix === entry.matrix &&
          point.row === entry.row && point.column === entry.column);
        if (!counterpart?.pose || ['x', 'y', 'rotation'].every(axis =>
          Math.abs(entry.pose[axis] - counterpart.pose[axis]) <= fixture.tolerances.upstream)) {
          throw new Error('Declared divergence does not differ from its upstream point');
        }
      }
    } else if (item.comparison !== 'match') throw new Error('Unknown comparison kind');
  }
  for (const item of fixture.boardStudioOnly) {
    identity(item);
    if (!item.basis) throw new Error('Worked example needs its independent basis');
    if (item.beforeMatrices) {
      poses(item.beforeMatrices, item.expectedBefore);
      poses(item.afterMatrices, item.expectedAfter);
    } else poses(item.boardStudioMatrices, item.expected);
  }
}

function realDestination(destination) {
  let existing = path.resolve(destination);
  const suffix = [];
  while (!fs.existsSync(existing)) {
    suffix.unshift(path.basename(existing));
    const parent = path.dirname(existing);
    if (parent === existing) throw new Error(`Cannot resolve path: ${destination}`);
    existing = parent;
  }
  return path.join(fs.realpathSync(existing), ...suffix);
}

function ensureCheckout(cache) {
  const relative = path.relative(fs.realpathSync(ROOT), realDestination(cache));
  if (!relative || (relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative))) {
    throw new Error('The pinned source checkout must be outside the repository');
  }
  const git = args => execFileSync('git', ['-c', 'core.hooksPath=/dev/null', ...args],
    { encoding: 'utf8', env: { ...process.env, GIT_TERMINAL_PROMPT: '0' }, maxBuffer: 8 * 1024 * 1024 }).trim();
  if (!fs.existsSync(cache)) {
    // shortcut: interrupted bootstrap leaves its cache for inspection, make creation atomic if concurrent generators are needed.
    fs.mkdirSync(path.dirname(cache), { recursive: true });
    git(['init', '--quiet', cache]);
    git(['-C', cache, 'fetch', '--quiet', '--depth=1', SOURCE.repository, SOURCE.commit]);
    git(['-C', cache, 'checkout', '--quiet', '--detach', 'FETCH_HEAD']);
  }
  if (fs.realpathSync(git(['-C', cache, 'rev-parse', '--show-toplevel'])) !== fs.realpathSync(cache)) {
    throw new Error('Cache must name the pinned checkout root');
  }
  if (git(['-C', cache, 'rev-parse', 'HEAD']) !== SOURCE.commit) throw new Error('Cache is not the pinned Ergogen commit');
  if (git(['-C', cache, 'status', '--porcelain', '--untracked-files=all'])) throw new Error('Pinned checkout has tracked modifications or untracked files');
  const version = JSON.parse(fs.readFileSync(path.join(cache, 'package.json'), 'utf8')).version;
  if (version !== SOURCE.version) throw new Error('Pinned checkout package version does not match');
}

function generate(cache) {
  ensureCheckout(cache);
  const scenarios = require('./ergogen-parity/cases.cjs');
  const actual = JSON.parse(execFileSync(process.execPath,
    [path.join(__dirname, 'ergogen-parity/oracle.cjs'), cache], {
      encoding: 'utf8', input: JSON.stringify(scenarios.cases.map(item => item.ergogenConfig)),
      env: { ...process.env, NODE_PATH: path.join(__dirname, 'ergogen-parity/node_modules') },
      maxBuffer: 8 * 1024 * 1024,
    }));
  if (actual.length !== scenarios.cases.length) throw new Error('Oracle case count mismatch');
  const ids = new Set();
  const cases = scenarios.cases.map((scenario, index) => {
    if (ids.has(scenario.id)) throw new Error(`Duplicate case identity: ${scenario.id}`);
    ids.add(scenario.id);
    return { ...scenario, expected: collectExpected(scenario, actual[index]) };
  });
  const fixture = { fixtureVersion: 1, targetDocumentFormatVersion: 7, source: SOURCE,
    tolerances: { upstream: 1e-6, boardStudioOnly: 1e-9 },
    cases, boardStudioOnly: scenarios.boardStudioOnly };
  validateFixture(fixture);
  return fixture;
}

function writeFixture(output, fixture) {
  fs.mkdirSync(path.dirname(output), { recursive: true });
  const directory = fs.mkdtempSync(path.join(path.dirname(output), '.ergogen-parity-'));
  const temporary = path.join(directory, 'fixture.json');
  try {
    fs.writeFileSync(temporary, fixture, { flag: 'wx' });
    fs.renameSync(temporary, output);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
}

function main() {
  const { values } = parseArgs({ options: {
    cache: { type: 'string', default: path.join(os.homedir(), '.cache/boardstudio/ergogen', SOURCE.commit) },
    output: { type: 'string', default: path.join(ROOT, 'core/tests/fixtures/ergogen-layout-parity.json') },
    check: { type: 'boolean', default: false }, help: { type: 'boolean', default: false },
  } });
  if (values.help) {
    console.log('node scripts/generate-ergogen-parity-fixtures.cjs [--cache EXTERNAL_CHECKOUT] [--output JSON] [--check]\n' +
      'Prepare once: npm ci --ignore-scripts --prefix scripts/ergogen-parity\n' +
      '--check regenerates in memory and refuses differences without writing.');
    return;
  }
  const cache = path.resolve(values.cache);
  const output = path.resolve(values.output);
  const destination = realDestination(output);
  if (destination === realDestination(path.join(ROOT, 'core/tests/fixtures/matrix-projections.json'))) {
    throw new Error('Refusing to overwrite the current projection oracle before its replacement is wired');
  }
  const fromCache = path.relative(realDestination(cache), destination);
  if (!fromCache || (fromCache !== '..' && !fromCache.startsWith(`..${path.sep}`) && !path.isAbsolute(fromCache))) {
    throw new Error('Fixture output must not modify the pinned source checkout');
  }
  const fixture = JSON.stringify(generate(cache), null, 2) + '\n';
  if (values.check) {
    if (fs.readFileSync(output, 'utf8') !== fixture) throw new Error(`Fixture differs: ${output}`);
    console.log(`Fixture is reproducible: ${output}`);
    return;
  }
  writeFixture(output, fixture);
  console.log(`Wrote ${output}`);
}

module.exports = { collectExpected, validateFixture, ensureCheckout, writeFixture, generate, SOURCE };
if (require.main === module) {
  try { main(); } catch (error) {
    console.error(`Ergogen fixture generation failed: ${error.message}`);
    process.exitCode = 1;
  }
}
