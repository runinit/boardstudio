import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs/promises';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '../../../..');
const archivePath = path.resolve(process.argv[2] ?? path.join(repo, 'web/target/builds/browser-storage-20261001/inputs/reviung41-rust-export.boardstudio'));
const attempt = process.argv[3] ?? 'm1-reference-roundtrip-20261001';
assert.match(attempt, /^[a-zA-Z0-9-]+$/);
const output = path.join(os.tmpdir(), `boardstudio-m1-reference-${attempt}`);
const evidenceResult = path.join(here, `${attempt}.json`);
await fs.mkdir(output, { recursive: false });

const appRequire = createRequire(path.join(repo, 'app/package.json'));
const { build } = await import(appRequire.resolve('vite'));
const { chromium } = appRequire('@playwright/test');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const referenceEntry = path.join(here, 'reference.ts');
const rustJs = path.join(repo, 'core/pkg/boardstudio_core.js');
const rustWasm = path.join(repo, 'core/pkg/boardstudio_core_bg.wasm');
const archive = await fs.readFile(archivePath);
const sourceFiles = [path.join(repo, 'app/src/storage.ts'), rustJs, rustWasm, referenceEntry, fileURLToPath(import.meta.url), archivePath];
const sourceHashes = Object.fromEntries(await Promise.all(sourceFiles.map(async (file) => [file, sha256(await fs.readFile(file))])));
const report = {
  integrationCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim(),
  sourceHashes,
  rustArchiveSha256: sha256(archive),
  isolatedDatabase: `boardstudio-m1-reference-test-${attempt}`,
  productionDatabaseProtectedByOpenAlias: true,
  browser: null,
  cases: [],
};

await build({ configFile: false, logLevel: 'error', root: path.join(repo, 'app'), publicDir: false, build: {
  target: 'es2022', outDir: path.join(output, 'reference'), emptyOutDir: false,
  assetsInlineLimit: 0, rolldownOptions: { input: referenceEntry, output: { entryFileNames: 'reference.js' } },
} });

const mime = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.boardstudio': 'application/zip' };
const server = http.createServer(async (request, response) => {
  try {
    const pathname = new URL(request.url, 'http://localhost').pathname;
    if (pathname === '/probe.html') return response.writeHead(200, { 'content-type': 'text/html' }).end('<!doctype html><html lang="en"><title>M1 React storage compatibility</title><script type="module" src="/reference/reference.js"></script></html>');
    const prefix = ['/reference/', '/core/', '/archive/'].find((entry) => pathname.startsWith(entry));
    const base = { '/reference/': path.join(output, 'reference'), '/core/': path.dirname(rustJs), '/archive/': path.dirname(archivePath) }[prefix];
    if (!base) return response.writeHead(404).end();
    const relative = prefix === '/archive/' ? path.basename(archivePath) : decodeURIComponent(pathname.slice(prefix.length));
    const file = path.resolve(base, relative);
    if (file !== base && !file.startsWith(base + path.sep)) return response.writeHead(400).end();
    const bytes = await fs.readFile(file);
    response.writeHead(200, { 'content-type': mime[path.extname(file)] ?? 'application/octet-stream', 'cache-control': 'no-store' }).end(bytes);
  } catch { response.writeHead(404).end(); }
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
try {
  const context = await browser.newContext({ serviceWorkers: 'block' });
  await context.addInitScript((suffix) => {
    const database = `boardstudio-m1-reference-test-${suffix}`;
    const activeKey = `boardstudio-m1-reference-test-${suffix}-active-project`;
    const open = IDBFactory.prototype.open;
    IDBFactory.prototype.open = function (name, ...args) { return open.call(this, name === 'boardstudio-v2' ? database : name, ...args); };
    const get = Storage.prototype.getItem;
    const set = Storage.prototype.setItem;
    const remove = Storage.prototype.removeItem;
    const map = (key) => key === 'boardstudio-v2-active-project' ? activeKey : key;
    Storage.prototype.getItem = function (key) { return get.call(this, map(key)); };
    Storage.prototype.setItem = function (key, value) { return set.call(this, map(key), value); };
    Storage.prototype.removeItem = function (key) { return remove.call(this, map(key)); };
  }, attempt);
  const page = await context.newPage();
  report.browser = await browser.version();
  report.origin = origin;
  await page.goto(`${origin}/probe.html`);
  await page.waitForFunction(() => globalThis.referenceStorage);
  const result = await page.evaluate(async (attemptName) => {
    const rust = await import('/core/boardstudio_core.js');
    await rust.default();
    const reference = globalThis.referenceStorage;
    const bytes = new Uint8Array(await (await fetch('/archive/m1-reviung-export.boardstudio')).arrayBuffer());
    const unpack = (archiveBytes) => {
      const [json, buffers] = rust.archive_request(JSON.stringify({ kind: 'unpack-project' }), [archiveBytes]);
      const reply = JSON.parse(json);
      if (reply.kind !== 'unpacked') throw new Error(json);
      return { reply, buffers };
    };
    const rustOriginal = unpack(bytes);
    const document = JSON.parse(rustOriginal.reply.projectJson);
    const assetBytes = new Map(rustOriginal.reply.assets.map((asset) => [asset.sha256, rustOriginal.buffers[asset.bufferIndex]]));
    const bridge = {
      async archive(input) {
        const [replyJson, replyBuffers] = rust.archive_request(JSON.stringify(input.request), input.buffers);
        const reply = JSON.parse(replyJson);
        if (reply.kind === 'error') throw new Error(reply.message);
        return { kind: 'archive', reply: reply.kind === 'packed'
          ? { kind: 'packed', bytes: replyBuffers[0] }
          : { kind: 'unpacked', projectJson: reply.projectJson, assets: reply.assets.map((asset) => ({ sha256: asset.sha256, bytes: replyBuffers[asset.bufferIndex] })) } };
      },
    };
    const imported = await reference.unpackProject(bytes, bridge);
    await reference.saveProject(imported);
    const loaded = await reference.loadProject(document.id);
    if (JSON.stringify(loaded) !== JSON.stringify(document)) throw new Error('React adapter changed Rust-exported document');
    if (reference.activeProjectId('missing') !== document.id) throw new Error('React active-project preference failed');
    const checkedAssets = [];
    for (const [hash] of assetBytes) {
      const asset = await reference.loadAsset(hash);
      if (!(asset instanceof Uint8Array)) throw new Error(`Reference adapter asset ${hash} was not Uint8Array`);
      const digest = await crypto.subtle.digest('SHA-256', asset);
      const actual = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
      if (actual !== hash) throw new Error(`Reference adapter asset hash mismatch ${hash}`);
      checkedAssets.push({ sha256: hash, byteLength: asset.byteLength, typedUint8Array: true });
    }
    const referenceArchive = await reference.packProject(loaded, { embedUsedModels: false }, bridge);
    const rustReopened = unpack(referenceArchive);
    if (JSON.stringify(JSON.parse(rustReopened.reply.projectJson)) !== JSON.stringify(document)) throw new Error('Rust reopen changed React-produced document');
    const reopenedHashes = [];
    for (const asset of rustReopened.reply.assets) {
      const digest = await crypto.subtle.digest('SHA-256', rustReopened.buffers[asset.bufferIndex]);
      const actual = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
      if (actual !== asset.sha256) throw new Error(`Rust reopened asset hash mismatch ${asset.sha256}`);
      reopenedHashes.push(actual);
    }
    const beforeInvalid = JSON.stringify(await reference.loadProject(document.id));
    let invalidRejected = false;
    try { await reference.unpackProject(new Uint8Array([1, 2, 3]), bridge); } catch { invalidRejected = true; }
    if (!invalidRejected || JSON.stringify(await reference.loadProject(document.id)) !== beforeInvalid) throw new Error('Invalid archive changed React document state');
    const databases = await indexedDB.databases();
    if (databases.some((database) => database.name === 'boardstudio-v2')) throw new Error('Unaliased production reference database was opened');
    return { projectId: document.id, name: document.name, revision: document.revision, parts: document.parts.length,
      rustAssetHashes: [...assetBytes.keys()].sort(), checkedAssets, referenceArchiveBytes: referenceArchive.byteLength,
      reopenedHashes: reopenedHashes.sort(), exactDocument: true, activePreference: true,
      invalidArchiveRejectedWithoutDocumentChange: true, aliasedDatabase: databases.map((database) => database.name) };
  }, attempt);
  assert.equal(result.name, 'REVIUNG41');
  assert.equal(result.revision, 5);
  assert.equal(result.parts, 85);
  assert.equal(result.checkedAssets.length, 4);
  assert.deepEqual(result.rustAssetHashes, result.reopenedHashes);
  report.cases.push(result);
  report.status = 'passed';
  await context.close();
} catch (error) {
  report.status = 'failed';
  report.error = String(error.stack ?? error);
  process.exitCode = 1;
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
  for (const file of sourceFiles) assert.equal(sha256(await fs.readFile(file)), sourceHashes[file], `Source changed during proof: ${file}`);
  await fs.writeFile(evidenceResult, JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
}
