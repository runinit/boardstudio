import { spawnSync } from 'node:child_process';
import { cp, mkdtemp, readdir, readFile, rm, writeFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const generated = path.join(root, 'contracts/src/generated');
const mode = process.argv[2];
if (mode !== 'generate' && mode !== 'check') throw new Error('Expected generate or check');

async function filesUnder(directory, prefix = '') {
  try {
    const entries = await readdir(directory, { withFileTypes: true });
    return (await Promise.all(entries.map(async (entry) => {
      const full = path.join(directory, entry.name);
      const relative = path.join(prefix, entry.name);
      return entry.isDirectory()
        ? filesUnder(full, relative)
        : [relative];
    }))).flat().sort();
  } catch (error) {
    if (error.code === 'ENOENT') return [];
    throw error;
  }
}

function run(output) {
  const result = spawnSync('cargo', ['run', '--quiet', '--manifest-path', 'core/Cargo.toml', '--example', 'export_contracts', '--features', 'export-types'], {
    cwd: root,
    env: { ...process.env, CONTRACTS_OUT_DIR: output },
    stdio: 'inherit',
  });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function barrel(types) {
  return `${types
    .filter((name) => !['CoreRequest.ts', 'CoreReply.ts', 'keycapDefaults.ts'].includes(path.basename(name)))
    .map((name) => `export type * from './${name.replace(/\.ts$/, '')}';`)
    .join('\n')}\n`;
}

const temporary = await mkdtemp(path.join(tmpdir(), 'boardstudio-contracts-'));
try {
  await mkdir(temporary, { recursive: true });
  run(temporary);
  const declarationFiles = await filesUnder(temporary);
  const generatedTypes = declarationFiles.filter((name) => name.endsWith('.ts') && name !== 'index.ts');
  for (const name of generatedTypes) {
    const filename = path.join(temporary, name);
    const content = await readFile(filename, 'utf8');
    const normalized = content.replace(/[ \t]+$/gm, '');
    if (normalized !== content) await writeFile(filename, normalized);
  }
  await mkdir(path.join(temporary, 'serde_json'), { recursive: true });
  const jsonValue = 'export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };\n';
  await writeFile(path.join(temporary, 'serde_json/JsonValue.ts'), jsonValue);
  const types = [...generatedTypes, 'serde_json/JsonValue.ts'].sort();
  const index = barrel(types);
  if (mode === 'generate') {
    await writeFile(path.join(temporary, 'index.ts'), index);
    await rm(generated, { recursive: true, force: true });
    await cp(temporary, generated, { recursive: true });
    await rm(temporary, { recursive: true, force: true });
    process.exit(0);
  }

  const expected = await filesUnder(generated);
  const actual = [...types, 'index.ts'].sort();
  const issues = [];
  for (const name of [...new Set([...expected, ...actual])]) {
    if (!expected.includes(name)) issues.push(`unexpected generated file: ${name}`);
    else if (!actual.includes(name)) issues.push(`missing generated file: ${name}`);
    else {
      const actualContent = name === 'index.ts' ? index : await readFile(path.join(temporary, name), 'utf8');
      const expectedContent = await readFile(path.join(generated, name), 'utf8');
      if (actualContent !== expectedContent) issues.push(`generated content differs: ${name}`);
    }
  }
  if (issues.length) {
    console.error(issues.join('\n'));
    process.exitCode = 1;
  }
} finally {
  await rm(temporary, { recursive: true, force: true });
}
