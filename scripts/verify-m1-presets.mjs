#!/usr/bin/env node
/** Compare maintained Rust presets with the reference policy on copied fixtures. */
import { createServer } from '../app/node_modules/vite/dist/node/index.js';
import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import assert from 'node:assert/strict';

const root = fileURLToPath(new URL('../', import.meta.url));
const fixtures = process.argv[2];
if (!fixtures) throw new Error('usage: node scripts/verify-m1-presets.mjs <copied-fixtures-directory>');
execFileSync('cargo', ['build', '--manifest-path', 'web/Cargo.toml', '--locked', '--example', 'case-settings'], { cwd: root, stdio: 'inherit' });
const server = await createServer({ root: path.join(root, 'app'), configFile: false, server: { middlewareMode: true, hmr: false }, appType: 'custom' });
try {
  const { createMechanicalConfiguration } = await server.ssrLoadModule('/src/mechanicalPresets.ts');
  let count = 0;
  for (const name of ['reviung41', 'sofle']) {
    const original = JSON.parse(await readFile(path.join(fixtures, `${name}.json`), 'utf8'));
    for (const board of original.boards) {
      for (const keepExisting of [true, false]) {
        const document = structuredClone(original);
        if (!keepExisting) delete document.mechanical;
        const expected = document.mechanical?.boardId === board.id ? document.mechanical : createMechanicalConfiguration(document, board.id);
        const actual = JSON.parse(execFileSync(path.join(root, 'web/target/debug/examples/case-settings'), [board.id], { input: JSON.stringify(document), encoding: 'utf8' }));
        assert.deepEqual(actual, expected, `${name}/${board.id}, existing=${keepExisting}`);
        count++;
      }
    }
  }
  console.log(`Reference mechanical presets: ${count} copied-fixture/board/settings cases passed.`);
} finally { await server.close(); }
