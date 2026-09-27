import { readFileSync } from 'node:fs';
import { expect, test } from 'vitest';
import { CoreEngine, initSync } from '../../../core/pkg/boardstudio_core';
import type { CoreReply, CoreRequest } from '@boardstudio/v2-contracts';
import { keyboardProject, keyboardDemos } from './keyboards';
import { sofleProject, sofleDemos } from './sofle';
import { matrixWithPreset } from '../ui/matrixPresets';
import { resizeMatrix } from '../ui/matrixResize';
initSync({ module: readFileSync(new URL('../../../core/pkg/boardstudio_core_bg.wasm', import.meta.url)) });

test('changing GH60 presets retains wide caps and stabilizers', () => {
  const engine = new CoreEngine();
  const request = (input: CoreRequest) => JSON.parse(engine.request(JSON.stringify(input))) as CoreReply;
  try {
    const doc = keyboardProject('gh60');
    request({ id:'open',kind:'open',document:doc });
    const prepared = matrixWithPreset(doc.matrices[0], 'mx-solder');
    const result = request({id:'edit',kind:'edit',command:{baseRevision:0,transactionId:'preset',phase:'commit',targetIds:[],operation:{kind:'set-matrix',matrix:prepared.matrix,definitions:prepared.definitions}}});
    expect(result.kind).toBe('scene');
    if(result.kind!=='scene') throw new Error(JSON.stringify(result));
    const stabilizers = (document: typeof doc) => document.parts.filter(part=>part.id.endsWith('/stabilizer'));
    expect(stabilizers(result.document).length).toBe(stabilizers(doc).length);
    for(const before of doc.parts.filter(part=>doc.definitions.find(d=>d.id===part.definitionId)?.kind==='switch')) {
      const after=result.document.parts.find(part=>part.id===before.id)!;
      const size=(document:typeof doc,part:typeof before)=>part.keycap??document.definitions.find(d=>d.id===part.definitionId)?.keycap;
      expect(size(result.document,after)).toEqual(size(doc,before));
    }
  } finally {engine.free();}
});

test('Plaid uses four physical rows and twelve columns', () => {
  expect(keyboardProject('plaid').matrices[0]).toMatchObject({rows:4,columns:12});
});

test('growing the right Corne cluster uses its local board origin and live outline', () => {
  const doc=keyboardProject('corne');
  const matrix=doc.matrices.find(matrix=>matrix.boardId==='right')!;
  const left=doc.matrices.find(matrix=>matrix.boardId==='left')!;
  expect(matrix.origin.x).toBeGreaterThan(left.origin.x);
  expect(doc.outline.every(feature=>feature.kind==='part-envelope')).toBe(true);
  const grown=resizeMatrix(matrix,matrix.rows+1,matrix.columns);
  expect(grown.cells!.filter(cell=>cell.row===matrix.rows).every(cell=>!cell.offset)).toBe(true);
});

test.each([...keyboardDemos,...sofleDemos])('$name regenerates measured poses and caps without drift', ({id}) => {
  const engine = new CoreEngine();
  const request = (input: CoreRequest) => JSON.parse(engine.request(JSON.stringify(input))) as CoreReply;
  try {
    let doc = id==='v2'||id==='rgb'||id==='choc'?sofleProject(id):keyboardProject(id);
    request({id:'open',kind:'open',document:doc});
    for (const matrix of doc.matrices) {
      const result = request({id:'edit',kind:'edit',command:{baseRevision:doc.revision,transactionId:'layout',phase:'commit',targetIds:[matrix.id],operation:{kind:'set-matrix',matrix}}});
      if(result.kind!=='scene') throw new Error(JSON.stringify(result));
      for (const before of doc.parts) {
        const after=result.document.parts.find(part=>part.id===before.id)!;
        expect(after.pose.at.x,before.id).toBeCloseTo(before.pose.at.x,5);
        expect(after.pose.at.y,before.id).toBeCloseTo(before.pose.at.y,5);
        expect(after.pose.rotation,before.id).toBeCloseTo(before.pose.rotation,5);
        expect(after.keycap).toEqual(before.keycap);
      }
      doc=result.document;
    }
  } finally {engine.free();}
});

test('MX stabilizers reject Choc presets and retain world pose for north-facing switches', () => {
  const doc=keyboardProject('gh60');
  expect(()=>matrixWithPreset(doc.matrices[0],'choc-solder','south',doc.definitions)).toThrow(/MX stabilizer/);
  const north=matrixWithPreset(doc.matrices[0],'mx-solder','north',doc.definitions);
  const south=matrixWithPreset(north.matrix,'mx-hotswap','south',doc.definitions);
  for(const cell of doc.matrices[0].cells!) {
    const after=south.matrix.cells.find(other=>other.row===cell.row&&other.column===cell.column)!;
    expect(after.assemblies.filter(member=>member.id==='stabilizer')).toEqual(cell.assemblies?.filter(member=>member.id==='stabilizer'));
  }
  const grown=resizeMatrix(north.matrix,north.matrix.rows+1,north.matrix.columns);
  expect(grown.cells!.filter(cell=>cell.row===north.matrix.rows).every(cell=>!cell.assemblies?.some(member=>member.id==='stabilizer'))).toBe(true);
});

test('saved authored keycap definitions survive replacing a preset',()=>{
  const engine=new CoreEngine();
  const request=(input:CoreRequest)=>JSON.parse(engine.request(JSON.stringify(input))) as CoreReply;
  try {
    const doc=keyboardProject('gh60');
    const wide=doc.parts.find(part=>part.keycap)!;
    const authored={...doc.definitions.find(definition=>definition.id===wide.definitionId)!,id:'saved-wide',keycap:wide.keycap,envelopeSource:{keycap:'authored' as const}};
    doc.definitions.push(authored);wide.definitionId=authored.id;delete wide.keycap;
    doc.matrices[0].cells!.find(cell=>`matrix/${doc.matrices[0].id}/r${cell.row}c${cell.column}`===wide.id)!.definitionId=authored.id;
    request({id:'open',kind:'open',document:doc});
    const prepared=matrixWithPreset(doc.matrices[0],'mx-solder');
    const result=request({id:'edit',kind:'edit',command:{baseRevision:0,transactionId:'preset',phase:'commit',targetIds:[],operation:{kind:'set-matrix',...prepared}}});
    if(result.kind!=='scene') throw new Error(JSON.stringify(result));
    expect(result.document.parts.find(part=>part.id===wide.id)!.keycap).toEqual(authored.keycap);
  } finally {engine.free();}
});
