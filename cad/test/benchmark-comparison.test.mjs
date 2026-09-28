import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFile, mkdtemp, writeFile, rm, access } from 'node:fs/promises';
import { gzipSync, gunzipSync } from 'node:zlib';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

test('CAD acceptance rejects diagnostic instrumentation even with baseline sampling', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'cad-diagnostic-'));
  try {
    const candidate = JSON.parse(await readFile(new URL('../bench/results/baseline/summary.json', import.meta.url)));
    candidate.metadata.acceptanceEligible = false;
    candidate.metadata.hostObservations = true;
    await writeFile(join(directory, 'summary.json'), JSON.stringify(candidate));
    const run = spawnSync(process.execPath, [fileURLToPath(new URL('../bench/compare.mjs', import.meta.url)), directory], { encoding: 'utf8' });
    assert.equal(run.status, 1);
    assert.match(run.stderr, /cannot be used for acceptance/);
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('paired export diagnostics reject incompatible bindings before creating a report', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'cad-bindings-'));
  const output = join(directory, 'report');
  try {
    await writeFile(join(directory, 'boardstudio_cadrum_wasm.js'), '// deliberately incompatible control bindings');
    const run = spawnSync(process.execPath, [fileURLToPath(new URL('../../app/scripts/diagnose-cad-exports.mjs', import.meta.url)), directory, output], { encoding: 'utf8' });
    assert.equal(run.status, 1);
    assert.match(run.stderr, /binary-only comparison is unsafe/);
    await assert.rejects(access(output), { code: 'ENOENT' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('CAD acceptance rejects a complete but undersampled smoke run', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'cad-sampling-'));
  try {
    const reference = JSON.parse(await readFile(new URL('../bench/results/baseline/summary.json', import.meta.url)));
    const session = JSON.parse(gunzipSync(await readFile(new URL('../bench/results/baseline/session-1.json.gz', import.meta.url))));
    const samples = [...new Map(session.samples.map(sample => [`${sample.fixture}/${sample.scenario}`, sample])).values()];
    reference.metadata.sessions = 1;
    reference.metadata.warmSamplesPerSession = 1;
    for (const row of Object.values(reference.summary)) row.samples = 1;
    await writeFile(join(directory, 'summary.json'), JSON.stringify(reference));
    await writeFile(join(directory, 'session-1.json.gz'), gzipSync(JSON.stringify({ ...session, samples })));
    const run = spawnSync(process.execPath, [fileURLToPath(new URL('../bench/compare.mjs', import.meta.url)), directory], { encoding: 'utf8' });
    assert.equal(run.status, 1, run.stderr || run.stdout);
    const comparison = JSON.parse(await readFile(join(directory, 'comparison.json')));
    assert.equal(comparison.sufficientSampling, false);
    const refreshed = spawnSync(process.execPath, [fileURLToPath(new URL('../bench/compare.mjs', import.meta.url)), directory, directory], { encoding: 'utf8' });
    assert.equal(refreshed.status, 1, 'a new reference cannot lower frozen sample requirements');
    const withReference = JSON.parse(await readFile(join(directory, 'comparison.json')));
    assert.equal(withReference.referenceDirectory, directory);
    assert.equal(withReference.sufficientSampling, false);
  } finally { await rm(directory, { recursive: true, force: true }); }
});
