import { createServer } from 'node:http';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import os from 'node:os';
const directory = resolve(fileURLToPath(new URL('.', import.meta.url)));
const { chromium } = createRequire(resolve(directory, '../../../app/package.json'))('@playwright/test');
const stage = resolve(process.argv[2]);
const output = resolve(process.argv[3]);
await mkdir(output, { recursive: true });
const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url, 'http://localhost');
    if (url.pathname === '/') { response.setHeader('Content-Type', 'text/html'); response.end('<title>Bounded gasket comparison</title>'); return; }
    const roots = { cadrum: resolve(stage, 'cad/wasm/pkg'), manifold: resolve(directory, 'node_modules/manifold-3d'), experiment: directory };
    const [, key, ...segments] = url.pathname.split('/');
    const root = roots[key];
    if (!root) throw Error('Unknown route');
    const path = resolve(root, ...segments);
    if (!path.startsWith(root + '/')) throw Error('Invalid path');
    response.setHeader('Content-Type', extname(path) === '.wasm' ? 'application/wasm' : 'text/javascript');
    response.end(await readFile(path));
  } catch { response.statusCode = 404; response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true });
const fixtures = [];
for (const name of (process.env.CAD_EXPERIMENT_FIXTURES ?? 'regression,edit-0,edit-2,edit-5').split(',')) {
  const input = await readFile(resolve(stage, 'inputs', name + '.json'), 'utf8');
  fixtures.push({ name, input, sha256: createHash('sha256').update(input).digest('hex') });
}
const variants = (process.env.CAD_EXPERIMENT_VARIANTS ?? 'control,no-edges,no-ids,lean,profiles,manifold').split(',');
const attributionVariants = process.env.CAD_EXPERIMENT_VARIANTS ? variants : ['control', 'lean', 'profiles', 'split-timing'];
const geometryVariants = process.env.CAD_EXPERIMENT_VARIANTS ? variants : ['control', 'profiles', 'manifold'];
const rows = [];
const metadata = { date: new Date().toISOString(), platform: os.platform(), cpu: os.cpus()[0].model,
  browser: browser.version(), sessions: 5, samplesPerFixture: 10, variants,
  fixtures: fixtures.map(({input, ...rest}) => rest), note: 'Uncached single-bottom kernel diagnostic; excludes UI, initialization, JSON and correctness checks.' };
await writeFile(resolve(output, 'metadata.json'), JSON.stringify(metadata, null, 2));
try {
  for (let session = 0; session < 5; session++) {
    const page = await browser.newPage();
    page.on('pageerror', error => console.error(error));
    page.on('requestfailed', request => console.error(request.url(), request.failure()));
    await page.goto(url);
    await page.evaluate(() => {
      window.worker = new Worker('/experiment/worker.mjs', { type: 'module' });
      window.call = data => new Promise((resolve, reject) => {
        const timeout = setTimeout(() => { window.worker.terminate(); reject(Error('Kernel exceeded 30 seconds')); }, 30000);
        window.worker.onmessage = ({data}) => { clearTimeout(timeout); data.error ? reject(Error(data.error)) : resolve(data); };
        window.worker.onerror = event => { clearTimeout(timeout); reject(Error(event.message)); };
        window.worker.postMessage(data);
      });
    });
    // Cold and warm-up observations are retained separately, not silently pooled.
    for (const variant of variants) {
      console.log(`Session ${session+1}: initialize ${variant}`);
      for (let warmup = 0; warmup < 3; warmup++) {
        const data = await page.evaluate(args => window.call(args), { input: fixtures[0].input, variant });
        rows.push({ session, phase: warmup ? 'warmup' : 'cold', fixture: fixtures[0].name, variant, ...data });
      }
    }
    for (let sample = 0; sample < 10; sample++) {
      for (const fixture of fixtures) {
        const order = (sample + session) % 2 ? [...variants].reverse() : variants;
        for (const variant of order) {
          const data = await page.evaluate(args => window.call(args), { input: fixture.input, variant });
          rows.push({ session, phase: 'warm', sample, fixture: fixture.name, variant, ...data });
        }
      }
    }
    // Attribution is intentionally outside the instrumentation-off speed samples.
    for (const variant of attributionVariants) {
      for (let sample = 0; sample < 5; sample++) {
        const data = await page.evaluate(args => window.call(args), { input: fixtures[0].input, variant, timed: true });
        rows.push({ session, phase: 'attribution', sample, fixture: fixtures[0].name, variant, ...data });
      }
    }
    if (session === 0) {
      for (const fixture of fixtures) for (const variant of geometryVariants) {
        const data = await page.evaluate(args => window.call(args), { input: fixture.input, variant, mesh: true });
        await writeFile(resolve(output, `${fixture.name}-${variant}-geometry.json`), JSON.stringify(data));
      }
    }
    await page.close();
    await writeFile(resolve(output, 'raw.json'), JSON.stringify(rows, null, 2));
    console.log(`Completed session ${session + 1}/5`);
  }
} catch (error) {
  await writeFile(resolve(output, 'failure.json'), JSON.stringify({error: String(error.stack ?? error), rows}, null, 2));
  throw error;
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
const quantile = (values, q) => values.sort((a,b) => a-b)[Math.max(0, Math.ceil(values.length*q)-1)];
const summary = [];
for (const fixture of fixtures) for (const variant of variants) {
  const samples = rows.filter(r => r.phase === 'warm' && r.fixture === fixture.name && r.variant === variant);
  summary.push({ fixture: fixture.name, variant, count: samples.length,
    medianMs: quantile(samples.map(r => r.result.generationMs), 0.5),
    p95Ms: quantile(samples.map(r => r.result.generationMs), 0.95),
    sessionMedians: Array.from({length:5},(_,i)=>quantile(samples.filter(r=>r.session===i).map(r=>r.result.generationMs),0.5)),
    triangles: samples[0].result.triangles, meshBytes: samples[0].result.meshBytes,
    heapBytesMin: Math.min(...samples.map(r=>r.heapBytes ?? NaN)), heapBytesMax: Math.max(...samples.map(r=>r.heapBytes ?? NaN)) });
}
await writeFile(resolve(output, 'summary.json'), JSON.stringify(summary, null, 2));
console.log(JSON.stringify(summary.filter(r => r.fixture === 'regression'), null, 2));
