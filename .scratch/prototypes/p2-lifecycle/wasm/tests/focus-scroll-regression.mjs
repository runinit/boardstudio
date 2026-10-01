import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const wasmDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(wasmDir, '../../../../../');
const appRequire = createRequire(resolve(repoRoot, 'app/package.json'));
const { chromium } = appRequire('@playwright/test');
const url = process.argv[2];

if (!url) {
  throw new Error('Usage: node focus-scroll-regression.mjs <prototype-url>');
}

const browser = await chromium.launch({
  headless: true,
  ...(process.env.P2_CHROMIUM_EXECUTABLE
    ? { executablePath: process.env.P2_CHROMIUM_EXECUTABLE }
    : {}),
  args: ['--no-sandbox', '--enable-unsafe-swiftshader'],
});

try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 577 } });
  const runtimeErrors = [];
  page.on('pageerror', (error) => runtimeErrors.push(error.message));
  page.on('requestfailed', (request) => {
    runtimeErrors.push(`${request.url()}: ${request.failure()?.errorText ?? 'request failed'}`);
  });

  const response = await page.goto(url, { waitUntil: 'networkidle' });
  assert.equal(response?.status(), 200);
  await page.waitForSelector('svg.layout-canvas');
  await page.waitForFunction(() => document.documentElement.lang === 'en');

  const initial = await page.evaluate(() => {
    const svg = document.querySelector('svg.layout-canvas');
    const target = document.querySelector('[data-part-id="matrix/main-right-keys/r0c0"]');
    const rect = svg.getBoundingClientRect();
    return {
      scrollY,
      svgRect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
      lang: document.documentElement.lang,
      targetHit: document.elementFromPoint(393, 442)?.closest('[data-part-id]') === target,
      transform: target?.getAttribute('transform'),
    };
  });
  assert.equal(initial.scrollY, 0);
  assert.ok(Math.abs(initial.svgRect.top - 273.125) < 1, JSON.stringify(initial));
  assert.equal(initial.lang, 'en');
  assert.ok(initial.targetHit, JSON.stringify(initial));

  await page.mouse.move(393, 442);
  await page.mouse.down();
  const afterDown = await page.evaluate(() => ({
    scrollY,
    svgTop: document.querySelector('svg.layout-canvas').getBoundingClientRect().top,
    focused: document.activeElement === document.querySelector('svg.layout-canvas'),
  }));
  assert.equal(afterDown.scrollY, 0, JSON.stringify(afterDown));
  assert.ok(Math.abs(afterDown.svgTop - initial.svgRect.top) < 1, JSON.stringify(afterDown));
  assert.ok(afterDown.focused, JSON.stringify(afterDown));

  await page.mouse.move(423, 466);
  await page.mouse.up();
  await page.waitForFunction(
    (before) => document
      .querySelector('[data-part-id="matrix/main-right-keys/r0c0"]')
      ?.getAttribute('transform') !== before,
    initial.transform,
  );
  await page.waitForFunction(() => document
    .querySelector('p.status[role="status"]')
    ?.textContent?.includes('Drag committed by public CoreEngine worker'));

  const final = await page.evaluate(() => {
    const svg = document.querySelector('svg.layout-canvas').getBoundingClientRect();
    const target = document.querySelector('[data-part-id="matrix/main-right-keys/r0c0"]');
    const rect = target.getBoundingClientRect();
    return {
      scrollY,
      svgTop: svg.top,
      targetRect: { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom },
      transform: target.getAttribute('transform'),
      focused: document.activeElement === document.querySelector('svg.layout-canvas'),
    };
  });
  assert.equal(final.scrollY, 0, JSON.stringify(final));
  assert.ok(Math.abs(final.svgTop - initial.svgRect.top) < 1, JSON.stringify(final));
  assert.ok(final.targetRect.right > initial.svgRect.left, JSON.stringify(final));
  assert.ok(final.targetRect.left < initial.svgRect.left + initial.svgRect.width, JSON.stringify(final));
  assert.ok(final.targetRect.bottom > initial.svgRect.top, JSON.stringify(final));
  assert.ok(final.targetRect.top < initial.svgRect.top + initial.svgRect.height, JSON.stringify(final));
  assert.ok(final.focused, JSON.stringify(final));
  assert.deepEqual(runtimeErrors, []);

  console.log(JSON.stringify({ url, viewport: [1280, 577], initial, afterDown, final, runtimeErrors }, null, 2));
} finally {
  await browser.close();
}
