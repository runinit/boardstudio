import { chromium } from '@playwright/test';
import { spawn, execFileSync } from 'node:child_process';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { cpus, hostname, platform, release } from 'node:os';
import { fileURLToPath } from 'node:url';
import { resolve, join, dirname } from 'node:path';
import { gzipSync } from 'node:zlib';

const root = fileURLToPath(new URL('../../', import.meta.url));
const args = process.argv.slice(2);
const option = (name, fallback) => args.includes(name) ? args[args.indexOf(name) + 1] : fallback;
const sessions = Number(option('--sessions', '5'));
const samples = Number(option('--samples', '20'));
const coldSamples = Number(option('--cold-samples', '1'));
const profiled = option('--profile', 'on') === 'on';
const detailedMemory = args.includes('--detailed-memory');
const hostObservations = args.includes('--host-observations');
const diagnosticBusyHost = args.includes('--diagnostic-busy-host');
const rotateScenarios = args.includes('--rotate-scenarios');
const output = resolve(root, option('--output', 'cad/bench/results/latest'));
const browserPath = process.env.BOARDSTUDIO_CHROMIUM ?? '/usr/bin/chromium';
const port = Number(option('--port', '4341'));
const allowedScenarios = ['cold', 'warm-uncached', 'cache-hit', 'region-edit', 'preview-export', 'direct-export', 'cancel-retry', 'undo-redo', 'board-switch', 'edit-burst', 'export-edit', 'editing-soak'];
const scenarios = option('--scenarios', allowedScenarios.slice(0, 7).join(',')).split(',');
if (scenarios.some(value => !allowedScenarios.includes(value))) throw new Error('Unknown scenario');
function assertQuietHost() {
  const commands = execFileSync('ps', ['-eo', 'args='], { encoding: 'utf8' }).split('\n');
  const competing = commands.filter(command => !/^(?:\S*\/)?(?:bash|sh|zsh)\s/.test(command.trim()) && /(?:^|\/)(?:cargo|rustc|wasm-opt)(?:\s|$)|vitest(?:\.mjs)? run|playwright(?:[^ ]*\/cli\.js)? test/.test(command));
  if (competing.length && !diagnosticBusyHost) throw new Error(`Competing build/test processes prevent measurement:\n${competing.join('\n')}`);
  if (competing.length && diagnosticBusyHost) console.warn('Diagnostic-only run: competing work detected; results cannot establish acceptance.');
}
const manifest = JSON.parse(await readFile(resolve(root, option('--manifest', 'cad/bench/fixtures/manifest.json'))));
if (!Number.isInteger(coldSamples) || coldSamples < 1) throw new Error('Positive cold-samples required');
const selected = option('--fixtures', '').split(',').filter(Boolean);
if (selected.some(id => !manifest.fixtures.some(fixture => fixture.id === id))) throw new Error('Unknown fixture');
if (selected.length) manifest.fixtures = manifest.fixtures.filter(fixture => selected.includes(fixture.id));
if (!Number.isInteger(sessions) || sessions < 1 || !Number.isInteger(samples) || samples < 1) throw new Error('Positive sessions/samples required');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const fixtures = await Promise.all(manifest.fixtures.map(async fixture => {
  const bytes = await readFile(join(root, fixture.kind === 'step' ? fixture.file : `cad/bench/fixtures/${fixture.file}`));
  if (hash(bytes) !== fixture.sha256) throw new Error(`Fixture hash changed: ${fixture.id}`);
  return { ...fixture, input: fixture.kind === 'step' ? { step: [...bytes] } : { assembly: JSON.parse(bytes) } };
}));

// Read all threads' child lists: Chromium can spawn subprocesses off the main thread.
async function descendants(pid, excluded, seen = new Set()) {
  if (seen.has(pid) || excluded.has(pid)) return seen;
  seen.add(pid);
  try {
    const threads = await readdir(`/proc/${pid}/task`);
    for (const thread of threads) {
      let children;
      try { children = await readFile(`/proc/${pid}/task/${thread}/children`, 'utf8'); } catch { continue; }
      for (const child of children.trim().split(/\s+/).filter(Boolean)) await descendants(Number(child), excluded, seen);
    }
  } catch { /* A process may exit during the census. */ }
  return seen;
}

function rssSampler(excluded) {
  const rows = [];
  let label = 'browser-startup';
  let stopped = false;
  let pids = new Set();
  let censusAt = -Infinity;
  let detailAt = -Infinity;
  let hostAt = -Infinity;
  const started = performance.now();
  const task = (async () => {
    while (!stopped) {
      const now = performance.now();
      if (now - censusAt >= 100) { pids = await descendants(process.pid, excluded); pids.delete(process.pid); censusAt = now; }
      const processes = [];
      const readDetail = detailedMemory && now - detailAt >= 1000;
      if (readDetail) detailAt = now;
      for (const pid of pids) {
        try {
          const [status, stat] = await Promise.all([readFile(`/proc/${pid}/status`, 'utf8'), readFile(`/proc/${pid}/stat`, 'utf8')]);
          const rss = Number(status.match(/^VmRSS:\s+(\d+)/m)?.[1] ?? 0) * 1024;
          const statFields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
          const startTicks = statFields[19];
          const cpuTicks = hostObservations ? Number(statFields[11]) + Number(statFields[12]) : undefined;
          let pssBytes, privateBytes;
          if (readDetail) {
            try {
              const rollup = await readFile(`/proc/${pid}/smaps_rollup`, 'utf8');
              pssBytes = Number(rollup.match(/^Pss:\s+(\d+)/m)?.[1] ?? 0) * 1024;
              privateBytes = [...rollup.matchAll(/^Private_(?:Clean|Dirty|Hugetlb):\s+(\d+)/gm)].reduce((sum, match) => sum + Number(match[1]) * 1024, 0);
            } catch { /* Exited or inaccessible process: missing detail is not zero. */ }
          }
          processes.push({ pid, startTicks, rss, pssBytes, privateBytes, cpuTicks });
        } catch { /* A sample cannot include an exited process. */ }
      }
      let host;
      if (hostObservations && now - hostAt >= 1000) {
        hostAt = now;
        const [cpuPressure, memoryPressure, vmstat, frequencies] = await Promise.all([
          readFile('/proc/pressure/cpu', 'utf8'), readFile('/proc/pressure/memory', 'utf8'), readFile('/proc/vmstat', 'utf8'),
          Promise.all(cpus().map((_, index) => readFile(`/sys/devices/system/cpu/cpu${index}/cpufreq/scaling_cur_freq`, 'utf8').then(Number).catch(() => null))),
        ]);
        host = { cpuPressure: cpuPressure.trim(), memoryPressure: memoryPressure.trim(), frequencyKhz: frequencies,
          vmstat: Object.fromEntries(vmstat.trim().split('\n').filter(line => /^(pgmajfault|pswpin|pswpout|pgscan_kswapd|pgsteal_kswapd) /.test(line)).map(line => {
            const [name, value] = line.split(' ');
            return [name, Number(value)];
          })) };
      }
      rows.push({ ms: performance.now() - started, label, rssBytes: processes.reduce((sum, entry) => sum + entry.rss, 0), processes, host });
      await new Promise(resolve => setTimeout(resolve, Math.max(0, 10 - (performance.now() - now))));
    }
  })();
  return { label(value) { label = value; }, async stop() { stopped = true; await task; return rows; } };
}

const all = [];
async function sourceHashes(directory) {
  const files = {};
  for (const entry of (await readdir(join(root, directory), { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
    const path = `${directory}/${entry.name}`;
    if (entry.isDirectory()) Object.assign(files, await sourceHashes(path));
    else files[path] = hash(await readFile(join(root, path)));
  }
  return files;
}
const metadata = {
  schemaVersion: 1, revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  dirty: Boolean(execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).trim()),
  sourceDiffSha256: hash(execFileSync('git', ['diff', 'HEAD'], { cwd: root })),
  wasmSha256: hash(await readFile(join(root, 'cad/wasm/pkg/boardstudio_cadrum_wasm_bg.wasm'))),
  coreWasmSha256: hash(await readFile(join(root, 'core/pkg/boardstudio_core_bg.wasm'))),
  rendererWasmSha256: hash(await readFile(join(root, 'renderer/pkg/boardstudio_renderer_wasm_bg.wasm'))),
  sources: { ...await sourceHashes('cad/src'), ...await sourceHashes('cad/wasm/src'),
    ...await sourceHashes('app/src'), ...await sourceHashes('cad/bench/fixtures'),
    'app/scripts/run-cad-benchmark.mjs': hash(await readFile(new URL(import.meta.url))) },
  host: hostname(), cpu: cpus()[0].model, os: `${platform()} ${release()}`, browserPath,
  acceptanceEligible: !diagnosticBusyHost && !hostObservations, profiled, coldSamplesPerSession: coldSamples, detailedMemory, hostObservations, scenarios, rotateScenarios,
  kernel: { cadrum: '0.8.20', occt: '8.0.1-rev2' }, sessions, warmSamplesPerSession: samples,
  viewport: { width: 1280, height: 720 }, fixtures: manifest.fixtures,
  memoryMethod: 'sum of sampled descendant RSS; 10 ms target, 100 ms process census; shared pages may be double counted; driver/server excluded',
  limitations: ['Headless software-rendered canvas paint-opportunity proxy, not physical display latency', 'Prepared input mutation scenarios are deterministic; STEP input has no geometry edit/cache/export scenarios', 'Cold sample count is explicit; warm and cold distributions aggregate across sessions', 'Unprofiled runs report completion only; paint is unavailable', 'Interaction traces in the canvas exercise worker workloads; full UI coverage is separate'],
};

// Refuse stale distribution assets: the browser must execute the recorded binaries.
const assets = await readdir(join(root, 'app/dist/assets'));
metadata.appAssetSha256 = Object.fromEntries(await Promise.all(assets.filter(name => /\.(js|wasm)$/.test(name)).sort()
  .map(async name => [name, hash(await readFile(join(root, 'app/dist/assets', name)))])));
metadata.benchmarkHtmlSha256 = hash(await readFile(join(root, 'app/dist/bench-cad.html')));
for (const [prefix, expected] of [
  ['boardstudio_cadrum_wasm_bg-', metadata.wasmSha256],
  ['boardstudio_core_bg-', metadata.coreWasmSha256],
  ['boardstudio_renderer_wasm_bg-', metadata.rendererWasmSha256],
]) {
  const matches = assets.filter(name => name.startsWith(prefix) && name.endsWith('.wasm'));
  if (matches.length !== 1 || hash(await readFile(join(root, 'app/dist/assets', matches[0]))) !== expected) {
    throw new Error(`Stale production asset ${prefix}; rebuild the app before measuring`);
  }
}

const server = spawn(process.execPath, ['node_modules/vite/bin/vite.js', 'preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: join(root, 'app'), stdio: ['ignore', 'pipe', 'pipe'] });
let serverLog = '';
server.stdout.on('data', value => { serverLog += value; });
server.stderr.on('data', value => { serverLog += value; });
const origin = `http://127.0.0.1:${port}`;

try {
  for (let attempts = 0; ; attempts++) {
    if (server.exitCode !== null) throw new Error(serverLog);
    try { if (serverLog.includes(origin) && (await fetch(`${origin}/bench-cad.html`)).ok) break; } catch { /* Wait for Vite. */ }
    if (attempts >= 100) throw new Error(`Preview server did not start: ${serverLog}`);
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  await mkdir(dirname(output), { recursive: true });
  await mkdir(output);
  for (let session = 0; session < sessions; session++) {
    assertQuietHost();
    const sampler = rssSampler(new Set([server.pid]));
    let browserServer;
    try {
      browserServer = await chromium.launchServer({ executablePath: browserPath, headless: true });
      const browser = await chromium.connect(browserServer.wsEndpoint());
      const version = browser.version();
      if (metadata.browserVersion && metadata.browserVersion !== version) throw new Error('Browser version changed between sessions');
      metadata.browserVersion = version;
      const page = await browser.newPage({ viewport: metadata.viewport });
      page.on('pageerror', error => console.error(error));
      page.on('console', message => { if (message.type() === 'error') console.error(message.text()); });
      page.on('requestfailed', request => console.error(`${request.url()}: ${request.failure()?.errorText}`));
      await page.goto(`${origin}/bench-cad.html?cadMetrics=${profiled ? 1 : 0}`);
      await page.waitForFunction(() => Boolean(window.cadBenchmark)).catch(async error => {
        console.error(await page.evaluate(() => ({ title: document.title, resources: performance.getEntriesByType('resource').map(entry => entry.name) })));
        throw error;
      });
      for (const fixture of fixtures) {
        assertQuietHost();
        const ordered = rotateScenarios ? [...scenarios.slice(session % scenarios.length), ...scenarios.slice(0, session % scenarios.length)] : scenarios;
        const applicable = fixture.kind === 'step' ? ordered.filter(scenario => ['cold', 'warm-uncached'].includes(scenario)) : ordered;
        for (const scenario of applicable) {
          sampler.label(`${fixture.id}/${scenario}/setup`);
          await page.evaluate(input => window.cadBenchmark.configure(input), fixture.input);
          if (!['cold', 'cancel-retry'].includes(scenario)) await page.evaluate(() => window.cadBenchmark.sample({ scenario: 'cache-hit', index: 0 }));
          const count = ['cold', 'cancel-retry'].includes(scenario) ? coldSamples : samples;
          for (let index = 1; index <= count; index++) {
            const label = `${fixture.id}/${scenario}/${index}`;
            sampler.label(label);
            const result = await page.evaluate(options => window.cadBenchmark.sample(options), { scenario, index });
            all.push({ session, fixture: fixture.id, label, ...result });
          }
          assertQuietHost();
          console.info(`session ${session + 1}/${sessions}: ${fixture.id} ${scenario} (${count})`);
        }
      }
      await browser.close();
    } finally {
      await browserServer?.close();
      const memory = await sampler.stop();
      const current = all.filter(result => result.session === session);
      for (const result of current) result.sampledPeakRssBytes = Math.max(0, ...memory.filter(row => row.label === result.label).map(row => row.rssBytes));
      await writeFile(join(output, `session-${session + 1}.json.gz`), gzipSync(JSON.stringify({ metadata, session, samples: current, memory })));
    }
  }
  const percentile = (values, quantile) => values.toSorted((a, b) => a - b)[Math.ceil(values.length * quantile) - 1];
  const summaries = {};
  for (const result of all) {
    const key = `${result.fixture}/${result.scenario}`;
    (summaries[key] ??= []).push(result);
  }
  const summary = Object.fromEntries(Object.entries(summaries).map(([key, rows]) => [key, {
    samples: rows.length,
    completedMs: { p50: percentile(rows.map(row => row.completedMs), 0.5), p95: percentile(rows.map(row => row.completedMs), 0.95) },
    paintedMs: profiled && rows.every(row => row.paintedMs !== null) ? { p50: percentile(rows.map(row => row.paintedMs), 0.5), p95: percentile(rows.map(row => row.paintedMs), 0.95) } : null,
    sampledPeakRssBytes: Math.max(...rows.map(row => row.sampledPeakRssBytes)),
  }]));
  await writeFile(join(output, 'summary.json'), JSON.stringify({ metadata, summary }, null, 2) + '\n');
} finally { server.kill('SIGTERM'); }
