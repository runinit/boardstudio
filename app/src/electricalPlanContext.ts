import type { ElectricalPlan, ProjectDoc } from '@boardstudio/v2-contracts';

export type ElectricalPlanContext = {
  document: ProjectDoc;
  plan: ElectricalPlan;
};

export function isCurrentElectricalPlan(context: ElectricalPlanContext | undefined, document: ProjectDoc, boardId: string): context is ElectricalPlanContext {
  return Boolean(context && context.document === document && context.plan.revision === document.revision && context.plan.boardId === boardId);
}

export function canPublishElectricalPlan(capturedDocument: ProjectDoc, capturedBoardId: string, currentDocument: ProjectDoc, currentBoardId: string): boolean {
  return capturedDocument === currentDocument && capturedBoardId === currentBoardId;
}

export function isWiringApplied(document: ProjectDoc, plan: ElectricalPlan): boolean {
  const boardId = plan.boardId;
  if (!boardId || plan.revision !== document.revision) return false;
  const board = document.boards.find(item => item.id === boardId);
  if (!board) return false;
  const configuration = document.hardware?.boards.find(item => item.boardId === boardId);
  const prefix = `generated/electrical/${boardId}/`;
  const current = document.nets.filter(net => net.id.startsWith(prefix));
  return current.length === plan.nets.length
    && configuration?.mode === plan.mode
    && configuration.controllerPartId === plan.controllerPartId
    && plan.nets.every(net => JSON.stringify(current.find(item => item.id === net.id)) === JSON.stringify(net) && board.netIds.includes(net.id));
}
