import { readFile, readdir, writeFile } from 'node:fs/promises';
import { gunzipSync } from 'node:zlib';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = resolve(process.argv[2] ?? 'cad/bench/results/latest');
const frozen = JSON.parse(await readFile(new URL('./results/baseline/summary.json', import.meta.url)));
const referenceDirectory = process.argv[3] ? resolve(process.argv[3]) : fileURLToPath(new URL('./results/baseline/', import.meta.url));
const reference = JSON.parse(await readFile(join(referenceDirectory, 'summary.json')));
const candidate = JSON.parse(await readFile(join(directory, 'summary.json')));
if (candidate.metadata.acceptanceEligible === false) throw new Error('Diagnostic reports cannot be used for acceptance');
const budgets = JSON.parse(await readFile(new URL('./budgets.json', import.meta.url)));
const mean = values => values.reduce((sum, value) => sum + value, 0) / values.length;
async function details(directory) {
  const groups = {};
  for (const name of (await readdir(directory)).filter(name => /^session-\d+\.json\.gz$/.test(name))) {
    const session = JSON.parse(gunzipSync(await readFile(join(directory, name))));
    for (const sample of session.samples) {
      const key = `${sample.fixture}/${sample.scenario}`;
      const result = { copiedMeshBytes: 0, transferredBytes: 0, stages: {}, counters: {}, wasmPeakBytes: null, completedMs: sample.completedMs, workerExecutionMs: 0 };
      for (const entry of sample.entries.filter(entry => entry.name === 'boardstudio.cad.request')) {
        result.copiedMeshBytes += entry.detail.counters.copiedMeshBytes ?? 0;
        result.transferredBytes += entry.detail.transferredBytes;
        result.workerExecutionMs += entry.detail.workerExecutionMs;
        if (entry.detail.wasmAllocatedBytes.peak !== null) result.wasmPeakBytes = Math.max(result.wasmPeakBytes ?? 0, entry.detail.wasmAllocatedBytes.peak);
        for (const [name, count] of Object.entries(entry.detail.counters)) result.counters[name] = (result.counters[name] ?? 0) + count;
        for (const [name, stage] of Object.entries(entry.detail.stages)) result.stages[name] = (result.stages[name] ?? 0) + stage.durationMs;
      }
      (groups[key] ??= []).push(result);
    }
  }
  return Object.fromEntries(Object.entries(groups).map(([key, samples]) => [key, {
    sampleCount: samples.length,
    completedMeanMs: mean(samples.map(sample => sample.completedMs)),
    workerExecutionMeanMs: mean(samples.map(sample => sample.workerExecutionMs)),
    copiedMeshBytes: mean(samples.map(sample => sample.copiedMeshBytes)),
    transferredBytes: mean(samples.map(sample => sample.transferredBytes)),
    wasmPeakBytes: samples.some(sample => sample.wasmPeakBytes !== null) ? Math.max(...samples.map(sample => sample.wasmPeakBytes ?? 0)) : null,
    countersMean: Object.fromEntries([...new Set(samples.flatMap(sample => Object.keys(sample.counters)))].map(name => [name, mean(samples.map(sample => sample.counters[name] ?? 0))])),
    stagesMeanMs: Object.fromEntries([...new Set(samples.flatMap(sample => Object.keys(sample.stages)))].map(name => [name, mean(samples.map(sample => sample.stages[name] ?? 0))])),
  }]));
}
const beforeDetails = await details(referenceDirectory);
const afterDetails = await details(directory);
const rows = {};
for (const [key, after] of Object.entries(candidate.summary)) {
  const before = reference.summary[key];
  const budget = budgets.scenarios[key];
  if (beforeDetails[key]?.sampleCount !== before.samples || afterDetails[key]?.sampleCount !== after.samples) {
    throw new Error(`Incomplete or mixed raw reports for ${key}; use a fresh output directory`);
  }
  rows[key] = { before, after, budget,
    completedP95Pass: after.completedMs.p95 <= budget.completedP95Ms,
    paintedP95Pass: after.paintedMs.p95 <= budget.paintedP95Ms,
    rssPass: after.sampledPeakRssBytes <= budget.peakRssBytes,
    beforeDetails: beforeDetails[key], afterDetails: afterDetails[key],
  };
}
const completeScenarioSet = Object.keys(reference.summary).every(key => key in rows);
// Browser provenance is recorded for interpretation, but the harness does not
// pin or reject a particular Chrome build. CPU, OS, and app/WASM identity still
// define whether timing comparisons are eligible.
const browserVersionMatch = reference.metadata.browserVersion === candidate.metadata.browserVersion;
const sufficientSampling = [frozen, reference].every(required => candidate.metadata.sessions >= required.metadata.sessions
  && candidate.metadata.warmSamplesPerSession >= required.metadata.warmSamplesPerSession
  && Object.entries(required.summary).every(([key, before]) => (candidate.summary[key]?.samples ?? 0) >= before.samples));
const matchingFixtures = [...frozen.metadata.fixtures, ...reference.metadata.fixtures].every(fixture =>
  candidate.metadata.fixtures.some(value => value.id === fixture.id && value.sha256 === fixture.sha256));
const report = { referenceDirectory, completeScenarioSet, sufficientSampling, matchingFixtures, browserVersionMatch,
  comparableEnvironment: ['cpu', 'os', 'rendererWasmSha256', 'coreWasmSha256'].every(key => reference.metadata[key] === candidate.metadata[key]), rows };
await writeFile(join(directory, 'comparison.json'), JSON.stringify(report, null, 2));
for (const [key, row] of Object.entries(rows)) console.log(`${key}: p95 ${row.before.completedMs.p95.toFixed(1)} -> ${row.after.completedMs.p95.toFixed(1)} ms; completion ${row.completedP95Pass}, paint ${row.paintedP95Pass}, RSS ${row.rssPass}; copy ${Math.round(row.afterDetails.copiedMeshBytes)} / ${Math.round(row.beforeDetails.copiedMeshBytes)} B; transfer ${Math.round(row.afterDetails.transferredBytes)} / ${Math.round(row.beforeDetails.transferredBytes)} B`);
if (!report.sufficientSampling || !report.matchingFixtures || !report.completeScenarioSet || !report.comparableEnvironment || Object.values(rows).some(row => !row.completedP95Pass || !row.paintedP95Pass || !row.rssPass)) process.exitCode = 1;
