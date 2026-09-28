import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { compareLive } from '../../app/scripts/compare-live-performance.mjs';

const report = () => ({
  sessions: 5,
  provenance: Array.from({ length: 5 }, () => ({ cpu: 'reference', chromium: '153', cpuRate: 1,
    viewport: { width: 1280, height: 720 }, cadWasmSha256: 'artifact',
    gpu: { devices: [{ deviceString: 'hardware' }] } })),
  scenarios: Object.fromEntries(['numeric', 'undo', 'gaskets', 'mounts'].map(name => [name,
    { samples: 25, p50: 100, p95: 100, sessionP95: [100, 100, 100, 100, 100] }])),
});

test('numeric exact latency cannot be masked by fast gestures', () => {
  const reference = report(), candidate = report();
  reference.scenarios.numeric.p95 = 300;
  candidate.scenarios.numeric.p95 = 250;
  const result = compareLive(candidate, reference);
  assert.equal(result.regressionsPassed, true);
  assert.equal(result.improvementPassed, true);
  assert.equal(result.exactTargetPassed, false);
  assert.equal(result.scenarios.numeric.exactPassed, false);
});

test('comparison rejects regressions, insufficient samples and invalid timing', () => {
  for (const change of [
    value => { value.scenarios.undo.p95 = 120; },
    value => { value.sessions = 1; },
    value => { value.scenarios.gaskets.samples = 5; },
    value => { value.scenarios.numeric.p95 = NaN; },
    value => { delete value.scenarios.numeric; },
    value => { value.provenance[0].gpu.devices[0].deviceString = 'software'; },
    value => { value.provenance[0].cpuRate = 4; },
  ]) {
    const candidate = report(); change(candidate);
    assert.equal(compareLive(candidate, report()).regressionsPassed, false);
  }
  const reference = report(); reference.sessions = 1;
  assert.equal(compareLive(report(), reference).regressionsPassed, false);
});

test('unchanged timings pass regression budgets without claiming an improvement', () => {
  const result = compareLive(report(), report());
  assert.equal(result.regressionsPassed, true);
  assert.equal(result.improvementPassed, false);
  assert.equal(result.exactTargetPassed, true);
});

test('CLI enforces incremental acceptance and optionally the exact target', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'live-comparison-'));
  try {
    const reference = report(), candidate = report();
    reference.scenarios.numeric.p95 = 300; candidate.scenarios.numeric.p95 = 250;
    const before = join(directory, 'before.json'), after = join(directory, 'after.json');
    await writeFile(before, JSON.stringify(reference)); await writeFile(after, JSON.stringify(candidate));
    const command = fileURLToPath(new URL('../../app/scripts/compare-live-performance.mjs', import.meta.url));
    const run = (...args) => spawnSync(process.execPath, [command, after, before, ...args], { encoding: 'utf8' });
    assert.equal(run().status, 0);
    assert.equal(run('--require-exact').status, 1);
    candidate.scenarios.undo.p95 = 130;
    await writeFile(after, JSON.stringify(candidate));
    assert.equal(run().status, 1);
  } finally { await rm(directory, { recursive: true, force: true }); }
});
