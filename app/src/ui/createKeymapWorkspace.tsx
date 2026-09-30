import type { ReactNode } from 'react';
import type { KeyBinding, KeymapConfiguration, EditOperation, KeycapBoardSettings, KeycapKeySettings, KeycapMatrixSettings, Part, PartDefinition, ProjectDoc, SceneDelta, Vec2 } from '@boardstudio/v2-contracts';
import { defaultKeycapBoard, defaultKeycapKey, defaultKeycapMatrix, keyBinding } from '../keycapSettings';
import { bindingLabel } from './keyBindingChoices';
import { KeymapLayout } from './KeymapLayout';
import { KeycapPanel } from './KeycapPanel';
import { bindingTitle } from './KeyBindingEditor';
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
  encoders?: { id: string; name: string }[];
  showFinding: (finding: SceneDelta['findings'][number]) => void;
  layerId: string;
  onLayer: (id: string) => void;
};

/** Both keymap surfaces share membership, presentation defaults, and edit targets. */
export function createKeymapWorkspace({ document, scene, boardId, parts, definitions, selected, selectedKeyId, selectKey, emit, exportFile, layerId, onLayer, encoders = [], showFinding }: Inputs) {
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
  const map: KeymapConfiguration = (document.keymap?.layers.length ? document.keymap : undefined) ?? { layers: [{ id: 'base', name: 'Base', bindings: {}, sensors: {} }], macros: [] };
  const layer = map.layers.find(entry => entry.id === layerId) ?? map.layers[0];
  const bindingFor = (id: string): KeyBinding => {
    if (layer.bindings[id]) return layer.bindings[id];
    if (layer !== map.layers[0]) return { kind: 'transparent' };
    const legacy = keyBinding(document, boardId, id);
    return legacy.startsWith('&kp ') ? { kind: 'key-press', keycode: legacy.slice(4) } : { kind: legacy === '&trans' ? 'transparent' : 'none' };
  };

  return {
    panel: () => <KeymapPanel view={readView()} map={map} layer={layer} selectedKeyId={selectedKeyId} onSelect={selectKey} onLayer={onLayer} bindingFor={bindingFor} encoders={encoders}
      onChange={change => emit({ kind: 'edit-keymap', change }, [boardId])} onExport={() => exportFile('firmware', boardId)} />,
    keycapsPanel: () => <KeycapPanel view={readView()} selectedKeyId={selectedKeyId} onSelect={selectKey} onEdit={edit}
      onExportKeycaps={() => exportFile('keycaps-step', boardId)}
      document={document} onShowFinding={showFinding} findings={scene.findings.filter(finding => finding.id.startsWith('keycaps/'))} />,
    canvas: () => <KeymapLayout keys={readView().keys.map(key => ({ ...key, legend: bindingTitle(bindingFor(key.part.id), map) }))} selected={selected} onSelect={selectKey} />,
    keycapsCanvas: () => <KeymapLayout keys={readView().keys} selected={selected} onSelect={selectKey} />,
  };
}
