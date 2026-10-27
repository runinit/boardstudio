// Run with NODE_PATH, CHROME_BIN, and BOARDSTUDIO_PREVIEW_URL pointing to a running site.
const assert = require('node:assert/strict');
const { chromium } = require('playwright-core');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROME_BIN, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    for (const [width, height] of [[390, 844], [1513, 1043]]) {
      const page = await browser.newPage({ viewport: { width: 1513, height: 1043 } });
      page.setDefaultTimeout(30000);
      await page.goto(process.env.BOARDSTUDIO_PREVIEW_URL, { waitUntil: 'domcontentloaded', timeout: 60000 });
      for (const name of ['Create new keyboard', 'Split keyboard', 'Continue to layout', 'Create matrix'])
        await page.getByRole('button', { name, exact: true }).click();
      await page.locator('.m1-scene-part[data-part-id^="matrix/"]').first().waitFor({ timeout: 60000 });
      await page.getByRole('button', { name: 'Hide guide', exact: true }).click();
      await page.getByRole('button', { name: 'Open Parts workspace', exact: true }).click();
      await page.locator('#m1-objects-panel').getByRole('button', { name: 'New assembly', exact: true }).click();
      await page.getByRole('option', { name: 'MX Hotswap', exact: true }).click();
      await page.setViewportSize({ width, height });
      const body = page.locator('.m1-inspector-body');
      const box = await body.boundingBox();
      await page.mouse.move(box.x + box.width / 2, Math.min(box.y + box.height / 2, height - 40));
      await page.mouse.wheel(0, 1000);
      await page.waitForTimeout(300);
      const side = page.getByLabel('side', { exact: true });
      const state = await side.evaluate(e => {
        const r = e.getBoundingClientRect(), body = e.closest('.m1-inspector-body');
        return { top: r.top, bottom: r.bottom, scrollTop: body.scrollTop, overflow: getComputedStyle(body).overflowY,
          hit: r.top >= 0 && r.bottom <= innerHeight && (document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2) === e),
          pageOverflow: document.documentElement.scrollWidth > innerWidth };
      });
      console.log(`${width}x${height}`, JSON.stringify(state));
      assert.ok(state.scrollTop > 0 && state.hit && !state.pageOverflow, 'wheel must expose the board side selector without horizontal overflow');
      await side.click({ timeout: 5000 });
      await side.press('End');
      await side.press('Enter');
      assert.equal(await side.inputValue(), 'B', 'board side changes via pointer and keyboard');
      console.log(`PASS ${width}x${height}: board side accessible and changed`);
      await page.close();
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
