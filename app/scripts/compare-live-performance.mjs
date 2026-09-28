import { readFile, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

const names = ['numeric', 'undo', 'gaskets', 'mounts'];
const finiteTime = value => Number.isFinite(value) && value >= 0;
const identity = value => JSON.stringify([value?.cpu, value?.chromium, value?.viewport,
  value?.gpu?.devices, value?.cpuRate]);

export function compareLive(candidate, reference) {
  const valid = report => report?.sessions >= 5 && report.provenance?.length === report.sessions
    && report.provenance.every(value => value.cpu && value.chromium && value.viewport
      && value.gpu?.devices?.length && value.cadWasmSha256 && value.cpuRate === 1
      && identity(value) === identity(report.provenance[0]))
    && names.every(name => {
      const row = report.scenarios?.[name];
      return row?.samples >= report.sessions * 5 && finiteTime(row.p50) && finiteTime(row.p95)
        && row.p95 >= row.p50 && row.sessionP95?.length === report.sessions && row.sessionP95.every(finiteTime);
    });
  const comparable = Boolean(valid(candidate) && valid(reference)
    && identity(candidate.provenance[0]) === identity(reference.provenance[0]));
  const scenarios = Object.fromEntries(names.map(name => {
    const current = candidate?.scenarios?.[name]?.p95;
    const before = reference?.scenarios?.[name]?.p95;
    const limit = before + Math.max(4, before * 0.1);
    return [name, { beforeP95Ms: before, currentP95Ms: current, regressionLimitMs: limit,
      regressionPassed: comparable && current <= limit,
      exactPassed: comparable && current <= 200,
      improvementFraction: comparable && before > 0 ? (before - current) / before : null }];
  }));
  const regressionsPassed = comparable && Object.values(scenarios).every(row => row.regressionPassed);
  return { comparable, regressionsPassed,
    improvementPassed: regressionsPassed && ['numeric', 'gaskets'].some(name => scenarios[name].improvementFraction >= 0.1),
    exactTargetPassed: comparable && Object.values(scenarios).every(row => row.exactPassed), scenarios };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [candidatePath, referencePath, outputPath] = process.argv.slice(2);
  if (!candidatePath || !referencePath) throw new Error('Usage: compare-live-performance.mjs candidate.json reference.json [output.json] [--require-exact]');
  const result = compareLive(JSON.parse(await readFile(candidatePath)), JSON.parse(await readFile(referencePath)));
  if (outputPath && !outputPath.startsWith('--')) await writeFile(outputPath, JSON.stringify(result, null, 2));
  console.info(JSON.stringify(result, null, 2));
  if (!result.regressionsPassed || !result.improvementPassed
    || (process.argv.includes('--require-exact') && !result.exactTargetPassed)) process.exitCode = 1;
}
