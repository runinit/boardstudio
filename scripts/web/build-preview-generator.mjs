// Package the existing Ergogen-to-KiCad preview producer as a private ESM worker graph.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { resolve, join } from 'node:path';

const root = fileURLToPath(new URL('../../', import.meta.url));
const destination = process.argv[2];
if (!destination) throw new Error('Usage: node scripts/web/build-preview-generator.mjs <assets-directory>');
const outputDirectory = join(resolve(destination), 'preview-generator');
await mkdir(outputDirectory, { recursive: true });

const workerSource = await readFile(join(root, 'scripts/web/preview-generator-worker.ts'), 'utf8');
const kicadSource = await readFile(join(root, 'kicad/src/ergogen.ts'), 'utf8');
const ergogenSource = await readFile(join(root, 'ergogen/src/index.ts'), 'utf8');
const catalogue = await readFile(join(root, 'ergogen/generated/catalogue.mjs'));

function rewriteImport(source, from, to, filename) {
  if (!source.includes(from)) throw new Error(`${filename} no longer contains expected import ${from}`);
  return source.replaceAll(from, to);
}

const workerModule = rewriteImport(
  rewriteImport(stripTypeScriptTypes(workerSource), "'../../ergogen/src/index.ts'", "'./ergogen.mjs'", 'preview-generator-worker.ts'),
  "'../../kicad/src/ergogen.ts'", "'./kicad.mjs'", 'preview-generator-worker.ts',
);
const kicadModule = rewriteImport(
  stripTypeScriptTypes(kicadSource), "'@boardstudio/v2-ergogen'", "'./ergogen.mjs'", 'kicad/src/ergogen.ts',
);
const ergogenModule = rewriteImport(
  stripTypeScriptTypes(ergogenSource), "'../generated/catalogue.mjs'", "'./catalogue.mjs'", 'ergogen/src/index.ts',
);

const files = {
  'worker.mjs': Buffer.from(workerModule),
  'kicad.mjs': Buffer.from(kicadModule),
  'ergogen.mjs': Buffer.from(ergogenModule),
  'catalogue.mjs': catalogue,
};
for (const [name, bytes] of Object.entries(files)) await writeFile(join(outputDirectory, name), bytes);
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
await writeFile(join(outputDirectory, 'provenance.json'), JSON.stringify({
  purpose: 'Private Ergogen preview job worker',
  node: process.version,
  sources: {
    'scripts/web/preview-generator-worker.ts': hash(Buffer.from(workerSource)),
    'kicad/src/ergogen.ts': hash(Buffer.from(kicadSource)),
    'ergogen/src/index.ts': hash(Buffer.from(ergogenSource)),
    'ergogen/generated/catalogue.mjs': hash(catalogue),
  },
  outputs: Object.fromEntries(Object.entries(files).map(([name, bytes]) => [name, hash(bytes)])),
}, null, 2) + '\n');
