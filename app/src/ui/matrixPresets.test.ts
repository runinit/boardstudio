import { expect, it } from 'vitest';
import { render } from '@boardstudio/v2-ergogen';
import { demoProject } from '../demo';
import { matrixPresetDefinitions, type MatrixPresetId } from './assemblyCatalog';
import { matrixWithPreset } from './matrixPresets';

const cases = (Object.keys(matrixPresetDefinitions) as MatrixPresetId[]).flatMap(id =>
  (['south', 'north'] as const).map(orientation => ({ id, orientation })));

it.each(cases)('$id $orientation preserves layout identity and is idempotent', ({ id, orientation }) => {
  const original = demoProject().matrices[0];
  const matrix = { ...original, cells: [{ row: 0, column: 0, enabled: false, rotation: 23, offset: { x: 2, y: 3 } }] };
  expect(Object.keys(matrixPresetDefinitions)).toHaveLength(8);
  const first = matrixWithPreset(matrix, id, orientation);
  expect(matrixWithPreset(first.matrix, id, orientation)).toEqual(first);
  const restored = matrixWithPreset(first.matrix, id, 'south');
  expect(restored.matrix).toMatchObject({ id: original.id, partIds: original.partIds });
  expect(restored.matrix.cells[0]).toMatchObject({ enabled: false, rotation: 23, offset: { x: 2, y: 3 } });
  expect(first.definitions[0].generator?.source).toBe(matrixPresetDefinitions[id].definitionId.slice('ergogen:'.length));
  expect(first.matrix.cells[0].assemblies).toHaveLength(matrixPresetDefinitions[id].led ? 2 : 1);
});

it('reversible presets use existing two-sided footprints and can return to single-sided', () => {
  const original = demoProject().matrices[0];
  const reversible = matrixWithPreset(original, 'mx-hotswap', 'north', [], 'reversible');
  expect(reversible.definitions.every(definition => definition.generator?.parameters.reversible === true)).toBe(true);
  const footprint = render(reversible.definitions[0]).find(form => form[0] === 'footprint')!;
  const pads = footprint.filter(form => Array.isArray(form) && form[0] === 'pad' && form[2] === 'smd');
  expect(JSON.stringify(pads)).toContain('F.Cu');
  expect(JSON.stringify(pads)).toContain('B.Cu');
  expect(matrixWithPreset(reversible.matrix, 'mx-hotswap', 'north', reversible.definitions, 'reversible')).toEqual(reversible);
  const single = matrixWithPreset(reversible.matrix, 'mx-hotswap', 'south', reversible.definitions, 'single-sided');
  expect(single.definitions[0].generator?.parameters.reversible).toBe(false);
  expect(single.definitions[0].id).not.toBe(reversible.definitions[0].id);
  expect(single.matrix.cells[0].rotation).toBe(original.cells?.[0]?.rotation ?? 0);
});
