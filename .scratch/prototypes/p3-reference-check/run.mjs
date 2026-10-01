import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '../../..');
const p3 = path.resolve(process.argv[2]);
const attempt = process.argv[3];
assert(attempt && /^[a-zA-Z0-9-]+$/.test(attempt), 'provide a unique attempt name');
const output = path.join(here, 'target', attempt);
await fs.mkdir(output, { recursive: false });
const appRequire = createRequire(path.join(repo, 'app/package.json'));
const { build } = await import(appRequire.resolve('vite'));
const { chromium } = appRequire('@playwright/test');
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const sourceInputs = [
  path.join(repo, 'app/src/storage.ts'), path.join(repo, 'core/pkg/boardstudio_core.js'),
  path.join(repo, 'core/pkg/boardstudio_core_bg.wasm'),
  path.join(p3, 'dist/root/pkg/p3_durability.js'), path.join(p3, 'dist/root/pkg/p3_durability_bg.wasm'),
  path.join(p3, 'fixtures/reviung41-original.boardstudio'),
];
const identities = Object.fromEntries(await Promise.all(sourceInputs.map(async (file) => [file, hash(await fs.readFile(file))])));
const report = { integrationCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim(), p3SourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: p3, encoding: 'utf8' }).trim(), sourceInputs: identities, scope: 'Actual reference storage.ts, public archive WASM and P3 storage on an ephemeral origin/context; not React presentation or full M1.', cases: [] };
await build({ configFile: false, root: path.join(repo, 'app'), publicDir: false, build: {
  target: 'es2022', outDir: path.join(output, 'reference'), emptyOutDir: false,
  assetsInlineLimit: 0, rolldownOptions: { input: path.join(here, 'reference.ts'), output: { entryFileNames: 'reference.js' } },
} });
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.boardstudio': 'application/zip' };
const server = http.createServer(async (request, response) => {
  try {
    const pathname = new URL(request.url, 'http://localhost').pathname;
    if (pathname === '/probe.html') return response.writeHead(200, { 'content-type': 'text/html' }).end('<!doctype html><html lang="en"><title>Reference storage proof</title><script type="module" src="/reference/reference.js"></script></html>');
    const prefix = Object.keys({ '/reference/': 1, '/p3/': 1, '/core/': 1 }).find(prefix => pathname.startsWith(prefix));
    const base = { '/reference/': path.join(output, 'reference'), '/p3/': path.join(p3, 'dist/root'), '/core/': path.join(repo, 'core/pkg') }[prefix];
    if (!base) return response.writeHead(404).end();
    const file = path.resolve(base, decodeURIComponent(pathname.slice(prefix.length)));
    if (!file.startsWith(base + path.sep)) return response.writeHead(400).end();
    const bytes = await fs.readFile(file);
    response.writeHead(200, { 'content-type': mime[path.extname(file)] ?? 'application/octet-stream', 'cache-control': 'no-store' }).end(bytes);
  } catch { response.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
try {
  const context = await browser.newContext({ serviceWorkers: 'block' });
  const page = await context.newPage();
  report.browser = await browser.version();
  report.origin = origin;
  await page.goto(`${origin}/p3/index.html?skip-sw=1`);
  await page.waitForFunction(() => document.documentElement.dataset.p3WasmReady === 'true');
  await page.evaluate(async () => { await p3.importFixture(); p3.commitOneEdit(); await p3.savePending(true).catch(() => {}); await p3.retryPending(); });
  await page.goto(`${origin}/probe.html`);
  await page.waitForFunction(() => globalThis.referenceStorage);
  const result = await page.evaluate(async () => {
    const rust = await import('/p3/pkg/p3_durability.js'); await rust.default();
    const core = await import('/core/boardstudio_core.js'); await core.default();
    const reference = globalThis.referenceStorage;
    const stored = await rust.load_stored_project();
    const doc = stored.document;
    const read = (store, key) => new Promise((resolve, reject) => {
      const request = indexedDB.open('boardstudio-p3-durability', 1);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const db = request.result;
        const value = db.transaction(store).objectStore(store).get(key);
        value.onsuccess = () => { db.close(); resolve(value.result); };
        value.onerror = () => { db.close(); reject(value.error); };
      };
    });
    const digest = async bytes => [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(byte => byte.toString(16).padStart(2, '0')).join('');
    const assets = [];
    for (const asset of doc.assets) {
      const bytes = await read('assets', asset.sha256);
      if (!(bytes instanceof Uint8Array)) throw new Error('P3 asset representation is not Uint8Array');
      if (await digest(bytes) !== asset.sha256) throw new Error('P3 asset hash mismatch');
      assets.push({ hash: asset.sha256, bytes });
    }
    const [packedJson, packedBuffers] = rust.archive_request(JSON.stringify({ kind: 'pack-project', projectJson: JSON.stringify(doc), assets: assets.map((asset, index) => ({ path: `assets/${asset.hash}`, bufferIndex: index })) }), assets.map(asset => asset.bytes));
    if (JSON.parse(packedJson).kind !== 'packed') throw new Error(packedJson);
    const transport = { async archive(input) {
      const [replyJson, buffers] = core.archive_request(JSON.stringify(input.request), input.buffers);
      const reply = JSON.parse(replyJson);
      if (reply.kind === 'error') throw new Error(reply.message);
      return { kind: 'archive', reply: reply.kind === 'packed' ? { kind: 'packed', bytes: buffers[0] } : { kind: 'unpacked', projectJson: reply.projectJson, assets: reply.assets.map(asset => ({ sha256: asset.sha256, bytes: buffers[asset.bufferIndex] })) } };
    } };
    const imported = await reference.unpackProject(packedBuffers[0], transport);
    await reference.saveProject(imported);
    const loaded = await reference.loadProject(imported.id);
    const assertSame = (actual, expected, label) => { if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(label); };
    assertSame(loaded, doc, 'Reference read changed Rust-persisted document');
    if (reference.activeProjectId('missing') !== doc.id) throw new Error('Reference active preference mismatch');
    for (const asset of assets) {
      const bytes = await reference.loadAsset(asset.hash);
      if (!(bytes instanceof Uint8Array) || await digest(bytes) !== asset.hash) throw new Error('Reference asset mismatch');
    }
    const referenceArchive = await reference.packProject(loaded, { embedUsedModels: false }, transport);
    const [unpackedJson, unpackedBuffers] = rust.archive_request(JSON.stringify({ kind: 'unpack-project' }), [referenceArchive]);
    const unpacked = JSON.parse(unpackedJson);
    if (unpacked.kind !== 'unpacked') throw new Error(unpackedJson);
    assertSame(JSON.parse(unpacked.projectJson), doc, 'Reference archive changed document on Rust reopen');
    for (const asset of unpacked.assets) if (await digest(unpackedBuffers[asset.bufferIndex]) !== asset.sha256) throw new Error('Reference archive asset mismatch');
    const beforeInvalid = JSON.stringify(await reference.loadProject(doc.id));
    let rejected = false;
    try { await reference.unpackProject(new Uint8Array([1, 2, 3]), transport); } catch { rejected = true; }
    if (!rejected || JSON.stringify(await reference.loadProject(doc.id)) !== beforeInvalid) throw new Error('Invalid archive changed reference state');
    return { projectId: doc.id, revision: doc.revision, parts: doc.parts.length, assetHashes: assets.map(asset => asset.hash), rustArchiveBytes: packedBuffers[0].byteLength, referenceArchiveBytes: referenceArchive.byteLength, exactDocument: true, typedAssetBytes: true, activePreference: true, invalidArchiveRejectedWithoutDocumentChange: true };
  });
  assert.equal(result.revision, 4); assert.equal(result.parts, 85); assert.equal(result.assetHashes.length, 4);
  report.cases.push(result);
  report.status = 'passed';
  await context.close();
} catch (error) { report.status = 'failed'; report.error = String(error.stack ?? error); process.exitCode = 1; }
finally {
  await browser.close(); await new Promise(resolve => server.close(resolve));
  for (const file of sourceInputs) assert.equal(hash(await fs.readFile(file)), identities[file], `Source changed during proof: ${file}`);
  await fs.writeFile(path.join(output, 'results.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
}
