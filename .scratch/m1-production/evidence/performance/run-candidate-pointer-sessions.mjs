import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const driver = fileURLToPath(new URL('./measure-candidate-pointer.mjs', import.meta.url));
const sessionCount = Number(process.env.BOARDSTUDIO_POINTER_SESSIONS ?? 5);
const runId = process.env.BOARDSTUDIO_RUN_ID ?? new Date().toISOString().replaceAll(':', '-');
const runRoot = resolve(process.env.BOARDSTUDIO_PERF_OUTPUT ?? '.scratch/m1-production/evidence/performance/runs');
const outputDir = resolve(runRoot, runId);
const wrapperSha256 = createHash('sha256').update(await readFile(new URL(import.meta.url))).digest('hex');
const thresholds = { 30: 33, 100: 50, 200: 100 };
assert.equal(Number.isInteger(sessionCount) && sessionCount > 0, true, 'session count must be a positive integer');
assert.ok(process.env.BOARDSTUDIO_CANDIDATE_URL, 'provide the exact fresh candidate release URL');
assert.ok(process.env.BOARDSTUDIO_RELEASE_RECORD, 'provide the exact source/build/assets release record');
const paired = Boolean(process.env.BOARDSTUDIO_REFERENCE_URL || process.env.BOARDSTUDIO_REFERENCE_RECORD);
if (paired) {
  assert.ok(process.env.BOARDSTUDIO_REFERENCE_URL, 'paired runs require the exact unchanged reference URL');
  assert.ok(process.env.BOARDSTUDIO_REFERENCE_RECORD, 'paired runs require the exact prebuilt reference asset record');
}

await mkdir(outputDir, { recursive: true });
const summary = {
  runId,
  mode: paired ? 'paired-candidate-reference-native-pointer-to-painted-transform' : 'candidate-native-pointer-to-painted-transform',
  comparator: paired ? {
    eligible: false,
    eligibility: 'pending matched session and archive validation',
    endpoint: 'native pointermove receivedAt → first requestAnimationFrame opportunity after MutationObserver observes the same archived target part transform change',
    operations: 'same generated .boardstudio archive, same single-part drag, same on-screen ±50px movement, 10 warmups, 100 measured samples',
    viewport: { width: 1280, height: 720, deviceScaleFactor: 1 },
    relativeThreshold: null,
    relativeThresholdMeaning: 'No paired relative pass/fail allowance is defined; report the per-session differences as observations only. Existing per-size absolute caps remain unchanged.',
  } : null,
  sessionsRequested: sessionCount,
  candidateUrl: process.env.BOARDSTUDIO_CANDIDATE_URL,
  driver,
  wrapperSha256,
  node: process.version,
  startedAt: new Date().toISOString(),
  invocations: [],
  sessions: [],
  scenarios: {},
  verdict: 'incomplete',
};

const percentile = (values, fraction) => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.ceil(sorted.length * fraction) - 1];
};

async function runVariant(sessionNumber, variant) {
  const sessionRunId = `session-${sessionNumber}-${variant}`;
  const argv = [process.execPath, driver];
  const startedAt = new Date().toISOString();
  const child = spawnSync(argv[0], argv.slice(1), {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    env: {
      ...process.env,
      BOARDSTUDIO_PERF_OUTPUT: outputDir,
      BOARDSTUDIO_RUN_ID: sessionRunId,
      BOARDSTUDIO_SESSION_NUMBER: String(sessionNumber),
      BOARDSTUDIO_POINTER_VARIANT: variant,
      BOARDSTUDIO_BROWSER_SESSION_PREFIX: `ticket06-pointer-${runId}-${sessionNumber}-${variant}`,
    },
  });
  const invocation = {
    variant,
    argv,
    cwd: root,
    startedAt,
    finishedAt: new Date().toISOString(),
    status: child.status,
    signal: child.signal,
    stdout: child.stdout ?? '',
    stderr: child.stderr ?? '',
    error: child.error?.message,
    output: resolve(outputDir, sessionRunId, 'run.json'),
  };
  summary.invocations.push(invocation);
  try {
    return JSON.parse(await readFile(invocation.output, 'utf8'));
  } catch (error) {
    return { variant, sessionNumber, status: 'run-record-unavailable', error: error instanceof Error ? error.message : String(error) };
  }
}

for (let sessionNumber = 1; sessionNumber <= sessionCount; sessionNumber += 1) {
  const order = paired && sessionNumber % 2 === 0 ? ['reference', 'candidate'] : paired ? ['candidate', 'reference'] : ['candidate'];
  const runs = {};
  for (const variant of order) runs[variant] = await runVariant(sessionNumber, variant);
  summary.sessions.push({ sessionNumber, order, ...runs });
}

for (const variant of paired ? ['candidate', 'reference'] : ['candidate']) for (const keys of [30, 100, 200]) {
  const sessions = summary.sessions.map((item) => item[variant]).filter((item) => Array.isArray(item?.scenarios)).map((item) => item.scenarios.find((scenario) => scenario.keys === keys)).filter(Boolean);
  if (sessions.length !== sessionCount) continue;
  (summary.scenarios[variant] ??= {})[keys] = {
    sessions: sessions.length,
    samplesPerSession: sessions.map((scenario) => scenario.measuredSamples),
    sessionP95Ms: sessions.map((scenario) => scenario.p95Ms),
    medianSessionP95Ms: percentile(sessions.map((scenario) => scenario.p95Ms), 0.5),
    existingAbsoluteLimitMs: thresholds[keys],
    everySessionWithinExistingLimit: sessions.every((scenario) => scenario.p95Ms <= thresholds[keys]),
    visibleTransformAssertionPassed: sessions.every((scenario) => scenario.changedFrames > 50),
  };
}

if (paired) {
  summary.pairedComparisons = {};
  for (const keys of [30, 100, 200]) {
    const rows = summary.sessions.map(({ candidate, reference, sessionNumber }) => {
      const candidateScenario = candidate?.scenarios?.find((scenario) => scenario.keys === keys);
      const referenceScenario = reference?.scenarios?.find((scenario) => scenario.keys === keys);
      const sameFixture = candidate?.fixtureManifest?.fixtures?.find((item) => item.keys === keys)?.archiveSha256
        === reference?.fixtureManifest?.fixtures?.find((item) => item.keys === keys)?.archiveSha256;
      const sameTarget = candidateScenario?.targetPartId === referenceScenario?.targetPartId
        && candidateScenario?.targetReference === referenceScenario?.targetReference;
      const sameBrowserEnvironment = candidate?.browserUserAgent === reference?.browserUserAgent
        && candidate?.gpuRenderer === reference?.gpuRenderer
        && JSON.stringify(candidate?.viewport) === JSON.stringify(reference?.viewport)
        && candidate?.node === reference?.node
        && JSON.stringify(candidate?.os) === JSON.stringify(reference?.os)
        && candidate?.cpu === reference?.cpu;
      const completeSamples = candidateScenario?.measuredSamples === 100 && referenceScenario?.measuredSamples === 100;
      return {
        sessionNumber,
        sameArchiveBytes: sameFixture,
        sameTarget,
        sameBrowserEnvironment,
        completeSamples,
        candidateP95Ms: candidateScenario?.p95Ms ?? null,
        referenceP95Ms: referenceScenario?.p95Ms ?? null,
        referenceMinusCandidateP95Ms: candidateScenario && referenceScenario ? referenceScenario.p95Ms - candidateScenario.p95Ms : null,
        relativePassFail: null,
      };
    });
    summary.pairedComparisons[keys] = { samplesPerSession: 100, warmupsPerSession: 10, sessions: rows,
      eligibleObservations: rows.length === sessionCount && rows.every((row) => row.sameArchiveBytes && row.sameTarget && row.sameBrowserEnvironment && row.completeSamples && row.referenceMinusCandidateP95Ms !== null),
      interpretation: 'Matched-endpoint observations only; there is no invented relative pass/fail threshold.' };
  }
  const allObservationsEligible = [30, 100, 200].every((keys) => summary.pairedComparisons[keys].eligibleObservations);
  summary.comparator.eligible = allObservationsEligible;
  summary.comparator.eligibility = allObservationsEligible
    ? 'same archive bytes, target, browser environment, and complete samples verified for every paired size/session'
    : 'one or more paired size/session records failed archive, target, environment, or sample identity checks';
}

summary.finishedAt = new Date().toISOString();
const variants = paired ? ['candidate', 'reference'] : ['candidate'];
summary.verdict = summary.invocations.every((item) => item.status === 0)
  && variants.every((variant) => [30, 100, 200].every((keys) => summary.scenarios[variant]?.[keys]?.everySessionWithinExistingLimit
    && summary.scenarios[variant][keys].samplesPerSession.every((count) => count === 100)
    && summary.scenarios[variant][keys].visibleTransformAssertionPassed))
  ? paired ? 'paired-observations-complete-all-sizes-within-existing-absolute-limits-no-relative-verdict' : 'five-session-absolute-budgets-pass'
  : 'incomplete-or-failed-runs-retained';
await writeFile(resolve(outputDir, 'summary.json'), `${JSON.stringify(summary, null, 2)}\n`);
console.log(`Candidate pointer sessions ${summary.verdict}: ${resolve(outputDir, 'summary.json')}`);
if (!['five-session-absolute-budgets-pass', 'paired-observations-complete-all-sizes-within-existing-absolute-limits-no-relative-verdict'].includes(summary.verdict)) process.exitCode = 1;
