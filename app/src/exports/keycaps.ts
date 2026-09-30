import type { ExportArtifact, ExportContext, ExportServices } from './context';

export async function exportKeycaps(context: ExportContext, services: ExportServices): Promise<ExportArtifact> {
  const { document, boardId } = context;
  if (!document.boards.some(board => board.id === boardId)) throw new Error('Select a board before export');
  const reply = await services.core().request({ id: crypto.randomUUID(), kind: 'resolve-keycaps', document, boardId, cases: null });
  context.assertCurrent();
  if (reply.kind !== 'keycaps-resolved') throw new Error(reply.kind === 'error' ? reply.message : 'Expected resolved keycaps');
  context.assertCurrent(reply.result.revision);
  const errors = reply.result.findings.filter(finding => finding.severity === 'error');
  if (errors.length) throw new Error(errors.map(finding => finding.message).join('\n'));
  if (!reply.result.specs.length) throw new Error('Choose a keycap profile in Keymap before export');
  const result = await services.cad().keycaps(document.revision, reply.result.specs, true);
  context.assertCurrent(result.revision);
  return { filename: `${document.name}-keycaps.step`, bytes: result.step, mediaType: 'model/step' };
}
