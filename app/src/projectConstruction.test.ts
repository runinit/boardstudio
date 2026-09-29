import { expect, it } from 'vitest';
import { demoProject } from './demo';
import { matrixWithPreset } from './ui/matrixPresets';
import { reversibleLayout, withReversibleLayout } from './projectConstruction';

it('changes existing assembly footprints and physical halves together without moving keys', () => {
  const doc = demoProject();
  const preset = matrixWithPreset(doc.matrices[0], 'mx-hotswap-rgb');
  doc.matrices = [preset.matrix];
  doc.definitions = preset.definitions;
  doc.hardware = { topology: 'split', transport: 'wireless', boards: [], sharedConstruction: null, instances: ['left','right'].map(half => ({ id: half, name: half, boardId: doc.boards[0].id, half, role: half === 'left' ? 'central' : 'peripheral', flipped: false, constructionLinked: true, controllerPartId: null, mechanical: null })) };
  const updated = withReversibleLayout(doc, true);
  expect(reversibleLayout(updated)).toBe(true);
  expect(updated.definitions.every(definition => definition.generator?.parameters.reversible === true)).toBe(true);
  expect(updated.matrices).toEqual(doc.matrices);
  expect(updated.hardware?.instances.map(instance => instance.flipped)).toEqual([false, true]);
  const restored = withReversibleLayout(updated, false);
  expect(restored.definitions.every(definition => definition.generator?.parameters.reversible === false)).toBe(true);
  expect(restored.hardware?.instances.every(instance => !instance.flipped)).toBe(true);
});
