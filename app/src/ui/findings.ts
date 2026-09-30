import type { Finding, MechanicalAssembly, ProjectDoc } from '../../../contracts/src/index';

const severityOrder = { error: 0, warning: 1, info: 2 };

/** Board diagnostics wrap outline diagnostics; collapse only the same source and geometry. */
export function presentedFindings(findings: Finding[], document: ProjectDoc): Finding[] {
  const boards = new Set(document.boards.map((board) => board.id));
  const result = new Map<string, Finding>();
  for (const finding of findings) {
    const marker = finding.id.indexOf(':feature:');
    const source = marker < 0 ? finding.id : finding.id.slice(marker + ':feature:'.length);
    const geometry = finding.targetIds.filter((id) => !boards.has(id));
    const targets = geometry.length ? geometry : finding.targetIds;
    const key = JSON.stringify([source, finding.severity, finding.message, [...targets].sort()]);
    const previous = result.get(key);
    result.set(key, previous ? { ...previous, targetIds: [...new Set([...previous.targetIds, ...finding.targetIds])] } : finding);
  }
  return [...result.values()].sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity]);
}

export function findingTarget(finding: Finding, document: ProjectDoc, assembly?: MechanicalAssembly) {
  const owner = document.boardOutlines?.find(state => state.versions.some(version => version.id === state.activeVersionId && version.geometry.features.some(feature => finding.targetIds.includes(feature.id))));
  const outline = document.outline.find((feature) => finding.targetIds.includes(feature.id))
    ?? owner?.versions.find(version => version.id === owner.activeVersionId)?.geometry.features.find(feature => finding.targetIds.includes(feature.id));
  const board = document.boards.find((item) => finding.targetIds.includes(item.id) || item.id === owner?.boardId || (outline && item.outlineIds.includes(outline.id)));
  const part = document.parts.find((item) => finding.targetIds.includes(item.id));
  const matrix = document.matrices.find((item) => finding.targetIds.includes(item.id));
  const body = document.caseBodies.find((item) => finding.targetIds.includes(item.id));
  const mechanicalLayer = assembly?.stack.find((layer) => finding.targetIds.includes(layer.id))
    ?? assembly?.stack.find((layer) => assembly.case.bodies.some((entry) => entry.body.id === layer.id && entry.body.mounts?.some((mount) => finding.targetIds.includes(mount.id))));
  const label = mechanicalLayer ? `Generated · ${mechanicalLayer.id}` : outline ? `${board?.name ?? 'Project'} · Outline` : part ? `${part.reference} · ${document.definitions.find((item) => item.id === part.definitionId)?.name ?? 'Component'}` : matrix?.name ?? body?.name ?? board?.name;
  return { outline, board, part, matrix, body, mechanicalLayer, label };
}
