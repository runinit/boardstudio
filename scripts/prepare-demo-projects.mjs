#!/usr/bin/env node
/** Copy accepted fixtures and prepare keyboard demos with the reference's public services. */
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir, mkdtemp, rm, rename } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createServer } from 'vite';
import { unzipSync } from 'fflate';

const root = fileURLToPath(new URL('../', import.meta.url));
const output = path.resolve(process.argv[2] ?? path.join(root, 'web/assets/fixtures'));
await mkdir(path.dirname(output), { recursive: true });
const stagingOutput = await mkdtemp(path.join(path.dirname(output), `.${path.basename(output)}-`));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const coreBytes = await readFile(path.join(root, 'core/pkg/boardstudio_core_bg.wasm'));
const core = await import('../core/pkg/boardstudio_core.js');
core.initSync({ module: coreBytes });
const engine = new core.CoreEngine();
const server = await createServer({ root: path.join(root, 'tooling/demo-projects'), configFile: false,
  server: { middlewareMode: true, hmr: false }, appType: 'custom' });
let generated = false;
try {

  const archivePath = 'content/archives/reviung41-original.boardstudio';
  const archive = await readFile(path.join(root, archivePath));
  const unpacked = core.archive_request(JSON.stringify({ kind: 'unpack-project' }), [archive]);
  const reply = JSON.parse(unpacked[0]);
  if (reply.kind !== 'unpacked') throw new Error(JSON.stringify(reply));
  const reviung = JSON.parse(reply.projectJson);
  const assets = [];
  for (const entry of reply.assets) {
    const bytes = unpacked[1][entry.bufferIndex];
    if (hash(bytes) !== entry.sha256) throw new Error(`Unpack hash mismatch: ${entry.sha256}`);
    await writeFile(path.join(stagingOutput, entry.sha256), bytes);
    assets.push({ sha256: entry.sha256, bytes: bytes.length });
  }
  await writeFile(path.join(stagingOutput, 'reviung41.boardstudio'), archive);
  await writeFile(path.join(stagingOutput, 'reviung41.json'), reply.projectJson);

  const { openSofleDemo, sofleDemos } = await server.ssrLoadModule('/src/demos/sofle.ts');
  const { keyboardDemos, openKeyboardDemo } = await server.ssrLoadModule('/src/demos/keyboards.ts');
  const { createMechanicalConfiguration } = await server.ssrLoadModule('/src/mechanicalPresets.ts');
  const request = async input => {
    if (input.kind === 'open') input.document.id = `m1-${input.document.parameters.demo}-copy`;
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
  // Model bytes are embedded through the reference packProject service.
  const { packProject } = await server.ssrLoadModule('/src/archive.ts');
  const packWithUsedModels = async document => {
    const originalFetch = globalThis.fetch;
    globalThis.fetch = async url => {
      const location = String(url);
      if (location.startsWith('/')) {
        const pathname = decodeURIComponent(new URL(location, 'http://fixture.local').pathname);
        const filename = pathname.startsWith('/@fs/')
          ? pathname.slice(4)
          : path.join(root, 'tooling/demo-projects', pathname);
        return new Response(await readFile(filename));
      }
      return originalFetch(url);
    };
    try {
      return await packProject(document, {}, { archive: async input => {
        const outputs = core.archive_request(JSON.stringify(input.request), input.buffers);
        const result = JSON.parse(outputs[0]);
        if (result.kind !== 'packed') throw new Error(JSON.stringify(result));
        return { kind: 'archive', reply: { kind: 'packed', bytes: outputs[1][0] } };
      } });
    } finally { globalThis.fetch = originalFetch; }
  };
  const sofleArchive = await packWithUsedModels(saved.document);
  const sofleFiles = unzipSync(sofleArchive);
  const projectJson = new TextDecoder().decode(sofleFiles['project.json']);
  const sofle = JSON.parse(projectJson);
  const sofleAssets = [];
  for (const asset of sofle.assets) {
    const bytes = sofleFiles[`assets/${asset.sha256}`];
    if (!bytes || hash(bytes) !== asset.sha256) throw new Error(`Sofle asset missing or mismatched: ${asset.id}`);
    await writeFile(path.join(stagingOutput, asset.sha256), bytes);
    sofleAssets.push({ sha256: asset.sha256, bytes: bytes.length });
  }
  await writeFile(path.join(stagingOutput, 'sofle.json'), projectJson);
  await writeFile(path.join(stagingOutput, 'sofle.boardstudio'), sofleArchive);
  const sofleVariantFixtures = [];
  for (const variant of ['rgb', 'choc']) {
    const variantDemo = sofleDemos.find(demo => demo.id === variant);
    if (!variantDemo) throw new Error(`Missing Sofle variant: ${variant}`);
    const variantOpened = await openSofleDemo(variant, request);
    const variantArchive = await packWithUsedModels(variantOpened.document);
    const variantFiles = unzipSync(variantArchive);
    const variantJson = new TextDecoder().decode(variantFiles['project.json']);
    const variantDocument = JSON.parse(variantJson);
    const variantAssets = [];
    for (const asset of variantDocument.assets) {
      const bytes = variantFiles[`assets/${asset.sha256}`];
      if (!bytes || hash(bytes) !== asset.sha256) throw new Error(`${variantDemo.name} asset missing or mismatched: ${asset.id}`);
      await writeFile(path.join(stagingOutput, asset.sha256), bytes);
      variantAssets.push({ sha256: asset.sha256, bytes: bytes.length });
    }
    const filename = `sofle-${variant}`;
    await writeFile(path.join(stagingOutput, `${filename}.json`), variantJson);
    await writeFile(path.join(stagingOutput, `${filename}.boardstudio`), variantArchive);
    sofleVariantFixtures.push({ name: variantDemo.name, variant, source: 'tooling/demo-projects/src/demos/sofle.ts',
      document_id: variantDocument.id, revision: variantDocument.revision,
      archive_sha256: hash(variantArchive), assets: variantAssets });
  }
  const { openModuleReviewDemo } = await server.ssrLoadModule('/src/demos/moduleReview.ts');
  const moduleReview = await openModuleReviewDemo(request);
  const moduleReviewArchive = await packWithUsedModels(moduleReview.document);
  const moduleReviewFiles = unzipSync(moduleReviewArchive);
  const moduleReviewJson = new TextDecoder().decode(moduleReviewFiles['project.json']);
  const moduleReviewDocument = JSON.parse(moduleReviewJson);
  const moduleReviewAssets = [];
  for (const asset of moduleReviewDocument.assets) {
    const bytes = moduleReviewFiles[`assets/${asset.sha256}`];
    if (!bytes || hash(bytes) !== asset.sha256) throw new Error(`VIK module review asset missing or mismatched: ${asset.id}`);
    await writeFile(path.join(stagingOutput, asset.sha256), bytes);
    moduleReviewAssets.push({ sha256: asset.sha256, bytes: bytes.length });
  }
  await writeFile(path.join(stagingOutput, 'vik-module-review.json'), moduleReviewJson);
  await writeFile(path.join(stagingOutput, 'vik-module-review.boardstudio'), moduleReviewArchive);
  const measuredFixtures = [];
  for (const demo of keyboardDemos) {
    const openedDemo = await openKeyboardDemo(demo.id, request);
    const demoArchive = await packWithUsedModels(openedDemo.document);
    const demoFiles = unzipSync(demoArchive);
    const demoJson = new TextDecoder().decode(demoFiles['project.json']);
    const demoDocument = JSON.parse(demoJson);
    const fixture = `measured-${demo.id}`;
    const demoAssets = [];
    for (const asset of demoDocument.assets) {
      const bytes = demoFiles[`assets/${asset.sha256}`];
      if (!bytes || hash(bytes) !== asset.sha256) throw new Error(`${demo.name} asset missing or mismatched: ${asset.id}`);
      await writeFile(path.join(stagingOutput, asset.sha256), bytes);
      demoAssets.push({ sha256: asset.sha256, bytes: bytes.length });
    }
    await writeFile(path.join(stagingOutput, `${fixture}.json`), demoJson);
    await writeFile(path.join(stagingOutput, `${fixture}.boardstudio`), demoArchive);
    measuredFixtures.push({ name: demo.name, fixture, source: 'tooling/demo-projects/src/demos/keyboard-layouts.json', demo_source: 'tooling/demo-projects/src/demos/keyboards.ts',
      id: demo.id, document_id: demoDocument.id, revision: demoDocument.revision,
      boards: demoDocument.boards.map(board => ({ id: board.id, name: board.name, part_count: board.partIds.length })),
      key_count: demoDocument.matrices.reduce((count, matrix) => count + matrix.cells.filter(cell => cell.enabled).length, 0),
      archive_sha256: hash(demoArchive), assets: demoAssets });
  }
  const inputs = ['scripts/prepare-demo-projects.mjs', 'tooling/demo-projects/src/demos/sofle.ts', 'tooling/demo-projects/src/demos/sofle-layouts.json', 'tooling/demo-projects/src/demos/keyboards.ts', 'tooling/demo-projects/src/demos/keyboard-layouts.json', 'tooling/demo-projects/src/demos/moduleReview.ts', 'tooling/demo-projects/src/demo.ts', 'catalogue/modules/imported-modules.json', 'tooling/demo-projects/src/modules/hostConnector.ts', 'catalogue/parts/imported-parts.json', 'tooling/demo-projects/src/demos/physicalLayout.ts', 'tooling/demo-projects/src/mechanicalPresets.ts', 'tooling/demo-projects/src/archive.ts', 'tooling/demo-projects/src/bundledModels.ts', 'core/Cargo.toml', 'core/Cargo.lock'];
  const sourceHashes = Object.fromEntries(await Promise.all(inputs.map(async file => [file, hash(await readFile(path.join(root, file)))])));
  const provenance = { schema: 1, source_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), source_hashes: sourceHashes, core_wasm_sha256: hash(coreBytes),
    preparation: 'Reference openSofleDemo for v2/RGB/Choc, openKeyboardDemo for measured layouts with the existing electrical resolver, and openModuleReviewDemo for VIK; public set-mechanical gasket edit for v2; reference packProject with embedded used models',
    fixtures: [
      { name: 'REVIUNG41', source: archivePath, source_sha256: hash(archive),
        document_id: reviung.id, revision: reviung.revision, assets },
      { name: 'Sofle v2 gasket', source: 'tooling/demo-projects/src/demos/sofle.ts', document_id: sofle.id,
        variant: 'v2', revision: sofle.revision, archive_sha256: hash(sofleArchive), assets: sofleAssets },
      ...sofleVariantFixtures,
      ...measuredFixtures,
      { name: 'VIK module review · above and below', source: 'tooling/demo-projects/src/demos/moduleReview.ts',
        document_id: moduleReviewDocument.id, revision: moduleReviewDocument.revision,
        archive_sha256: hash(moduleReviewArchive), assets: moduleReviewAssets,
        modules: moduleReviewDocument.modules.map(module => ({ id: module.id, definition_id: module.definitionId,
          host_face: module.hostFace, facing_face: module.facingFace, rotation: module.rotation })) },
    ] };
  await writeFile(path.join(stagingOutput, 'provenance.json'), JSON.stringify(provenance, null, 2) + '\n');
  console.log(`Prepared copied fixtures and assets in ${output}`);
  generated = true;
} finally {
  engine.free();
  await server.close();
  if (generated) {
    await rm(output, { recursive: true, force: true });
    await rename(stagingOutput, output);
  } else {
    await rm(stagingOutput, { recursive: true, force: true });
  }
}
