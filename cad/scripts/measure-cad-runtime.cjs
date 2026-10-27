// 011 capture driver, deliberately separate from product code. Requires a built packaged worker.
const { chromium } = require('playwright-core');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { performance } = require('node:perf_hooks');

const [origin, backend, profile, workload, output, pair, sequence] = process.argv.slice(2);
const [fixture, operation] = workload.split(':');
let activeBrowser;
const csv = values => values.map(value => `"${String(value ?? '').replaceAll('"', '""')}"`).join(',');
const validation = sequence === 'validation';
const metrics = fs.createWriteStream(path.join(output, validation ? 'diagnostic.csv' : 'runtime.csv'));
metrics.write(csv(['backend', 'profile', 'workload', 'pair', 'state', 'ordinal', 'metric', 'value', 'unit', 'status']) + '\n');
const event = (name, data = {}) => fs.appendFileSync(path.join(output, 'events.log'),
  JSON.stringify({ unix_ms: Date.now(), name, ...data }) + '\n');
function metric(state, ordinal, name, value, unit = 'ms', status = 'observed') {
  assert(Number.isFinite(value) && value >= 0);
  metrics.write(csv([backend, profile, workload, pair, state, ordinal, name, value, unit, validation ? 'diagnostic-not-benchmark' : status]) + '\n');
}

(async () => {
  event('purpose', { validationOnly: validation, values: validation ? 'diagnostic-not-benchmark' : 'observation-awaiting-admission' });
  const warmStartup = workload === 'startup:warm';
  const flags = ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'];
  const browser = activeBrowser = await chromium.launch({ executablePath: process.env.CHROME_BIN, args: flags });
  event('browser', { version: browser.version(), executable: process.env.CHROME_BIN, flags,
    viewport: [1513, 1043], gc: 'natural', httpCache: warmStartup, serviceWorkers: warmStartup ? 'allow' : 'block' });
  const context = await browser.newContext({ viewport: { width: 1513, height: 1043 },
    serviceWorkers: warmStartup ? 'allow' : 'block' });
  const page = await context.newPage();
  page.setDefaultTimeout(180000);
  const resources = [];
  context.on('response', response => resources.push({ url: response.url(), status: response.status(),
    serviceWorker: response.fromServiceWorker() }));
  const pageErrors = [];
  page.on('pageerror', error => { pageErrors.push(String(error)); event('page-error', { error: String(error) }); });
  const cdp = await context.newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.setCacheDisabled', { cacheDisabled: !warmStartup });
  let failed = false;
  try {
    if (fixture === 'U1') {
      event('U1', await require('./measure-cad-page.cjs')(page, origin, output, operation));
      return;
    }
    if (fixture === 'startup') {
      if (warmStartup) {
        await page.goto(`${origin}/boardstudio/`);
        await page.getByRole('button', { name: 'Create new keyboard', exact: true }).waitFor();
        event('prime-navigation');
        event('service-worker-state', await page.evaluate(async () => {
          const registrations = await navigator.serviceWorker.getRegistrations();
          if (registrations.length) await Promise.race([navigator.serviceWorker.ready,
            new Promise((_, reject) => setTimeout(() => reject(Error('service-worker activation watchdog: 180 seconds')), 180000))]);
          return { registrations: registrations.map(r => ({ scope: r.scope, active: r.active?.state,
            installing: r.installing?.state, waiting: r.waiting?.state })), controller: !!navigator.serviceWorker.controller };
        }));
      }
      event('navigation-start');
      const start = performance.now();
      await page.goto(`${origin}/boardstudio/`);
      await page.getByRole('button', { name: 'Create new keyboard', exact: true }).waitFor();
      metric(operation, 0, 'navigation-to-usable-landing', performance.now() - start);
      event('landing-ready');
      // A landing page is not a frozen project's usable editor or first rendered Case frame.
    }
    await page.goto(`${origin}/__perf/cad/scripts/performance-host.html`);
    const result = await page.evaluate(require('./measure-cad-worker.cjs'), { backend, fixture, operation, sequence, isStartup: fixture === 'startup' });
    event('prepared', result);
    result.startup.forEach((ms, i) => metric('worker-startup', i, 'worker-create-to-ready', ms));
    // A fixed 100ms dwell supplies idle/retained samples at 50ms; it is never inside latency or a retry.
    const dwell = () => new Promise(resolve => setTimeout(resolve, 100));
    await dwell();
    event('idle-baseline');
    const count = validation || fixture === 'startup' ? 1 : sequence === 'growth20' ? 20 : 6;
    for (let i = 0; i < count; ++i) {
      const state = sequence === 'growth20' ? 'growth' : i === 0 ? 'cold' : 'warm';
      if (operation === 'step' && i > 0) {
        await page.evaluate(() => window.__measure.startWorker());
        event('fresh-export-worker-ready', { ordinal: i });
      }
      event('operation-start', { state, ordinal: i });
      const sample = await page.evaluate(() => window.__measure.run());
      event('reply-retained', { state, ordinal: i, ...sample });
      metric(state, i, 'dispatch-to-validated-reply', sample.duration);
      for (const name of ['triangles', 'bufferBytes', 'stepBytes']) metric(state, i, name, sample[name], name === 'triangles' ? 'count' : 'bytes');
      await dwell();
      await page.evaluate(() => window.__measure.drop());
      event('references-dropped', { state, ordinal: i });
      await dwell();
    }
    event('scope-limit', { print: 'print-keycaps reply only; Runtime binary-STL/ZIP/download remains U1 admission',
      startup: 'landing plus isolated real worker/reply; frozen editor/project and rendered frame remain U1 admission',
      memory: 'process PSS/RSS in memory.csv; worker WASM linear capacity unavailable through production API',
      geometry: 'shape/census checks supplement required independent untimed admission; observed is not accepted' });
    assert.deepEqual(pageErrors, [], 'page errors invalidate the observation');
  } catch (error) {
    failed = true;
    event('failure', { error: String(error.stack ?? error) });
    process.exitCode = 1;
  } finally {
    event('resources', { responses: resources });
    event('result', { failed });
    metrics.end();
    await browser.close();
    activeBrowser = null;
  }
})().catch(async error => {
  event('failure', { error: String(error.stack ?? error) }); metrics.end(); process.exitCode = 1;
  await activeBrowser?.close();
});
