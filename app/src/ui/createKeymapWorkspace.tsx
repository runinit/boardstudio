import type { ReactNode } from 'react';
import type { EditOperation, KeycapBoardSettings, KeycapKeySettings, KeycapMatrixSettings, Part, PartDefinition, ProjectDoc, SceneDelta, Vec2 } from '@boardstudio/v2-contracts';
import { defaultKeycapBoard, defaultKeycapKey, defaultKeycapMatrix, keyBinding } from '../keycapSettings';
import { bindingLabel } from './keyBindingChoices';
import { KeymapLayout } from './KeymapLayout';
import { KeymapPanel } from './KeymapPanel';
import type { ExportKind } from './workbenchTypes';

export type KeymapView = {
  boardId: string;
  colors: KeycapBoardSettings;
  keys: { part: Part; binding: string; legend: string; settings: KeycapKeySettings; color: string; size: Vec2 }[];
  matrices: { id: string; name: string; settings: KeycapMatrixSettings }[];
};

type Inputs = {
  document: ProjectDoc;
  scene: SceneDelta;
  boardId: string;
  parts: Part[];
  definitions: Map<string, PartDefinition>;
  selected: ReadonlySet<string>;
  selectedKeyId?: string;
  selectKey: (id: string) => void;
  emit: (operation: EditOperation, targetIds: string[]) => unknown;
  exportFile: (kind: ExportKind, boardId: string) => void;
  firmwareControls?: ReactNode;
};

/** Both keymap surfaces share membership, presentation defaults, and edit targets. */
export function createKeymapWorkspace({ document, scene, boardId, parts, definitions, selected, selectedKeyId, selectKey, emit, exportFile, firmwareControls }: Inputs) {
  const board = document.boards.find(board => board.id === boardId);
  const colors = document.keycaps?.boards[boardId] ?? defaultKeycapBoard;
  let view: KeymapView | undefined;
  const readView = (): KeymapView => view ??= {
    boardId, colors,
    keys: parts.filter(part => board?.partIds.includes(part.id) && definitions.get(part.definitionId)?.kind === 'switch').map(part => {
      const settings = document.keycaps?.keys[part.id] ?? defaultKeycapKey;
      const binding = keyBinding(document, boardId, part.id);
      return {
        part, settings, binding,
        legend: settings.legend ?? bindingLabel(binding),
        color: settings.color ?? colors.color,
        size: settings.units ? { x: settings.units.x * 19.05 - 0.85, y: settings.units.y * 19.05 - 0.85 }
          : part.keycap ?? definitions.get(part.definitionId)?.keycap ?? { x: 18.2, y: 18.2 },
      };
    }),
    matrices: document.matrices.filter(matrix => matrix.boardId === boardId || matrix.partIds.some(id => board?.partIds.includes(id)))
      .map(matrix => ({ id: matrix.id, name: matrix.name ?? matrix.id, settings: document.keycaps?.matrices[matrix.id] ?? defaultKeycapMatrix })),
  };
  const edit = (operation: EditOperation) => {
    switch (operation.kind) {
      case 'set-key-binding': return emit(operation, [boardId, operation.keyId]);
      case 'set-keycap-board': return emit(operation, [boardId]);
      case 'set-matrix-keycaps': return emit(operation, [operation.matrixId]);
      case 'set-keycap-key': return emit(operation, [operation.keyId]);
    }
  };
  return {
    panel: () => <KeymapPanel view={readView()} selectedKeyId={selectedKeyId} onSelect={selectKey} onEdit={edit}
      onExport={() => exportFile('firmware', boardId)} onExportKeycaps={() => exportFile('keycaps-step', boardId)}
      findings={scene.findings.filter(finding => finding.id.startsWith('keycaps/')).map(finding => finding.message)} firmwareControls={firmwareControls} />,
    canvas: () => <KeymapLayout keys={readView().keys} selected={selected} onSelect={selectKey} />,
  };
}
