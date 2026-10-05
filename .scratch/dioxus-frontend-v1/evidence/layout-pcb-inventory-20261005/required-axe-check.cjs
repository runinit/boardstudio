// Required F5.8/F3.7 automated accessibility observations of the immutable app.
// Runs a separate ephemeral headless test process; never attaches to user tabs.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { createRequire } = require('node:module');
const root = '/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001';
const { chromium } = createRequire(path.join(root, 'app/package.json'))('@playwright/test');
const reference = process.argv.includes('--reference');
const output = path.join(root, '.scratch/dioxus-frontend-v1/evidence/' + (reference ? 'integrated-accessibility-reference-20261005' : 'integrated-accessibility-c1d3-20261005'));
const fixture = path.join(root, '.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/layered-sofle-export.boardstudio');
const axePath = '/tmp/boardstudio-required-a11y-20261005/axe.min.js';
const digest = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
fs.mkdirSync(output, { recursive: true });
(async () => {
  const browser = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
  try {
    const context = await browser.newContext({ viewport: { width: 1280, height: 720 }, colorScheme: 'light' });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(String(error)));
    await page.goto(reference ? 'http://127.0.0.1:5175/' : 'http://127.0.0.1:34825/', { waitUntil: 'domcontentloaded' });
    let chooser;
    if (reference) {
      await page.getByRole('button', { name: 'Project', exact: true }).click();
      await page.getByRole('button', { name: 'Open project…', exact: true }).waitFor({ state: 'visible', timeout: 90000 });
      chooser = page.waitForEvent('filechooser').catch(error => ({error}));
      await page.getByRole('button', { name: 'Open project…', exact: true }).click();
    } else {
      chooser = page.waitForEvent('filechooser').catch(error => ({error}));
      await page.getByRole('button', { name: 'Import .boardstudio', exact: true }).click();
    }
    const selected = await chooser;
    if (selected.error) throw selected.error;
    await selected.setFiles(fixture);
    await page.getByRole('button', { name: 'Add object', exact: true }).waitFor({ state: 'visible', timeout: 90000 });
    await page.addScriptTag({ path: axePath });
    const results = [];
    for (const workspace of ['PCB', 'Layout']) {
      await page.getByRole('tab', { name: workspace, exact: true }).click();
      if (workspace === 'PCB') {
        await page.getByRole('combobox', { name: 'Wiring mode', exact: true }).waitFor({ state: 'visible', timeout: 90000 });
      } else {
        await page.getByRole(reference ? 'treeitem' : 'button', { name: 'keys Independent', exact: true }).click();
        await page.getByRole('spinbutton', { name: reference ? 'Rows keys' : 'Rows', exact: true }).waitFor({ state: 'visible' });
      }
      const observation = await page.evaluate(() => ({
        url: location.href,
        scripts: [...document.scripts].map(s => s.src).filter(s => s.includes('boardstudio-web') || /main-.*\.js/.test(s)),
        selectedTab: document.querySelector('[role="tab"][aria-selected="true"]')?.textContent,
        viewport: { width: innerWidth, height: innerHeight },
        visibleHeadings: [...document.querySelectorAll('h1,h2,h3')].filter(e => e.getBoundingClientRect().height > 0).map(e => e.textContent),
      }));
      const report = await page.evaluate(async () => await axe.run(document));
      const evidence = { source: reference ? '5a472a9426e6e38993361da402cd4ec730feb369' : 'c1d3f13c642380c6a95ce8ae317fd99e6743c04f', fixture, fixtureSha256: digest(fixture), axeSha256: digest(axePath), observation, report };
      fs.writeFileSync(path.join(output, workspace.toLowerCase() + '-axe.json'), JSON.stringify(evidence, null, 2) + '\n');
      results.push({ workspace, violations: report.violations.map(v => ({ id: v.id, impact: v.impact, nodes: v.nodes.length })), incomplete: report.incomplete.map(v => ({ id: v.id, nodes: v.nodes.length })) });
    }
    fs.writeFileSync(path.join(output, 'summary.json'), JSON.stringify({ results, pageErrors: errors }, null, 2) + '\n');
    console.log(JSON.stringify({ results, pageErrors: errors }));
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error.stack || error); process.exitCode = 1; });
