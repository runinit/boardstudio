import { createHash } from 'node:crypto';
import { lstat, readFile, readdir, writeFile } from 'node:fs/promises';
import { basename, dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const evidenceDir = dirname(fileURLToPath(import.meta.url));
const distDir = resolve(process.env.BOARDSTUDIO_REFERENCE_DIST ?? '/home/chris/01_Projects/ts-boardstudio2/app/dist');
const outputPath = resolve(process.env.BOARDSTUDIO_REFERENCE_DIST_RECORD ?? `${evidenceDir}/reference-app-dist.json`);
const assetHashes = {};

async function walk(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = resolve(dir, entry.name);
    if (entry.isDirectory()) await walk(path);
    else {
      const stat = await lstat(path);
      if (!stat.isFile()) throw new Error(`Reference distribution contains a non-regular file: ${path}`);
      assetHashes[relative(distDir, path).split('\\').join('/')] = createHash('sha256').update(await readFile(path)).digest('hex');
    }
  }
}

await walk(distDir);
if (!assetHashes['index.html']) throw new Error(`Missing index.html in ${distDir}`);
const ordered = Object.fromEntries(Object.entries(assetHashes).sort(([a], [b]) => a.localeCompare(b)));
const distTreeSha256 = createHash('sha256').update(JSON.stringify(ordered)).digest('hex');
const record = {
  kind: 'unchanged React reference app/dist static distribution',
  distributionPath: distDir,
  distributionName: basename(distDir),
  recordedAt: new Date().toISOString(),
  fileCount: Object.keys(ordered).length,
  distTreeSha256,
  assetHashes: ordered,
  sourceCommit: null,
  sourceCommitNote: 'app/dist is a generated ignored distribution; this record pins every served file byte by SHA-256.',
};
await writeFile(outputPath, `${JSON.stringify(record, null, 2)}\n`, { flag: 'wx' });
console.log(JSON.stringify({ outputPath, distDir, fileCount: record.fileCount, distTreeSha256 }, null, 2));
