import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { check } from './repo-check.mjs';

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), 'boardstudio-repo-check-'));
  await mkdir(path.join(root, 'app/src'), { recursive: true });
  await mkdir(path.join(root, 'app/scripts'), { recursive: true });
  await mkdir(path.join(root, 'docs/reference'), { recursive: true });
  return root;
}

async function cleanup(root) {
  await rm(root, { recursive: true, force: true });
}

test('follows workers, dynamic imports, and type-only imports while finding dead modules', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'boardstudio-repo-check-'));
  await mkdir(path.join(root, 'app/src'), { recursive: true });
  await mkdir(path.join(root, 'app/scripts'), { recursive: true });
  await writeFile(path.join(root, 'app/src/main.tsx'), "import './feature'; import('./lazy');\n");
  await writeFile(path.join(root, 'app/src/feature.ts'), "export const live = 1;\n");
  await writeFile(path.join(root, 'app/src/lazy.ts'), "import type { Thing } from './types'; export const loaded = 1;\n");
  await writeFile(path.join(root, 'app/src/types.ts'), 'export interface Thing { id: string }\n');
  await writeFile(path.join(root, 'app/src/core.worker.ts'), "new URL('./worker-child.ts', import.meta.url);\n");
  await writeFile(path.join(root, 'app/src/worker-child.ts'), 'export const worker = true;\n');
  await writeFile(path.join(root, 'app/src/dead.ts'), 'export const dead = true;\n');
  await writeFile(path.join(root, 'app/src/unrelated.ts'), 'const dead = false; export const unrelated = dead;\n');
  await writeFile(path.join(root, 'app/src/feature.test.ts'), 'export const fixture = true;\n');
  const result = await check(root);
  assert.ok(result.reachable.includes('app/src/worker-child.ts'));
  assert.ok(result.reachable.includes('app/src/types.ts'));
  assert.ok(result.issues.some((issue) => issue.kind === 'unreachable-module' && issue.file === 'app/src/dead.ts'));
  assert.ok(result.issues.some((issue) => issue.kind === 'unused-export' && issue.file === 'app/src/dead.ts'));
  assert.ok(!result.issues.some((issue) => issue.file === 'app/src/feature.test.ts'));
  await rm(root, { recursive: true, force: true });
});

test('reports missing explicit roots without treating package imports as local edges', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'boardstudio-repo-check-'));
  await mkdir(path.join(root, 'app/src'), { recursive: true });
  await writeFile(path.join(root, 'app/src/main.tsx'), "import React from 'react';\n");
  await writeFile(path.join(root, 'package.json'), JSON.stringify({ dependencies: { react: '1.0.0', unused: '1.0.0' } }));
  await writeFile(path.join(root, 'README.md'), '[missing](./does-not-exist.md)\n');
  const result = await check(root);
  assert.ok(result.issues.some((issue) => issue.kind === 'missing-entrypoint' && issue.file === 'app/src/core.worker.ts'));
  assert.equal(result.issues.some((issue) => issue.kind === 'missing-module' && issue.file === 'react'), false);
  assert.ok(result.issues.some((issue) => issue.kind === 'unused-dependency' && issue.message.includes('unused')));
  assert.ok(result.issues.some((issue) => issue.kind === 'broken-markdown-link'));
  await rm(root, { recursive: true, force: true });
});

test('tracks re-export chains, type imports, namespaces, defaults, and worker URLs', async () => {
  const root = await fixture();
  try {
    await writeFile(path.join(root, 'app/src/main.tsx'), [
      "import { publicValue } from './reexport';",
      "import * as namespace from './namespace';",
      "import React from 'react';",
      "const lazy = React.lazy(() => import('./lazy'));",
      'console.log(publicValue, namespace.value, lazy);',
    ].join('\n'));
    await writeFile(path.join(root, 'app/src/reexport.ts'), "export { publicValue } from './source';\n");
    await writeFile(path.join(root, 'app/src/source.ts'), 'export const publicValue = 1;\n');
    await writeFile(path.join(root, 'app/src/namespace.ts'), 'export const value = 2;\n');
    await writeFile(path.join(root, 'app/src/lazy.ts'), 'export default function LazyView() { return null; }\n');
    await writeFile(path.join(root, 'app/src/types.ts'), 'export interface UsedType { value: string }\nexport interface OrphanType { value: number }\n');
    await writeFile(path.join(root, 'app/src/type-user.ts'), "import type { UsedType } from './types';\nexport const typeValue = null as UsedType | null;\n");
    await writeFile(path.join(root, 'app/src/core.worker.ts'), "new Worker(new URL('./worker-child.ts', import.meta.url));\n");
    await writeFile(path.join(root, 'app/src/worker-child.ts'), 'export const workerChild = true;\n');
    const result = await check(root);
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-export' && issue.file === 'app/src/source.ts'));
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-export' && issue.file === 'app/src/namespace.ts'));
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-export' && issue.file === 'app/src/lazy.ts'));
    assert.ok(!result.issues.some((issue) => issue.kind === 'unused-export' && issue.message.includes('UsedType')));
    assert.ok(result.issues.some((issue) => issue.kind === 'unused-export' && issue.message.includes('OrphanType')));
    assert.ok(result.reachable.includes('app/src/worker-child.ts'));
  } finally {
    await cleanup(root);
  }
});

test('scopes dependency usage to the package and distinguishes side effects from comments', async () => {
  const root = await fixture();
  try {
    await writeFile(path.join(root, 'package.json'), JSON.stringify({ dependencies: { 'root-used': '1.0.0', 'root-unused': '1.0.0' } }));
    await writeFile(path.join(root, 'app/package.json'), JSON.stringify({ dependencies: { 'side-effect-lib': '1.0.0', 'comment-only-lib': '1.0.0' } }));
    await writeFile(path.join(root, 'app/src/main.tsx'), "import 'side-effect-lib';\n// import 'comment-only-lib';\n");
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
