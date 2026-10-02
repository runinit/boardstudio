// Package the existing catalogue service; no React or UI code is included.
import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const root = fileURLToPath(new URL('../../', import.meta.url));
const destination = process.argv[2];
if (!destination) throw new Error('Usage: node scripts/web/build-layout-generators.mjs <assets-directory>');
execFileSync(process.execPath, ['ergogen/scripts/generate.mjs', '--check'], { cwd: root, stdio: 'inherit' });
const assets = resolve(destination);
const input = await readFile(join(root, 'ergogen/src/index.ts'), 'utf8');
const catalogue = await readFile(join(root, 'ergogen/generated/catalogue.mjs'));
const output = stripTypeScriptTypes(input);
const entrypoint = "export { isErgogen, modelAssetIdsForPaths, parameters, render } from './layout-generators/src/index.js';\n";
await mkdir(join(assets, 'layout-generators/src'), { recursive: true });
await mkdir(join(assets, 'layout-generators/generated'), { recursive: true });
await writeFile(join(assets, 'layout-generators/src/index.js'), output);
await copyFile(join(root, 'ergogen/generated/catalogue.mjs'), join(assets, 'layout-generators/generated/catalogue.mjs'));
await writeFile(join(assets, 'layout-generators.js'), entrypoint);
const service = await import(pathToFileURL(join(assets, 'layout-generators.js')).href);
const mxDefaults = service.parameters?.('ceoloide/switch_mx');
if (typeof service.isErgogen !== 'function' || typeof service.render !== 'function'
  || typeof service.modelAssetIdsForPaths !== 'function'
  || typeof service.parameters !== 'function' || !service.isErgogen('ceoloide/switch_mx')
  || mxDefaults?.keycap_width?.value !== 18 || mxDefaults?.keycap_height?.value !== 18
  || mxDefaults?.include_keycap?.value !== true) {
  throw new Error('Packaged generator service must expose isErgogen/modelAssetIdsForPaths/render/parameters and the MX preview defaults');
}
console.log('Verified packaged generator service and ceoloide/switch_mx preview defaults');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
await writeFile(join(assets, 'layout-generators/provenance.json'), JSON.stringify({
  purpose: 'Retained catalogue generator service for Dioxus Layout drawings and Parts envelope defaults',
  node: process.version,
  sources: { 'ergogen/src/index.ts': hash(input), 'ergogen/generated/catalogue.mjs': hash(catalogue) },
  outputs: { 'layout-generators.js': hash(entrypoint), 'src/index.js': hash(output), 'generated/catalogue.mjs': hash(catalogue) },
}, null, 2) + '\n');
