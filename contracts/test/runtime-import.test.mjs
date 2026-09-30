import assert from 'node:assert/strict';
import { defaultOutlineSettings, emptyProject, keycapDefaults } from '../src/index.ts';

assert.deepEqual(defaultOutlineSettings, {
  corners: 'fillet',
  size: 2,
  bridgeWidth: 10,
});

const document = emptyProject('node-import', 'Node import');
assert.equal(document.format, 'boardstudio/v2');
assert.equal(document.revision, 0);
assert.deepEqual(document.constraints, []);
assert.equal(keycapDefaults.board.color, '#e8e4dc');
assert.equal(keycapDefaults.matrix.wallThickness, 1.2);
assert.equal(keycapDefaults.key.legend, null);
