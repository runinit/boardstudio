import type { ProjectDoc } from '../../../contracts/src/index';

export const setupGuideStages = ['project', 'layout', 'wiring', 'case', 'review'] as const;
export type SetupGuideStage = typeof setupGuideStages[number];

type SetupGuideStageStatus = { ready: boolean; optional?: boolean; detail: string };
export type SetupGuideStages = Record<SetupGuideStage, SetupGuideStageStatus>;

type SetupGuideInputs = {
  document: ProjectDoc;
  boardId?: string;
  wiring: { current: boolean; ready: boolean; applied: boolean };
  caseReadiness: { configured: boolean; canExport: boolean; message: string };
  layoutErrorCount: number;
};

export function deriveSetupGuide({ document, boardId, wiring, caseReadiness, layoutErrorCount }: SetupGuideInputs): SetupGuideStages {
  const board = document.boards.find((entry) => entry.id === boardId);
  const boardParts = new Set(board?.partIds ?? []);
  const matrices = document.matrices.filter((matrix) => matrix.boardId === boardId || (!matrix.boardId && matrix.partIds.some((id) => boardParts.has(id))));
  const enabledCells = matrices.reduce((count, matrix) => count + (matrix.rows * matrix.columns - new Set((matrix.cells ?? []).filter(cell => !cell.enabled && cell.row >= 0 && cell.row < matrix.rows && cell.column >= 0 && cell.column < matrix.columns).map(cell => `${cell.row}:${cell.column}`)).size), 0);
  const hardwareBoard = document.hardware?.boards.some((entry) => entry.boardId === boardId) ?? false;
  const instanceBoard = document.hardware?.instances.some((entry) => entry.boardId === boardId) ?? false;
  const projectReady = Boolean(board && (hardwareBoard || instanceBoard));
  const layoutParts = new Set(matrices.flatMap((matrix) => matrix.partIds));
  const standaloneKeys = document.parts.filter(part => boardParts.has(part.id) && !layoutParts.has(part.id) && document.definitions.some(definition => definition.id === part.definitionId && definition.kind === 'switch')).length;
  const keyCount = enabledCells + standaloneKeys;
  const layoutReady = Boolean(board && keyCount > 0 && (standaloneKeys > 0 || [...layoutParts].some(id => boardParts.has(id))) && layoutErrorCount === 0);
  const wiringReady = Boolean(board && wiring.current && wiring.ready && wiring.applied);
  const reviewReady = Boolean(board && layoutReady && wiringReady && (!caseReadiness.configured || caseReadiness.canExport));
  return {
    project: { ready: projectReady, detail: projectReady ? `${document.hardware?.topology === 'split' ? 'Split keyboard' : 'One keyboard'} · ${board!.name}.` : 'Choose one keyboard or a split keyboard.' },
    layout: { ready: layoutReady, detail: layoutReady ? `${keyCount} enabled key position${keyCount === 1 ? '' : 's'} are ready.` : layoutErrorCount ? `${layoutErrorCount} layout finding${layoutErrorCount === 1 ? '' : 's'} need attention.` : 'Create a layout and place its assemblies.' },
    wiring: { ready: wiringReady, detail: wiringReady ? 'Controller pins and board nets are applied.' : wiring.current && wiring.ready ? 'Apply the resolved wiring before export.' : 'Resolve the controller and board wiring.' },
    case: { ready: caseReadiness.canExport, optional: true, detail: caseReadiness.configured ? caseReadiness.message : 'Optional: configure a case or continue without one.' },
    review: { ready: reviewReady, detail: reviewReady ? 'The selected board is ready for review and export.' : 'Complete the required steps for this board before export.' },
  };
}
