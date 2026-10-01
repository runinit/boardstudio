import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
const build = path.resolve(process.argv[2] ?? '');
if (!process.argv[2]) throw new Error('usage: node inventory-release.mjs <web/target/builds/<build-id>> [output.json]');
const output = path.resolve(process.argv[3] ?? path.join(path.dirname(fileURLToPath(import.meta.url)), 'release-inventory.json'));
const provenance = JSON.parse(await fs.readFile(path.join(build, 'provenance.json'), 'utf8'));
assert(provenance.source_commit, 'release provenance has no source_commit');
assert(provenance.root?.site && provenance.subpath?.site, 'release provenance must identify root and subpath sites');

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
async function inventory(site) {
  const base = path.resolve(site);
  const files = [];
  async function visit(dir) {
    for (const entry of await fs.readdir(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name);
      if (entry.isDirectory()) await visit(file);
      else if (entry.isFile()) {
        const bytes = await fs.readFile(file);
        files.push({ path: path.relative(base, file).split(path.sep).join('/'), bytes: bytes.byteLength, sha256: hash(bytes) });
      }
    }
  }
  await visit(base);
  files.sort((a, b) => a.path.localeCompare(b.path));
  const byPath = new Map(files.map((file) => [file.path, file]));
  for (const required of ['index.html', 'service-worker.js', 'boardstudio_offline_worker.js']) {
    assert(byPath.has(required), `${base} is missing required release file ${required}`);
  }
  const fixtures = files.filter((file) => file.path.startsWith('assets/fixtures/'));
  assert(fixtures.some((file) => file.path.endsWith('reviung41.boardstudio')), `${base} has no REVIUNG41 fixture`);
  assert(fixtures.some((file) => file.path.endsWith('sofle.boardstudio')), `${base} has no Sofle fixture`);
  return {
    site: base,
    fileCount: files.length,
    totalBytes: files.reduce((sum, file) => sum + file.bytes, 0),
    startupCandidates: files.filter((file) => file.path === 'index.html' || /\.(js|css|wasm)$/.test(file.path)),
    largestFiles: [...files].sort((a, b) => b.bytes - a.bytes).slice(0, 20),
    fixtureCount: fixtures.length,
    files,
  };
}

const root = await inventory(provenance.root.site);
const subpath = await inventory(provenance.subpath.site);
const rootFiles = new Map(root.files.map((file) => [file.path, file.sha256]));
const subpathFiles = new Map(subpath.files.map((file) => [file.path, file.sha256]));
const sharedPaths = [...rootFiles.keys()].filter((file) => subpathFiles.has(file));
const sharedDifferences = sharedPaths.filter((file) => rootFiles.get(file) !== subpathFiles.get(file));
const report = {
  buildId: path.basename(build),
  sourceCommit: provenance.source_commit,
  buildScope: provenance.scope ?? null,
  generatedAt: new Date().toISOString(),
  root,
  subpath,
  comparison: {
    commonFileCount: sharedPaths.length,
    differingCommonPaths: sharedDifferences,
    rootOnlyFiles: [...rootFiles.keys()].filter((file) => !subpathFiles.has(file)),
    subpathOnlyFiles: [...subpathFiles.keys()].filter((file) => !rootFiles.has(file)),
  },
  note: 'Static byte inventory. Browser performance resource timing is collected separately after the first online startup of each scope.',
};
await fs.mkdir(path.dirname(output), { recursive: true });
await fs.writeFile(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ output, buildId: report.buildId, sourceCommit: report.sourceCommit,
  rootFiles: root.fileCount, rootBytes: root.totalBytes, subpathFiles: subpath.fileCount,
  subpathBytes: subpath.totalBytes, differingCommonFiles: sharedDifferences.length }, null, 2));
