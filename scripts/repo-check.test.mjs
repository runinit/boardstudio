import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { check as checkRepository } from './repo-check.mjs';
const check = root => checkRepository(root, { packages: ['example', 'kicad'], entrypoints: ['example/src/main.ts', 'example/src/worker.ts'], publicFacades: new Map() });

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), 'boardstudio-repo-check-'));
  await mkdir(path.join(root, 'example/src'), { recursive: true });
  await mkdir(path.join(root, 'example/scripts'), { recursive: true });
  await mkdir(path.join(root, 'docs/reference'), { recursive: true });
  return root;
}

async function cleanup(root) {
  await rm(root, { recursive: true, force: true });
}

test('follows workers, dynamic imports, and type-only imports while finding dead modules', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'boardstudio-repo-check-'));
  await mkdir(path.join(root, 'example/src'), { recursive: true });
  await mkdir(path.join(root, 'example/scripts'), { recursive: true });
  await writeFile(path.join(root, 'example/src/main.ts'), "import './feature'; import('./lazy');\n");
  await writeFile(path.join(root, 'example/src/feature.ts'), "export const live = 1;\n");
  await writeFile(path.join(root, 'example/src/lazy.ts'), "import type { Thing } from './types'; export const loaded = 1;\n");
  await writeFile(path.join(root, 'example/src/types.ts'), 'export interface Thing { id: string }\n');
  await writeFile(path.join(root, 'example/src/worker.ts'), "new URL('./worker-child.ts', import.meta.url);\n");
  await writeFile(path.join(root, 'example/src/worker-child.ts'), 'export const worker = true;\n');
  await writeFile(path.join(root, 'example/src/dead.ts'), 'export const dead = true;\n');
  await writeFile(path.join(root, 'example/src/unrelated.ts'), 'const dead = false; export const unrelated = dead;\n');
  await writeFile(path.join(root, 'example/src/feature.test.ts'), 'export const fixture = true;\n');
  const result = await check(root);
  assert.ok(result.reachable.includes('example/src/worker-child.ts'));
  assert.ok(result.reachable.includes('example/src/types.ts'));
  assert.ok(result.issues.some((issue) => issue.kind === 'unreachable-module' && issue.file === 'example/src/dead.ts'));
  assert.ok(result.issues.some((issue) => issue.kind === 'unused-export' && issue.file === 'example/src/dead.ts'));
  assert.ok(!result.issues.some((issue) => issue.file === 'example/src/feature.test.ts'));
  await rm(root, { recursive: true, force: true });
});

test('reports missing explicit roots without treating package imports as local edges', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'boardstudio-repo-check-'));
  await mkdir(path.join(root, 'example/src'), { recursive: true });
  await writeFile(path.join(root, 'example/src/main.ts'), "import external from 'external';\n");
  await writeFile(path.join(root, 'package.json'), JSON.stringify({ dependencies: { external: '1.0.0', unused: '1.0.0' } }));
  await writeFile(path.join(root, 'README.md'), '[missing](./does-not-exist.md)\n');
  const result = await check(root);
  assert.ok(result.issues.some((issue) => issue.kind === 'missing-entrypoint' && issue.file === 'example/src/worker.ts'));
  assert.equal(result.issues.some((issue) => issue.kind === 'missing-module' && issue.file === 'external'), false);
  assert.ok(result.issues.some((issue) => issue.kind === 'unused-dependency' && issue.message.includes('unused')));
  assert.ok(result.issues.some((issue) => issue.kind === 'broken-markdown-link'));
  await rm(root, { recursive: true, force: true });
});

test('tracks re-export chains, type imports, namespaces, defaults, and worker URLs', async () => {
  const root = await fixture();
  try {
    await writeFile(path.join(root, 'example/src/main.ts'), [
      "import { publicValue } from './reexport';",
      "import * as namespace from './namespace';",
      "import external from 'external';",
      "const lazy = import('./lazy');",
      'console.log(publicValue, namespace.value, lazy);',
    ].join('\n'));
    await writeFile(path.join(root, 'example/src/reexport.ts'), "export { publicValue } from './source';\n");
    await writeFile(path.join(root, 'example/src/source.ts'), 'export const publicValue = 1;\n');
    await writeFile(path.join(root, 'example/src/namespace.ts'), 'export const value = 2;\n');
    await writeFile(path.join(root, 'example/src/lazy.ts'), 'export default function LazyView() { return null; }\n');
    await writeFile(path.join(root, 'example/src/types.ts'), 'export interface UsedType { value: string }\nexport interface OrphanType { value: number }\n');
    await writeFile(path.join(root, 'example/src/type-user.ts'), "import type { UsedType } from './types';\nexport const typeValue = null as UsedType | null;\n");
    await writeFile(path.join(root, 'example/src/worker.ts'), "new Worker(new URL('./worker-child.ts', import.meta.url));\n");
    await writeFile(path.join(root, 'example/src/worker-child.ts'), 'export const workerChild = true;\n');
    const result = await check(root);
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-export' && issue.file === 'example/src/source.ts'));
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-export' && issue.file === 'example/src/namespace.ts'));
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-export' && issue.file === 'example/src/lazy.ts'));
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-export' && issue.message.includes('UsedType')));
    assert.ok(result.issues.some((issue) => issue.kind === 'unused-export' && issue.message.includes('OrphanType')));
    assert.ok(result.reachable.includes('example/src/worker-child.ts'));
  } finally {
    await cleanup(root);
  }
});

test('scopes dependency usage to the package and distinguishes side effects from comments', async () => {
  const root = await fixture();
  try {
    await writeFile(path.join(root, 'package.json'), JSON.stringify({ dependencies: { 'root-used': '1.0.0', 'root-unused': '1.0.0' } }));
    await writeFile(path.join(root, 'example/package.json'), JSON.stringify({ dependencies: { 'side-effect-lib': '1.0.0', 'comment-only-lib': '1.0.0' } }));
    await writeFile(path.join(root, 'example/src/main.ts'), "import 'side-effect-lib';\n// import 'comment-only-lib';\n");
    await mkdir(path.join(root, 'kicad/src'), { recursive: true });
    await writeFile(path.join(root, 'kicad/src/index.ts'), "import 'root-used';\n");
    const result = await check(root);
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-dependency' && issue.message.includes('side-effect-lib')));
    assert.ok(result.issues.some((issue) => issue.kind === 'unused-dependency' && issue.message.includes('comment-only-lib')));
    assert.ok(result.issues.some((issue) => issue.kind === 'unused-dependency' && issue.message.includes('root-unused')));
    assert.ok(result.issues.some((issue) => issue.kind === 'unused-dependency' && issue.message.includes('root-used')));
  } finally {
    await cleanup(root);
  }
});

test('checks valid Markdown directories and reports missing local paths', async () => {
  const root = await fixture();
  try {
    await writeFile(path.join(root, 'README.md'), '[docs](./docs/) [missing](./docs/nope.md)\n');
    const result = await check(root);
    assert.ok(result.issues.some((issue) => issue.kind === 'broken-markdown-link' && issue.message.includes('nope.md')));
    assert.ok(!result.issues.some((issue) => issue.kind === 'broken-markdown-link' && issue.message.endsWith('./docs/')));
  } finally {
    await cleanup(root);
  }
});
