import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cpus, platform, release as osRelease, arch } from 'node:os';
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const fixtureDir = resolve(process.env.BOARDSTUDIO_FIXTURE_DIR ?? '.scratch/m1-production/evidence/performance/fixtures');
const candidateUrl = process.env.BOARDSTUDIO_CANDIDATE_URL;
const releaseRecordPath = process.env.BOARDSTUDIO_RELEASE_RECORD;
const runRoot = resolve(process.env.BOARDSTUDIO_PERF_OUTPUT ?? '.scratch/m1-production/evidence/performance/runs');
const runId = process.env.BOARDSTUDIO_RUN_ID ?? new Date().toISOString().replaceAll(':', '-');
const runDir = resolve(runRoot, runId);
const thresholds = { 30: 33, 100: 50, 200: 100 };
const warmups = 10;
const measured = 100;
const movePixels = 50;

if (!candidateUrl || !releaseRecordPath) {
  throw new Error('Set BOARDSTUDIO_CANDIDATE_URL and BOARDSTUDIO_RELEASE_RECORD for the exact fresh candidate; this script does not build or guess a release.');
}
const releaseRecord = JSON.parse(await readFile(releaseRecordPath, 'utf8'));
assert.ok(releaseRecord.sourceCommit, 'release record must identify the candidate source commit');
assert.ok(releaseRecord.assetHashes, 'release record must contain final built asset hashes');
const fixtureManifest = JSON.parse(await readFile(resolve(fixtureDir, 'manifest.json'), 'utf8'));
assert.equal(fixtureManifest.fixtures.length, 3, 'pointer archives must be generated and validated before browser runs');

await mkdir(runDir, { recursive: true });
let session;

const run = {
  runId,
  sessionNumber: Number(process.env.BOARDSTUDIO_SESSION_NUMBER ?? 1),
  sessionId: null,
  driverSha256: createHash('sha256').update(await readFile(new URL(import.meta.url))).digest('hex'),
  fixtureGeneratorSha256: fixtureManifest.generatorSha256,
  sourceCommit: releaseRecord.sourceCommit,
  releaseRecord,
  candidateUrl,
  fixtureManifest,
  tool: 'agent-browser CLI; visible UI import and native browser mouse input',
  node: process.version,
  os: { platform: platform(), release: osRelease(), arch: arch() },
  cpu: cpus()[0]?.model ?? 'unknown',
  browserUserAgent: null,
  gpuRenderer: null,
  viewport: null,
  startedAt: new Date().toISOString(),
  commands: [],
  sessions: [],
  scenarios: [],
  failures: [],
  verdict: 'incomplete',
};

function browser(args, input) {
  assert.ok(session, 'an isolated browser session must be active');
  const argv = ['agent-browser', '--session', session, '--json', ...args];
  const startedAt = new Date().toISOString();
  const result = spawnSync(argv[0], argv.slice(1), {
    cwd: root,
    encoding: 'utf8',
    input,
    maxBuffer: 8 * 1024 * 1024,
  });
  run.commands.push({
    argv,
    cwd: root,
    startedAt,
    finishedAt: new Date().toISOString(),
    status: result.status,
    stdin: input,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
    error: result.error?.message,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`agent-browser ${args.join(' ')} failed (${result.status}): ${result.stderr || result.stdout}`);
  const envelope = JSON.parse(result.stdout.trim());
  if (!envelope.success) throw new Error(`agent-browser ${args.join(' ')} failed: ${JSON.stringify(envelope.error)}`);
  return envelope.data;
}

function startBrowserSession() {
  const argv = ['agent-browser', 'session', 'id', '--scope', 'worktree', '--prefix', process.env.BOARDSTUDIO_BROWSER_SESSION_PREFIX ?? `ticket06-pointer-${runId}`];
  const startedAt = new Date().toISOString();
  const result = spawnSync(argv[0], argv.slice(1), { cwd: root, encoding: 'utf8' });
  run.commands.push({ argv, cwd: root, startedAt, finishedAt: new Date().toISOString(), status: result.status, stdout: result.stdout ?? '', stderr: result.stderr ?? '', error: result.error?.message });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`agent-browser session creation failed: ${result.stderr || result.stdout}`);
  session = result.stdout.trim();
  assert.ok(session, 'agent-browser must return an isolated named session');
  run.sessionId = session;
}

function evalPage(script) {
  return browser(['eval', '--stdin'], script).result;
}

const percentile = (samples, fraction) => {
  const sorted = [...samples].sort((a, b) => a - b);
  return sorted[Math.ceil(sorted.length * fraction) - 1];
};

try {
  startBrowserSession();
  browser(['open', candidateUrl]);
  browser(['set', 'viewport', '1280', '720']);
  const environment = evalPage(`(() => {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl');
    const debug = gl?.getExtension('WEBGL_debug_renderer_info');
    return { userAgent: navigator.userAgent, width: innerWidth, height: innerHeight,
      dpr: devicePixelRatio, gpu: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : 'unavailable',
      url: location.href };
  })()`);
  run.browserUserAgent = environment.userAgent;
  run.gpuRenderer = environment.gpu;
  run.viewport = { width: environment.width, height: environment.height, deviceScaleFactor: environment.dpr };
  assert.equal(environment.width, 1280);
  assert.equal(environment.height, 720);
  assert.equal(environment.dpr, 1, 'candidate pointer comparison requires DPR 1');

  for (const keys of [30, 100, 200]) {
    const fixture = fixtureManifest.fixtures.find((item) => item.keys === keys);
    assert.ok(fixture, `missing ${keys}-key archive`);
    const archive = resolve(fixtureDir, fixture.archive);
    const archiveBytes = await readFile(archive);
    assert.equal(createHash('sha256').update(archiveBytes).digest('hex'), fixture.archiveSha256, `${keys}-key archive hash must match the generation manifest`);

    browser(['open', candidateUrl]);
    browser(['upload', 'input[type="file"][accept=".boardstudio"]', archive]);
    const ready = evalPage(`(() => Boolean(document.querySelector('section.m1-editor svg.m1-canvas')))()`);
    if (!ready) {
      browser(['wait', 'section.m1-editor svg.m1-canvas']);
    }

    const targetCoordinates = evalPage(`(() => {
      const keyCount = ${keys};
      const columns = keyCount === 30 ? 5 : keyCount === 100 ? 10 : 20;
      const rows = keyCount / columns;
      const row = Math.floor(rows / 2);
      const column = Math.floor(columns / 2);
      const id = 'key-' + row + '-' + column;
      const target = document.querySelector('[data-part-id="' + id + '"]');
      if (!target) throw new Error('Imported project has no target part ' + id);
      const rect = target.getBoundingClientRect();
      const x = rect.left + rect.width / 2;
      const y = rect.top + rect.height / 2;
      const hit = document.elementFromPoint(x, y)?.closest('[data-part-id]');
      if (hit !== target) throw new Error('Target part is obscured or not hit-testable: ' + id);
      return { id, x, y, transform: target.getAttribute('transform'), partCount: document.querySelectorAll('[data-part-id]').length };
    })()`);
    assert.equal(targetCoordinates.partCount, keys * 3, `${keys}-key archive must render all expected parts`);

    evalPage(`(() => {
      const target = document.querySelector('[data-part-id="${targetCoordinates.id}"]');
      const observer = { target, previous: target.getAttribute('transform'), armed: false,
        pending: null, results: [], missed: 0 };
      document.addEventListener('pointermove', (event) => {
        if (!observer.armed) return;
        if (observer.pending) observer.missed += 1;
        observer.armed = false;
        observer.pending = { eventTimeStamp: event.timeStamp, inputAt: performance.now(), before: target.getAttribute('transform') };
      }, true);
      const mutations = new MutationObserver(() => {
        const sample = observer.pending;
        if (!sample || sample.scheduled || target.getAttribute('transform') === sample.before) return;
        sample.scheduled = true;
        sample.domChangedAt = performance.now();
        sample.afterMutation = target.getAttribute('transform');
        requestAnimationFrame(() => {
          sample.rafAt = performance.now();
          sample.after = target.getAttribute('transform');
          observer.results.push(sample);
          observer.pending = null;
        });
      });
      mutations.observe(target, { attributes: true, attributeFilter: ['transform'] });
      window.__ticket06PointerObservation = observer;
      return true;
    })()`);

    const x = Math.round(targetCoordinates.x);
    const y = Math.round(targetCoordinates.y);
    const count = warmups + measured;
    const pointerCommands = [
      ['mouse', 'move', String(x), String(y)],
      ['mouse', 'down', 'left'],
    ];
    for (let index = 0; index < count; index += 1) {
      const nextX = Math.round(targetCoordinates.x + (index % 2 === 0 ? movePixels : -movePixels));
      pointerCommands.push(
        ['eval', 'window.__ticket06PointerObservation.armed = true; true'],
        ['mouse', 'move', String(nextX), String(y)],
        ['wait', '--fn', `window.__ticket06PointerObservation.results.length >= ${index + 1}`],
      );
    }
    pointerCommands.push(['mouse', 'up', 'left']);
    browser(['batch', '--bail', ...pointerCommands.map((command) => JSON.stringify(command))]);

    const records = evalPage('window.__ticket06PointerObservation.results');
    assert.equal(records.length, count, `${keys}-key fixture must yield one transform observation per native movement`);
    const samples = [];
    for (let index = 0; index < count; index += 1) {
      const record = records[index];
      const latencyMs = record.rafAt - record.inputAt;
      if (index >= warmups) samples.push({ index: index - warmups, eventTimeStamp: record.eventTimeStamp, inputAt: record.inputAt, domChangedAt: record.domChangedAt, rafAt: record.rafAt, latencyMs, before: record.before, afterMutation: record.afterMutation, after: record.after });
    }

    const observerSummary = evalPage(`(() => ({ missed: window.__ticket06PointerObservation.missed,
      transform: window.__ticket06PointerObservation.target.getAttribute('transform') }))()`);
    const result = {
      sessionNumber: run.sessionNumber,
      keys,
      partCount: targetCoordinates.partCount,
      targetPartId: targetCoordinates.id,
      warmups,
      measuredSamples: samples.length,
      samples,
      p50Ms: percentile(samples.map((sample) => sample.latencyMs), 0.5),
      p95Ms: percentile(samples.map((sample) => sample.latencyMs), 0.95),
      changedTransforms: samples.filter((sample) => sample.before !== sample.after).length,
      changedFrames: records.filter((sample) => sample.before !== sample.after).length,
      missedInputMarkers: observerSummary.missed,
      thresholdMs: thresholds[keys],
      withinExistingAbsoluteLimit: percentile(samples.map((sample) => sample.latencyMs), 0.95) <= thresholds[keys],
      observerEndpoint: 'first requestAnimationFrame opportunity after MutationObserver confirms the target SVG transform changed; not physical display presentation',
      inputEndpoint: 'native browser pointermove event received in capture phase; latency starts at performance.now() in that listener, and event.timeStamp is retained',
    };
    assert.equal(result.measuredSamples, measured);
    assert.ok(result.changedFrames > measured / 2, 'preserve the existing visible-transform assertion across warmup and measured movements');
    assert.ok(result.missedInputMarkers <= 1, 'pointer delivery produced overlapping, unmeasured moves');
    run.scenarios.push(result);

    await writeFile(resolve(runDir, `pointer-${keys}.json`), `${JSON.stringify(result, null, 2)}\n`);
    evalPage('window.__ticket06PointerObservation = undefined; true');
  }

  run.verdict = run.scenarios.every((scenario) => scenario.withinExistingAbsoluteLimit) ? 'absolute-budgets-pass' : 'absolute-budget-fail';
} catch (error) {
  run.failures.push({ at: new Date().toISOString(), message: error instanceof Error ? error.stack ?? error.message : String(error) });
  run.verdict = 'failed-run-retained';
  throw error;
} finally {
  if (session) {
    try {
      browser(['close']);
    } catch (error) {
      run.failures.push({ at: new Date().toISOString(), message: `Browser close failed: ${error instanceof Error ? error.message : String(error)}` });
      run.verdict = 'failed-run-retained';
    }
  }
  run.finishedAt = new Date().toISOString();
  run.sourceCommit = releaseRecord.sourceCommit;
  await writeFile(resolve(runDir, 'run.json'), `${JSON.stringify(run, null, 2)}\n`);
}

console.log(`Candidate pointer run ${run.verdict}: ${resolve(runDir, 'run.json')}`);
if (run.verdict !== 'absolute-budgets-pass') process.exitCode = 1;
