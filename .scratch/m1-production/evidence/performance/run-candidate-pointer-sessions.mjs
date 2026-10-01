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

await mkdir(outputDir, { recursive: true });
const summary = {
  runId,
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

for (let sessionNumber = 1; sessionNumber <= sessionCount; sessionNumber += 1) {
  const sessionRunId = `session-${sessionNumber}`;
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
      BOARDSTUDIO_BROWSER_SESSION_PREFIX: `ticket06-pointer-${runId}-${sessionNumber}`,
    },
  });
  const invocation = {
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
    summary.sessions.push(JSON.parse(await readFile(invocation.output, 'utf8')));
  } catch (error) {
    summary.sessions.push({ sessionNumber, status: 'run-record-unavailable', error: error instanceof Error ? error.message : String(error) });
  }
}

for (const keys of [30, 100, 200]) {
  const sessions = summary.sessions.filter((item) => Array.isArray(item.scenarios)).map((item) => item.scenarios.find((scenario) => scenario.keys === keys)).filter(Boolean);
  if (sessions.length !== sessionCount) continue;
  summary.scenarios[keys] = {
    sessions: sessions.length,
    samplesPerSession: sessions.map((scenario) => scenario.measuredSamples),
    sessionP95Ms: sessions.map((scenario) => scenario.p95Ms),
    medianSessionP95Ms: percentile(sessions.map((scenario) => scenario.p95Ms), 0.5),
    existingAbsoluteLimitMs: thresholds[keys],
    everySessionWithinExistingLimit: sessions.every((scenario) => scenario.p95Ms <= thresholds[keys]),
    visibleTransformAssertionPassed: sessions.every((scenario) => scenario.changedFrames > 50),
  };
}

summary.finishedAt = new Date().toISOString();
summary.verdict = summary.invocations.every((item) => item.status === 0)
  && Object.keys(summary.scenarios).length === 3
  && Object.values(summary.scenarios).every((scenario) => scenario.everySessionWithinExistingLimit && scenario.samplesPerSession.every((count) => count === 100) && scenario.visibleTransformAssertionPassed)
  ? 'five-session-absolute-budgets-pass'
  : 'incomplete-or-failed-runs-retained';
await writeFile(resolve(outputDir, 'summary.json'), `${JSON.stringify(summary, null, 2)}\n`);
console.log(`Candidate pointer sessions ${summary.verdict}: ${resolve(outputDir, 'summary.json')}`);
if (summary.verdict !== 'five-session-absolute-budgets-pass') process.exitCode = 1;
