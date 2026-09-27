import type { Dispatch, SetStateAction } from 'react';
import type { EditOperation, Matrix, MatrixCell, Part, PartDefinition, ProjectDoc, Vec2 } from '@boardstudio/v2-contracts';
import type { MatrixPresetId } from './assemblyCatalog';
import type { PairSetup } from './MirroredPairSetup';
import { pairAt } from './MirroredPairSetup';
import { matrixWithPreset } from './matrixPresets';
import type { SwitchOrientation } from './assemblyPresets';
import { createPart, makeId, PITCH_MM, snapDelta } from './workbenchGeometry';
import type { PlacementState } from './placementState';
import type { SelectionScope } from './workbenchTypes';

type WorkbenchPlacementControllerInputs = {
  document: ProjectDoc; selectedBoard?: { id: string; outlineIds: string[] }; selectedBoardId: string;
  definitions: Map<string, PartDefinition>; libraryDefinitions: PartDefinition[]; matrixMap: Map<string, Matrix>;
  matrixCellOverrides: Map<string, Map<string, MatrixCell>>; scope: SelectionScope | null; layoutTargetId: string; snapFraction: number;
  viewBounds: { minX: number; maxX: number; minY: number; maxY: number }; orientation: SwitchOrientation;
  placement: PlacementState; setPlacement: Dispatch<SetStateAction<PlacementState>>; cancelPlacement: () => void;
  emit: (operation: EditOperation, ids: string[]) => void; setAddPartOpen: (open: boolean) => void; setLeftOpen: (open: boolean) => void;
  setRightOpen: (open: boolean) => void; setOutlineActive: (active: boolean) => void; setOutlineSettingsOpen: (open: boolean) => void;
  setMode: (mode: 'Design') => void; setScope: (scope: SelectionScope | null) => void;
  setExpandedTree: (update: (current: Set<string>) => Set<string>) => void; setSelected: (ids: string[]) => void;
  selectScope: (scope: { kind: 'matrix'; matrixId: string }) => void; addDefinition: (definition: PartDefinition) => void;
  setCell: (matrix: Matrix, row: number, column: number, changes: Partial<MatrixCell>, definitions?: PartDefinition[]) => void;
};

export function createWorkbenchPlacementActions(input: WorkbenchPlacementControllerInputs) {
  const beginMatrixPlacement = (rows: number, columns: number, preset: MatrixPresetId, pair?: PairSetup) => {
    const definition = input.libraryDefinitions.find((item) => item.id === 'ergogen:ceoloide/switch_mx');
    if (!definition || !input.selectedBoard) return;
    const matrix: Matrix = { id: makeId(), boardId: input.selectedBoard.id, rows, columns, pitch: { x: PITCH_MM, y: PITCH_MM }, edgeGap: { x: 1, y: 1 }, origin: { x: 0, y: 0 }, definitionId: definition.id, partIds: [], cells: [] };
    const prepared = matrixWithPreset(matrix, preset, input.orientation);
    input.setPlacement(pair ? { kind: 'mirrored-pair', matrix: prepared.matrix, definitions: prepared.definitions, pair: { ...pair, leftId: makeId(), rightId: makeId(), rightMatrixId: makeId() } } : { kind: 'matrix', matrix: prepared.matrix, definitions: prepared.definitions });
    input.setLeftOpen(false);
    input.setRightOpen(false);
    input.setOutlineSettingsOpen(false);
    input.setMode('Design');
    input.setScope(null);
  };
  const placeLibraryDefinition = (definition: PartDefinition) => {
    const scope = input.scope;
    if (scope?.kind === 'key' && scope.matrixId !== undefined && scope.row !== undefined && scope.column !== undefined) {
      const matrix = input.matrixMap.get(scope.matrixId);
      if (matrix) {
        const current = input.matrixCellOverrides.get(matrix.id)?.get(`${scope.row}:${scope.column}`);
        const changes = definition.kind === 'switch' ? { enabled: true, definitionId: definition.id } : { enabled: true, assemblies: [...(current?.assemblies ?? []), { id: `library-${definition.id}-${(current?.assemblies?.length ?? 0) + 1}`, definitionId: definition.id, offset: { x: 0, y: 0 } }] };
        input.setCell(matrix, scope.row, scope.column, changes, input.definitions.has(definition.id) ? undefined : [definition]);
        input.setScope({ kind: 'key', matrixId: scope.matrixId, row: scope.row, column: scope.column });
        input.setMode('Design');
        return;
      }
    }
    input.addDefinition(definition);
  };
  const beginPartPlacement = (definition: PartDefinition) => {
    const point = snapDelta({ x: (input.viewBounds.minX + input.viewBounds.maxX) / 2, y: (input.viewBounds.minY + input.viewBounds.maxY) / 2 }, { x: PITCH_MM, y: PITCH_MM }, input.snapFraction);
    input.setPlacement({ kind: 'part', definition, layoutId: input.layoutTargetId, point });
    input.setAddPartOpen(false);
    input.setLeftOpen(false);
    input.setRightOpen(false);
    input.setOutlineActive(false);
    input.setMode('Design');
  };
  const placePendingPart = (at: Vec2) => {
    if (input.placement.kind !== 'part' || !input.selectedBoard) return;
    const { definition, layoutId } = input.placement;
    const part = { ...createPart(definition, input.document.parts), pose: { at, rotation: 0 } };
    input.emit({ kind: 'replace-document', document: documentWithPlacedPart(input.document, input.selectedBoardId, part, definition, layoutId) }, [part.id, input.selectedBoard.id]);
    input.cancelPlacement();
    input.setScope({ kind: 'component', partId: part.id });
    input.setSelected([part.id]);
    input.setOutlineSettingsOpen(false);
    input.setRightOpen(true);
  };
  const placeMatrixAt = (point: Vec2) => {
    if (input.placement.kind !== 'matrix' && input.placement.kind !== 'mirrored-pair') return;
    const draft = input.placement.matrix;
    const pair = input.placement.kind === 'mirrored-pair' ? pairAt(draft, input.placement.pair, point) : undefined;
    const placed = pair?.matrix ?? { ...draft, origin: point };
    const needed = new Set([placed.definitionId, ...(placed.cells ?? []).flatMap((cell) => [cell.definitionId, ...(cell.assemblies ?? []).map((assembly) => assembly.definitionId)])]);
    const required = [...input.libraryDefinitions, ...(input.placement.definitions ?? [])].filter((definition) => needed.has(definition.id) && !input.definitions.has(definition.id));
    input.emit(pair ? { kind: 'create-mirrored-pair', matrix: placed, left: pair.left, right: pair.right, definitions: required } : { kind: 'set-matrix', matrix: placed, definitions: required }, [placed.id, ...required.map((definition) => definition.id)]);
    if (pair) input.setExpandedTree((current) => new Set([...current, `half:${pair.left.id}`, `half:${pair.right.id}`]));
    input.cancelPlacement();
    input.setExpandedTree((current) => new Set(current).add(`matrix:${placed.id}`));
    input.selectScope({ kind: 'matrix', matrixId: placed.id });
  };
  return { beginMatrixPlacement, placeLibraryDefinition, beginPartPlacement, placePendingPart, placeMatrixAt };
}

export function documentWithPlacedPart(document: ProjectDoc, boardId: string, part: Part, definition: PartDefinition, layoutId: string): ProjectDoc {
  const board = document.boards.find((item) => item.id === boardId);
  const definitions = document.definitions.some((item) => item.id === definition.id) ? document.definitions : [...document.definitions, definition];
  const parts = document.parts.some((item) => item.id === part.id) ? document.parts : [...document.parts, part];
  const boards = document.boards.map((item) => item.id === boardId && !item.partIds.includes(part.id) ? { ...item, partIds: [...item.partIds, part.id] } : item);
  const outline = document.outline.map((feature) => feature.kind === 'part-envelope' && board?.outlineIds.includes(feature.id) && !feature.partIds.includes(part.id) ? { ...feature, partIds: [...feature.partIds, part.id] } : feature);
  const layouts = document.layouts?.map((layout) => layout.id === layoutId && !layout.partIds.includes(part.id) ? { ...layout, partIds: [...layout.partIds, part.id] } : layout);
  return { ...document, definitions, parts, boards, outline, ...(layouts ? { layouts } : {}) };
}
