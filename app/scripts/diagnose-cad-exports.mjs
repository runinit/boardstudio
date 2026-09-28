import { chromium } from '@playwright/test';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { cpus, platform, release } from 'node:os';
import { resolve, join, extname, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

// Diagnostic A/B only: identical production JS and fixtures, alternate binaries
// across fresh browsers. This subset cannot replace the full frozen CAD gate.
const root = fileURLToPath(new URL('../../', import.meta.url));
const [controlPackage, outputArgument] = process.argv.slice(2);
if (!controlPackage || !outputArgument) throw new Error('Usage: diagnose-cad-exports.mjs control-wasm-package fresh-output-directory');
const output = resolve(outputArgument);
const dist = join(root, 'app/dist');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const wasmName = 'boardstudio_cadrum_wasm_bg.wasm';
const glueName = 'boardstudio_cadrum_wasm.js';
const currentPackage = join(root, 'cad/wasm/pkg');
const currentGlue = await readFile(join(currentPackage, glueName));
if (!currentGlue.equals(await readFile(join(controlPackage, glueName)))) {
  throw new Error('Control JS bindings differ; a binary-only comparison is unsafe');
}
const binaries = {
  current: await readFile(join(currentPackage, wasmName)),
  control: await readFile(join(controlPackage, wasmName)),
};
const assets = await readdir(join(dist, 'assets'));
const wasmAssets = assets.filter(name => /^boardstudio_cadrum_wasm_bg-.*\.wasm$/.test(name));
if (wasmAssets.length !== 1 || !binaries.current.equals(await readFile(join(dist, 'assets', wasmAssets[0])))) {
  throw new Error('Stale or ambiguous production CAD WASM');
}
const manifest = JSON.parse(await readFile(join(root, 'cad/bench/fixtures/manifest.json')));
const fixture = manifest.fixtures.find(value => value.id === 'split-assembly');
const fixtureBytes = await readFile(join(root, 'cad/bench/fixtures', fixture.file));
if (hash(fixtureBytes) !== fixture.sha256) throw new Error('Frozen fixture changed');
const input = { assembly: JSON.parse(fixtureBytes) };
const scenarios = ['preview-export', 'direct-export'];
const budgets = JSON.parse(await readFile(join(root, 'cad/bench/budgets.json'))).scenarios;
const percentile = (values, fraction) => [...values].sort((a, b) => a - b)[Math.ceil(values.length * fraction) - 1];
await mkdir(dirname(output), { recursive: true });
await mkdir(output);

function assertQuietHost() {
  const commands = execFileSync('ps', ['-eo', 'args='], { encoding: 'utf8' }).split('\n');
  const competing = commands.filter(command => !/^(?:\S*\/)?(?:bash|sh|zsh)\s/.test(command.trim())
    && /(?:^|\/)(?:cargo|rustc|wasm-opt)(?:\s|$)|vitest(?:\.mjs)? run|playwright(?:[^ ]*\/cli\.js)? test/.test(command));
  if (competing.length) throw new Error(`Build/test overlap prevents measurement: ${competing.join('\n')}`);
}

async function hostSnapshot() {
  const [stat, pressure, load, frequencies] = await Promise.all([
    readFile('/proc/stat', 'utf8'), readFile('/proc/pressure/cpu', 'utf8'), readFile('/proc/loadavg', 'utf8'),
    Promise.all(cpus().map((_, index) => readFile(`/sys/devices/system/cpu/cpu${index}/cpufreq/scaling_cur_freq`, 'utf8').then(Number).catch(() => null))),
  ]);
  return { ms: performance.now(), cpuTicks: stat.split('\n')[0].trim().split(/\s+/).slice(1).map(Number),
    cpuPressure: pressure.trim(), load: load.trim(), frequencyKhz: frequencies };
}

async function processSnapshot() {
  const entries = await readdir('/proc');
  const rows = await Promise.all(entries.filter(name => /^\d+$/.test(name)).map(async pid => {
    try {
      const stat = await readFile(`/proc/${pid}/stat`, 'utf8');
      const end = stat.lastIndexOf(')');
      const fields = stat.slice(end + 2).split(' ');
      return { pid, command: stat.slice(stat.indexOf('(') + 1, end), startTicks: fields[19], ticks: Number(fields[11]) + Number(fields[12]) };
    } catch { return null; }
  }));
  return rows.filter(Boolean);
}

let variant = 'current';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml' };
const server = createServer(async (request, response) => {
  try {
    const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const target = resolve(dist, `.${path}`);
    if (!target.startsWith(`${dist}/`)) { response.writeHead(403).end(); return; }
    const bytes = path === `/assets/${wasmAssets[0]}` ? binaries[variant] : await readFile(target);
    response.writeHead(200, { 'Content-Type': types[extname(target)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' });
    response.end(bytes);
  } catch { response.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const metadata = {
  kind: 'paired-export-diagnostic', acceptanceEligible: false, pairs: 5, samplesPerScenarioPerSession: 20,
  cpu: cpus()[0].model, os: `${platform()} ${release()}`, revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  binaries: Object.fromEntries(Object.entries(binaries).map(([name, bytes]) => [name, hash(bytes)])),
  appAssets: Object.fromEntries(await Promise.all(assets.filter(name => /\.(wasm|js)$/.test(name)).sort()
    .map(async name => [name, hash(await readFile(join(dist, 'assets', name)))]))),
  cpuPolicy: Object.fromEntries(await Promise.all(['scaling_governor', 'scaling_driver', 'energy_performance_preference']
    .map(async name => [name, await readFile(`/sys/devices/system/cpu/cpu0/cpufreq/${name}`, 'utf8').then(value => value.trim()).catch(() => null)]))),
  bindingsSha256: hash(currentGlue), fixture, viewport: { width: 1280, height: 720 },
  order: 'AB, BA, AB, BA, AB; A=current, B=control; fresh browser for each variant',
  limitations: ['Diagnostic subset, not full frozen acceptance', 'One-second host samples may miss short interference', 'CPU frequency is OS reported, not effective cycle rate', 'Process deltas exclude processes that exit during a session', 'Control disables only the two Phase 2 production optimizations, retaining current instrumentation'],
};
const sessions = [];
try {
  for (let pair = 0; pair < metadata.pairs; pair++) {
    for (const name of pair % 2 ? ['control', 'current'] : ['current', 'control']) {
      assertQuietHost();
      variant = name;
      const before = await processSnapshot();
      const host = [await hostSnapshot()];
      let sampling = true;
      const sampleHost = (async () => {
        while (sampling) {
          await new Promise(resolve => setTimeout(resolve, 1000));
          if (sampling) host.push(await hostSnapshot());
        }
      })();
      let browser;
      const samples = [];
      try {
        browser = await chromium.launch({ executablePath: process.env.BOARDSTUDIO_CHROMIUM ?? '/usr/bin/chromium', headless: true });
        metadata.browser = browser.version();
        const page = await browser.newPage({ viewport: metadata.viewport });
        await page.goto(`${origin}/bench-cad.html?cadMetrics=1`);
        await page.waitForFunction(() => Boolean(window.cadBenchmark));
        for (const scenario of scenarios) {
          await page.evaluate(value => window.cadBenchmark.configure(value), input);
          await page.evaluate(() => window.cadBenchmark.sample({ scenario: 'cache-hit', index: 0 }));
          for (let index = 1; index <= metadata.samplesPerScenarioPerSession; index++) {
            const startMs = performance.now();
            const sample = await page.evaluate(value => window.cadBenchmark.sample(value), { scenario, index });
            samples.push({ ...sample, startMs, endMs: performance.now() });
          }
        }
        assertQuietHost();
      } finally {
        sampling = false;
        await sampleHost;
        host.push(await hostSnapshot());
        await browser?.close();
      }
      const after = await processSnapshot();
      const processes = after.flatMap(row => {
        const previous = before.find(value => value.pid === row.pid && value.startTicks === row.startTicks);
        return previous ? [{ ...row, deltaTicks: row.ticks - previous.ticks }] : [];
      }).sort((a, b) => b.deltaTicks - a.deltaTicks).slice(0, 15);
      const session = { pair: pair + 1, variant: name, samples, host, processes };
      await writeFile(join(output, `pair-${pair + 1}-${name}.json.gz`), gzipSync(JSON.stringify(session)));
      sessions.push(session);
      console.info(`pair ${pair + 1}/5 ${name}: ${scenarios.map(scenario => `${scenario} p95 ${percentile(samples.filter(row => row.scenario === scenario).map(row => row.completedMs), .95).toFixed(1)} ms`).join('; ')}`);
    }
  }
  const summary = {};
  for (const name of Object.keys(binaries)) {
    summary[name] = Object.fromEntries(scenarios.map(scenario => {
      const samples = sessions.filter(row => row.variant === name).flatMap(row => row.samples.filter(sample => sample.scenario === scenario));
      const completedP95Ms = percentile(samples.map(row => row.completedMs), .95);
      const paintedP95Ms = percentile(samples.map(row => row.paintedMs), .95);
      const budget = budgets[`split-assembly/${scenario}`];
      return [scenario, { samples: samples.length, completedP95Ms, paintedP95Ms, budget,
        completionWithinBudget: completedP95Ms <= budget.completedP95Ms, paintWithinBudget: paintedP95Ms <= budget.paintedP95Ms }];
    }));
  }
  await writeFile(join(output, 'summary.json'), JSON.stringify({ metadata, summary }, null, 2));
} finally { await new Promise(resolve => server.close(resolve)); }
