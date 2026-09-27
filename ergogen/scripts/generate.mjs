import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { bindDefaults } from '../library/src/defaultModels.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const library = resolve(root, 'library');
const output = resolve(root, 'generated');
const sources = [
  ['ceoloide', library],
  ['infused-kim', resolve(library, 'vendor/infused-kim')],
];
const definitions = [];
for (const [namespace, directory] of sources) {
  for (const name of (await readdir(directory)).filter((file) => file.endsWith('.js')).sort()) {
    if (namespace === 'infused-kim' && ['choc.js', 'diode.js', 'nice_nano_pretty.js'].includes(name)) continue;
    const source = await readFile(resolve(directory, name), 'utf8');
    const id = `${namespace}/${name.slice(0, -3)}`;
    definitions.push(`  ${JSON.stringify(id)}: (() => { const module = { exports: {} }; let prop_name, local_nets, pad_cnt, label, row_traces;\n${bindDefaults(source, id)}\nreturn module.exports; })(),`);
  }
}
const content = `// Generated from the unchanged ergogen/library sources.\nexport default {\n${definitions.join('\n')}\n};\n`;
const target = resolve(output, 'catalogue.mjs');
if (process.argv.includes('--check')) {
  if (await readFile(target, 'utf8') !== content) throw new Error('Generated Ergogen catalogue is stale');
  console.log(`Verified generated catalogue for ${definitions.length} modules`);
} else {
  await mkdir(output, { recursive: true });
  await writeFile(target, content);
  console.log(`Generated ${definitions.length} bundled Ergogen modules`);
}
