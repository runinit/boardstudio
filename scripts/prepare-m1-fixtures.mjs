#!/usr/bin/env node
/** Copy accepted fixtures and prepare Sofle with the reference's public services. */
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createServer } from '../app/node_modules/vite/dist/node/index.js';

const root = fileURLToPath(new URL('../', import.meta.url));
const output = path.resolve(process.argv[2] ?? path.join(root, 'web/assets/fixtures'));
const require = createRequire(path.join(root, 'app/package.json'));
const { unzipSync } = require('fflate');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const coreBytes = await readFile(path.join(root, 'core/pkg/boardstudio_core_bg.wasm'));
const core = await import('../core/pkg/boardstudio_core.js');
core.initSync({ module: coreBytes });
const engine = new core.CoreEngine();
const server = await createServer({ root: path.join(root, 'app'), configFile: false,
  server: { middlewareMode: true, hmr: false }, appType: 'custom' });
try {
  await mkdir(path.dirname(output), { recursive: true });
  await mkdir(output);
  const archivePath = 'docs/design/evidence/board-outlines/reviung41-original.boardstudio';
  const archive = await readFile(path.join(root, archivePath));
  const unpacked = core.archive_request(JSON.stringify({ kind: 'unpack-project' }), [archive]);
  const reply = JSON.parse(unpacked[0]);
  if (reply.kind !== 'unpacked') throw new Error(JSON.stringify(reply));
  const reviung = JSON.parse(reply.projectJson);
  const assets = [];
  for (const entry of reply.assets) {
    const bytes = unpacked[1][entry.bufferIndex];
    if (hash(bytes) !== entry.sha256) throw new Error(`Unpack hash mismatch: ${entry.sha256}`);
    await writeFile(path.join(output, entry.sha256), bytes);
    assets.push({ sha256: entry.sha256, bytes: bytes.length });
  }
  await writeFile(path.join(output, 'reviung41.boardstudio'), archive);
  await writeFile(path.join(output, 'reviung41.json'), reply.projectJson);

  const { openSofleDemo } = await server.ssrLoadModule('/src/demos/sofle.ts');
  const { createMechanicalConfiguration } = await server.ssrLoadModule('/src/mechanicalPresets.ts');
  const request = async input => {
    if (input.kind === 'open') input.document.id = 'm1-sofle-v2-copy';
    const result = JSON.parse(engine.request(JSON.stringify(input)));
    if (result.kind === 'error') throw new Error(result.message);
    return result;
  };
  const opened = await openSofleDemo('v2', request);
  // Same public configuration defaults and edit path as the reference inspector.
  const configuration = { ...createMechanicalConfiguration(opened.document, 'left'),
    mount: 'gasket', integratedPlateFrame: false, bottomStyle: 'shell', gasketTravel: 0.3 };
  const saved = await request({ kind: 'edit', id: 'm1-sofle-gasket', command: {
    baseRevision: opened.document.revision, transactionId: 'm1-fixture-gasket', phase: 'commit',
    targetIds: [], operation: { kind: 'set-mechanical', configuration } } });
  if (saved.kind !== 'scene') throw new Error('Expected committed Sofle scene');
  // Model bytes are embedded using the same reference packProject service below.
  const { packProject } = await server.ssrLoadModule('/src/storage.ts');
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async (url, options) => {
    const location = String(url);
    if (location.startsWith('/@fs/')) return new Response(await readFile(location.slice(4).split('?')[0]));
    if (location.startsWith('/')) return new Response(await readFile(path.join(root, 'app', location.split('?')[0])));
    return originalFetch(url, options);
  };
  let sofleArchive;
  try {
    sofleArchive = await packProject(saved.document, {}, { archive: async input => {
      const outputs = core.archive_request(JSON.stringify(input.request), input.buffers);
      const result = JSON.parse(outputs[0]);
      if (result.kind !== 'packed') throw new Error(JSON.stringify(result));
      return { kind: 'archive', reply: { kind: 'packed', bytes: outputs[1][0] } };
    } });
  } finally { globalThis.fetch = originalFetch; }
  const sofleFiles = unzipSync(sofleArchive);
  const projectJson = new TextDecoder().decode(sofleFiles['project.json']);
  const sofle = JSON.parse(projectJson);
  const sofleAssets = [];
  for (const asset of sofle.assets) {
    const bytes = sofleFiles[`assets/${asset.sha256}`];
    if (!bytes || hash(bytes) !== asset.sha256) throw new Error(`Sofle asset missing or mismatched: ${asset.id}`);
    await writeFile(path.join(output, asset.sha256), bytes);
    sofleAssets.push({ sha256: asset.sha256, bytes: bytes.length });
  }
  await writeFile(path.join(output, 'sofle.json'), projectJson);
  await writeFile(path.join(output, 'sofle.boardstudio'), sofleArchive);
  const inputs = ['scripts/prepare-m1-fixtures.mjs', 'app/src/demos/sofle.ts', 'app/src/mechanicalPresets.ts', 'app/src/storage.ts', 'app/src/bundledModels.ts', 'core/Cargo.toml', 'core/Cargo.lock'];
  const sourceHashes = Object.fromEntries(await Promise.all(inputs.map(async file => [file, hash(await readFile(path.join(root, file)))])));
  const provenance = { schema: 1, source_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), source_hashes: sourceHashes, core_wasm_sha256: hash(coreBytes),
    preparation: 'Reference openSofleDemo v2 plus public set-mechanical gasket edit and reference packProject with embedded used models',
    fixtures: [ { name: 'REVIUNG41', source: archivePath, source_sha256: hash(archive),
      document_id: reviung.id, revision: reviung.revision, assets },
    { name: 'Sofle v2 gasket', source: 'app/src/demos/sofle.ts', document_id: sofle.id,
      revision: sofle.revision, archive_sha256: hash(sofleArchive), assets: sofleAssets } ] };
  await writeFile(path.join(output, 'provenance.json'), JSON.stringify(provenance, null, 2) + '\n');
  console.log(`Prepared copied fixtures and assets in ${output}`);
} finally {
  engine.free();
  await server.close();
}
