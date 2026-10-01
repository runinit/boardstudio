import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {buildAssembly,previewAssembly} from '../../../../../cad/src/index.ts';
const prepared=JSON.parse(readFileSync(new URL('../../fixtures/prepared.json',import.meta.url),'utf8'));
await buildAssembly(prepared);
const actual=await previewAssembly(prepared,()=>{});
// Frozen real-browser cold preview output, not a value derived from this reference.
const browser=JSON.parse(readFileSync(new URL('../initial-browser/boardstudio-p1-cad-20261001-root-browser.json',import.meta.url),'utf8'));
assert.equal(actual.bodies[0].positions.length/3,browser.report.preview.vertices);
