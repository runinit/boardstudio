import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';

const script = new URL('./import-kicad-parts.mjs', import.meta.url);
const source = '(footprint "test" (layer "F.Cu") (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask")) (pad "1" smd rect (at 3 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask")) (pad "2" np_thru_hole circle (at 6 0) (size 2 2) (drill 2) (layers "*.Cu" "*.Mask")))';

test('explicit roles retain duplicate SMD pads and reject holes and source drift atomically', () => {
  const directory = mkdtempSync(resolve(tmpdir(), 'boardstudio-import-test-'));
  try {
    writeFileSync(resolve(directory, 'part.kicad_mod'), source);
    writeFileSync(resolve(directory, 'LICENSE'), 'Test fixture authored for this test.');
    const entry = { id: 'test', name: 'Test', kind: 'custom', file: 'part.kicad_mod', sha256: createHash('sha256').update(source).digest('hex'), repository: 'https://example.com/test', revision: 'test', sourcePath: 'part.kicad_mod', license: 'test', licenseFile: 'LICENSE', terminals: { signal: ['1'] } };
    const run = entries => {
      writeFileSync(resolve(directory, 'manifest.json'), JSON.stringify({ formatVersion: 1, entries }));
      return spawnSync(process.execPath, [script.pathname, resolve(directory, 'manifest.json')], { encoding: 'utf8' });
    };
    const valid = run([entry]);
    assert.equal(valid.status, 0, valid.stderr);
    const output = resolve(directory, 'imported-parts.json');
    const accepted = readFileSync(output, 'utf8');
    const definition = JSON.parse(accepted).parts[0].definition;
    assert.equal(definition.terminals.signal.length, 2);
    assert.equal(definition.kicadSource.source, source);
    for (const [entries, message] of [
      [[{ ...entry, terminals: { signal: ['2'] } }], /Missing conductive pad/],
      [[{ ...entry, sha256: 'incorrect' }], /Source hash mismatch/],
      [[entry, entry], /Duplicate definition ID/],
      [[{ ...entry, matrixTerminals: { row: 'signal', column: 'missing' } }], /Missing matrix terminal/],
    ]) {
      const rejected = run(entries);
      assert.notEqual(rejected.status, 0);
      assert.match(rejected.stderr, message);
      assert.equal(readFileSync(output, 'utf8'), accepted);
    }
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
