import type { MatrixPresetId } from './assemblyCatalog';
import type { Matrix, PartDefinition } from '@boardstudio/v2-contracts';
import { catalogue as ergogenCatalogue } from '@boardstudio/v2-ergogen';
import { assemblyPreset, type SwitchOrientation, type AssemblyConstruction } from './assemblyPresets';
import { matrixWithAssembly } from './assemblyPlacement';

export const matrixWithPreset = (matrix: Matrix, presetId: MatrixPresetId, orientation: SwitchOrientation = 'south', currentDefinitions: readonly PartDefinition[] = [], construction: AssemblyConstruction = 'single-sided') => {
  const definitions = ergogenCatalogue();
  const assembly = assemblyPreset(presetId, definitions, orientation, construction);
  assembly.id = `preset-${presetId}-${orientation}${construction === 'reversible' ? '-reversible' : ''}`;
  const prepared = { ...matrix, cells: matrix.cells?.map(cell => ({ ...cell,
    rotation: (cell.rotation ?? 0) - (cell.variant?.startsWith('preset/') && cell.variant.endsWith('/north') ? 180 : 0),
  })) };
  const result = matrixWithAssembly(prepared, assembly, definitions, 0);
  const oldCells = new Map(matrix.cells?.map(cell => [`${cell.row}:${cell.column}`, cell]));
  for (const cell of result.matrix.cells) {
    const old = oldCells.get(`${cell.row}:${cell.column}`);
    const extras = old?.assemblies?.filter(member => !['diode', 'led'].includes(member.id)) ?? [];
    if (presetId.startsWith('choc') && extras.some(member => member.definitionId.startsWith('kicad:marbastlib/STAB_MX_') || currentDefinitions.find(definition => definition.id === member.definitionId)?.mechanicalProfile?.switchFamily === 'mx')) {
      throw new Error('This key has an MX stabilizer. Remove it or choose a compatible MX preset before switching to Choc.');
    }
    const delta = (cell.rotation - (old?.rotation ?? 0)) * Math.PI / 180;
    cell.assemblies.push(...extras.map(member => ({ ...member, side: member.side ?? 'front',
      offset: { x: member.offset.x * Math.cos(delta) + member.offset.y * Math.sin(delta), y: -member.offset.x * Math.sin(delta) + member.offset.y * Math.cos(delta) },
      rotation: (member.rotation ?? 0) - delta * 180 / Math.PI,
    })));
  }
  return { ...result, matrix: { ...result.matrix, cells: result.matrix.cells.map(cell => ({ ...cell, variant: `preset/${presetId}/${construction === 'reversible' ? 'reversible/' : ''}${orientation}` })) } };
};


