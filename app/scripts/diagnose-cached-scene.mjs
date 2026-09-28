import { chromium } from '@playwright/test';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { cpus } from 'node:os';
import { resolve, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

// Controlled diagnostic only. Alternates complete app builds with the same WASM
// and frozen fixture; worker/draw counters are deliberately outside acceptance.
const root = fileURLToPath(new URL('../../', import.meta.url));
const [controlDirectory, outputDirectory] = process.argv.slice(2);
if (!controlDirectory || !outputDirectory) throw new Error('Usage: diagnose-cached-scene.mjs control-dist fresh-output-directory');
const directories = { control: resolve(controlDirectory), current: join(root, 'app/dist') };
const output = resolve(outputDirectory);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const assets = {};
for (const [variant, directory] of Object.entries(directories)) {
  assets[variant] = Object.fromEntries(await Promise.all((await readdir(join(directory, 'assets')))
    .filter(name => /\.(js|wasm)$/.test(name)).sort()
    .map(async name => [name, hash(await readFile(join(directory, 'assets', name)))])));
}
const wasmHashes = variant => Object.entries(assets[variant]).filter(([name]) => name.endsWith('.wasm')).map(([, value]) => value).sort();
if (JSON.stringify(wasmHashes('control')) !== JSON.stringify(wasmHashes('current'))) throw new Error('WASM changed between diagnostic builds');
const manifest = JSON.parse(await readFile(join(root, 'cad/bench/fixtures/manifest.json')));
const fixture = manifest.fixtures.find(value => value.id === 'gasketed-pair');
const bytes = await readFile(join(root, 'cad/bench/fixtures', fixture.file));
if (hash(bytes) !== fixture.sha256) throw new Error('Frozen fixture changed');
await mkdir(output);
let variant = 'control';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css' };
const server = createServer(async (request, response) => {
  const directory = directories[variant];
  const path = resolve(directory, `.${new URL(request.url, 'http://localhost').pathname}`);
  if (!path.startsWith(`${directory}/`)) { response.writeHead(403).end(); return; }
  try { response.writeHead(200, { 'Content-Type': types[extname(path)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' }).end(await readFile(path)); }
  catch { response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const all = [];
const percentile = (values, q) => values.sort((a, b) => a - b)[Math.ceil(values.length * q) - 1];
try {
  for (let pair = 0; pair < 5; pair++) for (const name of pair % 2 ? ['current', 'control'] : ['control', 'current']) {
    const processes = execFileSync('ps', ['-eo', 'args='], { encoding: 'utf8' }).split('\n');
    if (processes.some(command => !/^(?:\S*\/)?(?:bash|sh|zsh)\s/.test(command.trim())
      && /(?:^|\/)(?:cargo|rustc|wasm-opt)(?:\s|$)|vitest(?:\.mjs)? run|playwright(?:[^ ]*\/cli\.js)? test/.test(command))) throw new Error('Competing build/test process');
    variant = name;
    const browser = await chromium.launch({ executablePath: process.env.BOARDSTUDIO_CHROMIUM ?? '/usr/bin/chromium', headless: true });
    try {
      const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
      await page.addInitScript(() => {
        window.sceneDiagnostic = { workerScenes: 0, drawCalls: 0 };
        const post = Worker.prototype.postMessage;
        Worker.prototype.postMessage = function(message, ...rest) {
          if (message?.scene) window.sceneDiagnostic.workerScenes++;
          return post.call(this, message, ...rest);
        };
        for (const key of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) {
          const draw = WebGL2RenderingContext.prototype[key];
          WebGL2RenderingContext.prototype[key] = function(...args) {
            window.sceneDiagnostic.drawCalls++;
            return draw.apply(this, args);
          };
        }
      });
      await page.goto(`http://127.0.0.1:${server.address().port}/bench-cad.html?cadMetrics=1`);
      await page.waitForFunction(() => Boolean(window.cadBenchmark));
      await page.evaluate(assembly => window.cadBenchmark.configure({ assembly }), JSON.parse(bytes));
      await page.evaluate(() => window.cadBenchmark.sample({ scenario: 'cache-hit', index: 0 }));
      const samples = [];
      for (let index = 1; index <= 20; index++) samples.push(await page.evaluate(async index => {
        const before = { ...window.sceneDiagnostic };
        const result = await window.cadBenchmark.sample({ scenario: 'cache-hit', index });
        return { ...result, workerScenes: window.sceneDiagnostic.workerScenes - before.workerScenes,
          drawCalls: window.sceneDiagnostic.drawCalls - before.drawCalls };
      }, index));
      const row = { pair, variant, browser: browser.version(), samples };
      all.push(row);
      await writeFile(join(output, `pair-${pair + 1}-${variant}.json`), JSON.stringify(row));
      console.info(`pair ${pair + 1} ${variant}: paint p95 ${percentile(samples.map(s => s.paintedMs), .95).toFixed(1)} ms`);
    } finally { await browser.close(); }
  }
  const summary = Object.fromEntries(Object.keys(directories).map(name => {
    const rows = all.filter(row => row.variant === name).flatMap(row => row.samples);
    return [name, { samples: rows.length, paintedP95Ms: percentile(rows.map(row => row.paintedMs), .95),
      completedP95Ms: percentile(rows.map(row => row.completedMs), .95),
      workerScenes: rows.reduce((sum, row) => sum + row.workerScenes, 0), drawCalls: rows.reduce((sum, row) => sum + row.drawCalls, 0) }];
  }));
  await writeFile(join(output, 'summary.json'), JSON.stringify({ acceptanceEligible: false, kind: 'cached-scene-paired-diagnostic',
    pairs: 5, samplesPerSession: 20, cpu: cpus()[0].model, fixture, assets, summary }, null, 2));
  console.info(summary);
} finally { server.close(); }
