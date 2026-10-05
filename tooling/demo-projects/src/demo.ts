import type { Part, ProjectDoc, Vec2 } from '@boardstudio/v2-contracts';
import { defaultOutlineSettings, emptyProject } from '@boardstudio/v2-contracts';
import { catalogue, normalizeDefinition } from '@boardstudio/v2-ergogen';

const ROWS = 3;
const COLUMNS = 5;
const PITCH_MM = 19.05;

export function demoProject(): ProjectDoc {
  const doc = emptyProject('starter', 'Starter keyboard');
  const definitions = [normalizeDefinition({ ...catalogue().find(item => item.id === 'ergogen:ceoloide/switch_mx')!, name: 'MX switch', generator: { source: 'ceoloide/switch_mx', version: 'bundled-1', parameters: { hotswap: false, solder: true } } })];
  const definition = definitions.find((entry) => entry.id === 'ergogen:ceoloide/switch_mx')!;

  const parts: Part[] = [];
  const rowPins: { partId: string; padId: string }[][] = Array.from({ length: ROWS }, () => []);
  const colPins: { partId: string; padId: string }[][] = Array.from({ length: COLUMNS }, () => []);

  for (let row = 0; row < ROWS; row += 1) {
    for (let col = 0; col < COLUMNS; col += 1) {
      const id = `matrix/matrix/r${row}c${col}`;
      const at: Vec2 = { x: col * PITCH_MM, y: row * PITCH_MM };

      parts.push({ id, definitionId: definition.id, reference: `SW${parts.length + 1}`, pose: { at, rotation: 0 }, side: 'front' });
      for (const padId of definition.terminals!.from) rowPins[row].push({ partId: id, padId });
      for (const padId of definition.terminals!.to) colPins[col].push({ partId: id, padId });
    }
  }

  doc.definitions = definitions;
  doc.parts = parts;
  doc.matrices = [{
    id: 'matrix', boardId: 'main-board', rows: ROWS, columns: COLUMNS,
    pitch: { x: PITCH_MM, y: PITCH_MM }, origin: { x: 0, y: 0 },
    definitionId: definition.id, partIds: parts.map((part) => part.id),
  }];
  doc.nets = [
    ...rowPins.map((pins, row) => ({ id: `row-${row}`, name: `ROW${row}`, pins })),
    ...colPins.map((pins, col) => ({ id: `col-${col}`, name: `COL${col}`, pins })),
  ];
  doc.outline = [{ id: 'board-envelope', kind: 'part-envelope', settings: defaultOutlineSettings, partIds: parts.map((part) => part.id), margin: 4, operation: 'add' }];
  doc.boards = [{ id: 'main-board', name: 'Main board', outlineIds: ['board-envelope'], partIds: parts.map((part) => part.id), netIds: doc.nets.map((net) => net.id), thickness: 1.6 }];
  doc.materials = [{ id: 'pla', name: 'PLA', thickness: 3 }];
  doc.caseBodies = [{ id: 'switch-plate', name: 'Switch plate', boardId: 'main-board', kind: 'plate', thickness: 3, clearance: 0.5, materialId: 'pla' }];

  return doc;
}
