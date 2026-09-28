import { defaultGasketLayout, defaultInternalGasket } from './gasketEditing';
import { expect, test } from 'vitest';
import type { PhysicalBoardInstance, SceneDelta } from '@boardstudio/v2-contracts';
import { demoProject } from './demo';
import { createMechanicalConfiguration } from './mechanicalPresets';
import { effectiveCaseDocument, effectiveCaseScene, mechanicalFingerprint, updateInstanceMechanical } from './hardwareInstances';

const flipPhysicalPoint = (point: { x: number; y: number; z: number }, pcbThickness: number) => ({ x: -point.x, y: point.y, z: -pcbThickness - point.z });

const instance = (id: string, flipped = false): PhysicalBoardInstance => ({
  id, name: id, boardId: 'main-board', half: flipped ? 'right' : 'left', role: flipped ? 'peripheral' : 'central',
  flipped, constructionLinked: true, controllerPartId: null, mechanical: null,
});

test('a reversible instance flips both its face and its coordinates without modifying the PCB design', () => {
  const doc = demoProject();
  const original = structuredClone(doc);
  const part = doc.parts[0];
  const right = effectiveCaseDocument(doc, instance('right', true));
  expect(right.parts[0].pose.at.x).toBe(-part.pose.at.x);
  expect(right.parts[0].pose.rotation).toBe(-part.pose.rotation);
  expect(right.parts[0].side).toBe('back');
  expect(flipPhysicalPoint({ x: 2, y: 3, z: 4 }, 1.6)).toEqual({ x: -2, y: 3, z: -5.6 });
  expect(doc).toEqual(original);
});

test('linked construction propagates common dimensions while openings stay local', () => {
  const doc = demoProject();
  const config = createMechanicalConfiguration(doc);
  doc.hardware = { topology: 'split', transport: 'wireless', boards: [], instances: [instance('left'), instance('right', true)], sharedConstruction: config };
  const next = updateInstanceMechanical(doc, 'left', { ...config, wallThickness: 4, openings: [{ points: [{x: 5,y: 0}], z: 0, height: 3 }] });
  const left = effectiveCaseDocument(next, next.hardware!.instances[0]);
  const right = effectiveCaseDocument(next, next.hardware!.instances[1]);
  expect(left.mechanical?.wallThickness).toBe(4);
  expect(right.mechanical?.wallThickness).toBe(4);
  expect(left.mechanical?.openings).toHaveLength(1);
  expect(right.mechanical?.openings ?? []).toHaveLength(0);
});

test('mechanical fingerprints ignore wiring and revisions, but track physical edits and instances', () => {
  const doc = demoProject();
  const scene = { boardContours: [{boardId:'main-board', contours:[]}], transforms: [] } as unknown as SceneDelta;
  const first = mechanicalFingerprint(doc, scene, 'main-board', instance('left'));
  const rewired = { ...doc, revision: doc.revision + 1, nets: [] };
  expect(mechanicalFingerprint(rewired, scene, 'main-board', instance('left'))).toBe(first);
  const changed = structuredClone(doc);
  changed.parts[0].pose.at.x += 1;
  expect(mechanicalFingerprint(changed, scene, 'main-board', instance('left'))).not.toBe(first);
  expect(mechanicalFingerprint(doc, scene, 'main-board', instance('right', true))).not.toBe(first);
});

test('physical scene contours and transforms agree with the selected instance', () => {
  const scene = { transforms: [{ id:'key', pose:{at:{x:10,y:3}, rotation:15} }], boardContours:[{boardId:'main-board', contours:[{hole:false,points:[{x:10,y:3}]}]}] } as unknown as SceneDelta;
  const doc = demoProject();
  doc.boards[0].partIds.push('key');
  const result = effectiveCaseScene(doc, scene, instance('right', true));
  expect(result.transforms[0].pose).toEqual({at:{x:-10,y:3},rotation:-15});
  expect(result.boardContours[0].contours[0].points[0].x).toBe(-10);
  expect(scene.transforms[0].pose.at.x).toBe(10);
});

test('disabling an instance does not fall back to the shared or board-level case', () => {
  const doc = demoProject();
  doc.mechanical = createMechanicalConfiguration(doc);
  doc.hardware = { topology:'split', transport:'wireless', boards:[], instances:[{...instance('left'), mechanical:doc.mechanical}], sharedConstruction:doc.mechanical };
  const next = updateInstanceMechanical(doc, 'left', null);
  expect(effectiveCaseDocument(next, next.hardware!.instances[0]).mechanical).toBeUndefined();
});

test('case setup and closure hardware are shared across both halves while gasket positions remain local', () => {
  const doc = demoProject();
  const config = createMechanicalConfiguration(doc);
  doc.hardware = { topology:'split', transport:'wired', boards:[], instances:[{...instance('left'),constructionLinked:false}, {...instance('right',true),constructionLinked:false}], sharedConstruction:null };
  const gasket = { ...config, mount:'gasket' as const, internalGasket:defaultInternalGasket(), gasketLayout:{...defaultGasketLayout(),supports:[{id:'left:0',regionId:'left',outlineKey:'test',anchor:0.2,unlinked:false,length:30}]} };
  const next = updateInstanceMechanical(doc,'left',gasket);
  const left = effectiveCaseDocument(next,next.hardware!.instances[0]).mechanical!;
  const right = effectiveCaseDocument(next,next.hardware!.instances[1]).mechanical!;
  expect(right.internalGasket).toEqual(left.internalGasket);
  expect(right.mount).toBe('gasket');
  expect(right.gasketLayout?.supports).toEqual([]);
  expect(left.gasketLayout?.supports[0].length).toBe(30);
  const updated = updateInstanceMechanical(next,'right',{...right,internalGasket:{...right.internalGasket!,hardware:{...right.internalGasket!.hardware,insertLength:4}}});
  expect(effectiveCaseDocument(updated,updated.hardware!.instances[0]).mechanical!.internalGasket!.hardware.insertLength).toBe(4);
});
