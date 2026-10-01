import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '../../../..');
const build = path.resolve(process.argv[2] ?? '');
const qaVersion = process.argv[3];
if (!process.argv[2] || !qaVersion || !/^[A-Za-z0-9._-]+$/.test(qaVersion)) {
  throw new Error('usage: node prepare-root-update-worker.mjs <build-output> <unique-update-version>');
}
const provenance = JSON.parse(await fs.readFile(path.join(build, 'provenance.json'), 'utf8'));
const root = path.resolve(provenance.root.site);
const output = path.join(build, 'qa-update-root-worker');
await fs.mkdir(output, { recursive: false });
const sourceManifest = JSON.parse(await fs.readFile(path.join(build, 'offline-manifest-root.json'), 'utf8'));
const manifest = { ...sourceManifest, version: qaVersion };
const manifestPath = path.join(output, 'offline-manifest.json');
await fs.writeFile(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
for (const asset of manifest.assets) {
  await fs.access(path.join(root, asset));
}

const commands = [];
async function run(name, argv, extraEnv = {}) {
  const started = new Date().toISOString();
  try {
    const stdout = execFileSync(argv[0], argv.slice(1), { cwd: repo,
      env: { ...process.env, ...extraEnv }, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
    await fs.writeFile(path.join(output, `${name}.log`), stdout);
    commands.push({ name, argv, exit: 0, log: `${name}.log`, started, finished: new Date().toISOString() });
    return stdout;
  } catch (error) {
    const log = `${String(error.stdout ?? '')}\n${String(error.stderr ?? '')}`;
    await fs.writeFile(path.join(output, `${name}.log`), log);
    commands.push({ name, argv, exit: error.status ?? 1, log: `${name}.log`, started, finished: new Date().toISOString() });
    throw error;
  }
}

const packageDirectory = path.join(output, 'pkg');
await run('offline-update-worker', ['wasm-pack', 'build', path.join(repo, 'web'), '--target', 'web', '--out-name', 'boardstudio_offline_worker', '--out-dir', packageDirectory, '--release', '--locked', '--no-default-features', '--features', 'service-worker'], { BOARDSTUDIO_OFFLINE_MANIFEST: manifestPath });
await run('embed-update-worker', ['node', path.join(repo, 'scripts/web/embed-worker-wasm.mjs'), packageDirectory, manifestPath, path.join(output, 'service-worker.js')]);

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const updateBootstrap = await fs.readFile(path.join(output, 'service-worker.js'));
const releaseBootstrap = await fs.readFile(path.join(root, 'service-worker.js'));
assert.notEqual(hash(updateBootstrap), hash(releaseBootstrap), 'update worker bootstrap did not change');
const report = { buildId: path.basename(build), sourceCommit: provenance.source_commit, scope: '/', releaseVersion: sourceManifest.version,
  updateVersion: qaVersion, assets: manifest.assets.length, serviceWorkerSha256: hash(updateBootstrap),
  workerModuleSha256: hash(await fs.readFile(path.join(output, 'boardstudio_offline_worker.js'))), commands };
await fs.writeFile(path.join(output, 'provenance.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ output, ...report }, null, 2));
