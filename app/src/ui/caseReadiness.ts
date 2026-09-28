import type { Finding, MechanicalAssembly, ProjectDoc } from '@boardstudio/v2-contracts';
import { generationMessage, type GenerationState } from '../generationState';
import { presentedFindings } from './findings';

export function mechanicalFindings(assembly: MechanicalAssembly | undefined, document: ProjectDoc): Finding[] {
  const grouped = new Map<string, Finding>();
  for (const finding of assembly?.diagnostics ?? []) {
    const key = `${finding.severity}:${finding.message}`;
    const previous = grouped.get(key);
    grouped.set(key, previous ? { ...previous, targetIds: [...new Set([...previous.targetIds, ...finding.targetIds])] } : finding);
  }
  return presentedFindings([...grouped.values()].map(finding => ({ ...finding,
    message: finding.targetIds.length > 1 ? `${finding.message} (${finding.targetIds.length} affected parts)` : finding.message,
  })), document);
}

export function caseReadiness({ revision, sceneRevision, previewRevision, boardId, configuredBoardId, generation, assembly, hasGeometry = false }: {
  revision: number;
  sceneRevision?: number;
  previewRevision?: number;
  boardId?: string;
  configuredBoardId?: string;
  generation?: GenerationState;
  assembly?: MechanicalAssembly;
  hasGeometry?: boolean;
}) {
  const state = generation ?? { status: 'required' };
  const active = Boolean(boardId && boardId === configuredBoardId);
  const resolved = active && assembly?.revision === revision && sceneRevision === revision;
  const errors = resolved ? assembly!.diagnostics.filter(finding => finding.severity === 'error').length : 0;
  const warnings = resolved ? assembly!.diagnostics.filter(finding => finding.severity === 'warning').length : 0;
  const busy = state.status === 'preparing' || state.status === 'running';
  const current = !state.draft && resolved && state.status === 'ready' && state.revision === revision && previewRevision === revision;
  const canExport = Boolean(current && !errors && !assembly?.generationBlocked);
  const retained = hasGeometry && !current ? ' Previous geometry is shown.' : '';
  let message: string;
  if (!active) message = 'Configure a mechanical stack for this board to generate its case.';
  else if (state.draft && !busy) message = generationMessage(state) + retained;
  else if (busy) message = generationMessage(state) + retained;
  else if (state.status === 'failed') message = `${state.message ?? 'Generation failed'}. Update preview to retry.${retained}`;
  else if (state.status === 'cancelled') message = `Preview paused. Update preview when ready.${retained}`;
  else if (state.status === 'blocked' || (resolved && assembly?.generationBlocked)) message = `Generation blocked · review mechanical findings.${retained}`;
  else if (current && errors) message = 'Geometry current · mechanical errors must be resolved before export.';
  else if (canExport) message = warnings ? 'Geometry current · export available with warnings to review.' : 'Geometry current · ready to export.';
  else if (!resolved) message = `Resolving configuration · export is unavailable.${retained}`;
  else message = `Preview out of date · update the solids before export.${retained}`;
  return { canExport, message, reviewRequired: errors > 0 || state.status === 'blocked', warnings, busy };
}

export type CaseReadiness = ReturnType<typeof caseReadiness>;
