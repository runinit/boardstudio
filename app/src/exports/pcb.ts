import type { ElectricalPlan, Part } from '@boardstudio/v2-contracts';
import { pcbAssemblyFiles } from '../electricalHandoff';
import { isWiringApplied } from '../electricalPlanContext';
import { modelFiles, packHandoff } from './assets';
import type { ExportArtifact, ExportContext, ExportServices } from './context';

async function prepareWiring(context: ExportContext, services: ExportServices, draft: boolean): Promise<ElectricalPlan> {
  const { document, boardId } = context;
  const plan = await services.resolveWiring(document, boardId);
  context.assertCurrent(plan.revision);
  const errors = plan.diagnostics.filter(finding => finding.severity === 'error');
  if (!draft && errors.length) throw new Error(errors.map(finding => finding.message).join('\n'));
  if (isWiringApplied(document, plan)) return plan;
  const reply = await services.core().request({ id: crypto.randomUUID(), kind: 'apply-electrical', baseRevision: document.revision, plan, draft });
  context.assertCurrent();
  if (reply.kind !== 'scene') throw new Error(reply.kind === 'error' ? reply.message : 'Expected applied wiring');
  await services.accept(reply, 'commit');
  context.adopt(reply.document);
  const applied = await services.resolveWiring(context.document, boardId);
  context.assertCurrent(applied.revision);
  return applied;
}

/** Owns wiring preparation and the protection record, including their ordering. */
export async function exportPcb(context: ExportContext, services: ExportServices, draft: boolean): Promise<ExportArtifact> {
  const board = context.document.boards.find(board => board.id === context.boardId);
  if (!board) throw new Error('Select a board before export');
  const plan = await prepareWiring(context, services, draft);
  const { document, scene } = context;
  if (!scene.boardReadiness.find(item => item.boardId === board.id)?.pcb) throw new Error('Resolve PCB findings before export');
  const usedParts = board.partIds.map(id => document.parts.find(part => part.id === id)).filter((part): part is Part => Boolean(part));
  const usedIds = new Set(usedParts.map(part => part.definitionId));
  const { paths, files } = await modelFiles(document, document.definitions.filter(definition => usedIds.has(definition.id)), usedParts);
  context.assertCurrent();
  const contours = scene.boardContours.find(entry => entry.boardId === board.id)?.contours ?? [];
  const result = await services.exporter().request({ kind: 'kicad', document, boardId: board.id, contours, paths: [...paths], files });
  context.assertCurrent();
  const handoffFiles: Record<string, string | Uint8Array> = { [result.filename]: result.bytes };
  const populations = [];
  for (const instance of document.hardware?.instances.filter(instance => instance.boardId === board.id) ?? []) {
    const population = await services.resolveWiring(document, board.id, instance.id);
    context.assertCurrent(population.revision);
    if (!draft && population.diagnostics.some(finding => finding.severity === 'error')) throw new Error(`Resolve wiring findings for ${instance.name} before export`);
    populations.push({ name: instance.name, plan: population });
  }
  Object.assign(handoffFiles, pcbAssemblyFiles(plan, draft, populations));
  const bytes = await packHandoff(handoffFiles, services.exporter());
  context.assertCurrent();
  // Only completed artifacts can protect assignments; saving must precede download.
  const reply = await services.core().request({ id: crypto.randomUUID(), kind: 'protect-electrical-handoff', baseRevision: document.revision, boardId: board.id, plan });
  context.assertCurrent();
  if (reply.kind !== 'scene') throw new Error(reply.kind === 'error' ? reply.message : 'Expected protected wiring');
  await services.accept(reply, 'commit');
  context.adopt(reply.document);
  return { filename: `${document.name}-${draft ? 'draft-' : ''}pcb-handoff.zip`, bytes, mediaType: 'application/zip' };
}
