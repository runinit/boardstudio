import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../../../../../');
const { chromium } = createRequire(resolve(repo, 'app/package.json'))('@playwright/test');
const [url, output] = process.argv.slice(2);
if (!url || !output) throw new Error('Usage: browser-acceptance.mjs <url> <evidence-directory>');
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.P2_CHROMIUM_EXECUTABLE || '/usr/bin/chromium',
  headless: true, args: ['--no-sandbox', '--enable-unsafe-swiftshader'],
});
const evidence = { url, checks: {}, runtimeErrors: [], failedRequests: [], limitations: [
  'GPU counters measure API resource ownership, not physical GPU memory.',
  'Accessibility tree and keyboard evidence do not establish screen-reader speech compatibility.',
  'Timings and transfer sizes are measurements; no new pass thresholds are introduced.',
] };
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  page.on('pageerror', error => evidence.runtimeErrors.push(error.message));
  page.on('requestfailed', request => evidence.failedRequests.push({ url: request.url(), error: request.failure() }));
  page.on('response', response => { if (response.status() >= 400) evidence.failedRequests.push({ url: response.url(), status: response.status() }); });
  await page.addInitScript(() => {
    const contexts = [];
    const observers = new Set();
    const nativeGetContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (...args) {
      const gl = nativeGetContext.apply(this, args);
      if (gl && this.classList.contains('renderer-canvas') && !contexts.some(entry => entry.gl === gl)) {
        const resources = {};
        contexts.push({ gl, canvas: this, resources });
        for (const name of ['Buffer', 'Texture', 'Program', 'Shader', 'Framebuffer', 'Renderbuffer', 'VertexArray']) {
          if (!gl[`create${name}`]) continue;
          const live = new Set();
          resources[name] = live;
          const create = gl[`create${name}`].bind(gl);
          const remove = gl[`delete${name}`].bind(gl);
          gl[`create${name}`] = (...parameters) => { const value = create(...parameters); if (value) live.add(value); return value; };
          gl[`delete${name}`] = value => { remove(value); live.delete(value); };
        }
      }
      return gl;
    };
    const NativeObserver = ResizeObserver;
    window.ResizeObserver = class extends NativeObserver {
      observe(target, options) { if (target.classList.contains('renderer-canvas')) observers.add(this); super.observe(target, options); }
      disconnect() { observers.delete(this); super.disconnect(); }
    };
    window.p2Resources = () => ({ observers: observers.size, contexts: contexts.map(({ canvas, resources }) => ({
      connected: canvas.isConnected, state: canvas.dataset.rendererState,
      live: Object.fromEntries(Object.entries(resources).map(([key, values]) => [key, values.size])),
    })) });
  });
  const start = Date.now();
  assert.equal((await page.goto(url))?.status(), 200);
  const ready = () => page.waitForFunction(() => {
    const canvas = document.querySelector('canvas.renderer-canvas');
    return canvas?.dataset.rendererState === 'active' && Number(canvas.dataset.renderSubmissions) > 0;
  });
  await ready();
  evidence.initialReadyMs = Date.now() - start;
  assert.equal(await page.locator('html').getAttribute('lang'), 'en');
  await page.screenshot({ path: resolve(output, 'initial.png'), fullPage: true });
  await writeFile(resolve(output, 'accessible-tree.txt'), await page.locator('body').ariaSnapshot());
  const target = page.locator('[data-part-id="matrix/main-right-keys/r0c0"]');
  const initial = await target.getAttribute('transform');
  const svg = page.getByRole('application', { name: 'Keyboard layout editor' });
  await svg.focus();
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(before => document.querySelector('[data-part-id="matrix/main-right-keys/r0c0"]').getAttribute('transform') !== before, initial);
  const moved = await target.getAttribute('transform');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await page.waitForFunction(before => document.querySelector('[data-part-id="matrix/main-right-keys/r0c0"]').getAttribute('transform') === before, initial);
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await page.waitForFunction(before => document.querySelector('[data-part-id="matrix/main-right-keys/r0c0"]').getAttribute('transform') === before, moved);
  evidence.checks.keyboardUndoRedo = { initial, moved };
  await page.getByRole('button', { name: 'Unmount editor panel' }).focus();
  const focusOrder = [];
  for (let index = 0; index < 4; index++) {
    focusOrder.push(await page.evaluate(() => ({ tag: document.activeElement.tagName, text: document.activeElement.getAttribute('aria-label') || document.activeElement.textContent })));
    await page.keyboard.press('Tab');
  }
  assert.deepEqual(focusOrder.map(entry => entry.text), ['Unmount editor panel', 'Undo', 'Redo', 'Keyboard layout editor']);
  evidence.checks.focusOrder = focusOrder;
  await svg.focus();
  evidence.checks.focusStyle = await svg.evaluate(element => ({ outline: getComputedStyle(element).outline, offset: getComputedStyle(element).outlineOffset }));
  await page.evaluate(() => { window.originalP2Canvas = document.querySelector('canvas.renderer-canvas'); });
  const dimensions = () => page.locator('canvas.renderer-canvas').evaluate(canvas => ({ width: canvas.width, height: canvas.height, cssWidth: canvas.clientWidth, cssHeight: canvas.clientHeight, dpr: canvas.dataset.dpr }));
  const beforeDpr = await dimensions();
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 720, deviceScaleFactor: 2, mobile: false });
  await page.waitForFunction(() => document.querySelector('canvas.renderer-canvas')?.dataset.dpr === '2');
  const afterDpr = await dimensions();
  assert.equal(await page.evaluate(() => window.originalP2Canvas === document.querySelector('canvas.renderer-canvas')), true);
  assert.equal(afterDpr.width, beforeDpr.width * 2);
  assert.equal(afterDpr.height, beforeDpr.height * 2);
  evidence.checks.sameCanvasDpr = { beforeDpr, afterDpr };
  await cdp.send('Emulation.clearDeviceMetricsOverride');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForFunction(() => document.querySelector('canvas.renderer-canvas').clientHeight === 260);
  const compact = await page.evaluate(() => ({ viewport: innerWidth, content: document.documentElement.scrollWidth, canvas: document.querySelector('canvas.renderer-canvas').getBoundingClientRect().toJSON() }));
  assert.ok(compact.content <= compact.viewport, JSON.stringify(compact));
  assert.ok(compact.canvas.right <= compact.viewport, JSON.stringify(compact));
  evidence.checks.compact = compact;
  await page.screenshot({ path: resolve(output, 'compact.png'), fullPage: true });
  await page.setViewportSize({ width: 1280, height: 720 });
  const cycles = [];
  for (let index = 0; index < 4; index++) {
    await page.getByRole('button', { name: 'Unmount editor panel' }).click();
    await page.waitForFunction(() => !document.querySelector('canvas.renderer-canvas'));
    const resources = await page.evaluate(() => window.p2Resources());
    assert.equal(resources.observers, 0, JSON.stringify(resources));
    assert.ok(resources.contexts.every(entry => entry.state === 'disposed' && Object.values(entry.live).every(count => count === 0)), JSON.stringify(resources));
    cycles.push(resources);
    await page.getByRole('button', { name: 'Mount editor panel' }).click();
    await ready();
  }
  evidence.checks.teardownCycles = cycles;
  await page.evaluate(() => {
    const gl = document.querySelector('canvas.renderer-canvas').getContext('webgl2');
    const extension = gl.getExtension('WEBGL_lose_context');
    if (!extension) throw new Error('WEBGL_lose_context unavailable');
    extension.loseContext();
  });
  await page.waitForFunction(() => document.querySelector('canvas.renderer-canvas')?.dataset.rendererState === 'context-lost');
  evidence.checks.contextLoss = await page.locator('p.status').textContent();
  assert.match(evidence.checks.contextLoss, /context lost.*failure/);
  evidence.resources = await page.evaluate(() => performance.getEntriesByType('resource').map(({ name, transferSize, encodedBodySize, decodedBodySize, duration }) => ({ name, transferSize, encodedBodySize, decodedBodySize, duration })));
  assert.deepEqual(evidence.runtimeErrors, []);
  assert.deepEqual(evidence.failedRequests, []);
  evidence.status = 'passed';
} catch (error) {
  evidence.status = 'failed';
  evidence.failure = error.stack;
  throw error;
} finally {
  await writeFile(resolve(output, 'browser-acceptance.json'), JSON.stringify(evidence, null, 2) + '\n');
  await browser.close();
}
