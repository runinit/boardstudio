import { caseAssembly } from '../caseAssembly';
import { exportMechanicalAssembly } from '../exportMechanicalAssembly';
import { effectiveCaseDocument, effectiveCaseScene } from '../hardwareInstances';
import { prepareCase } from '../prepareCase';
import { packHandoff } from './assets';
import type { ExportArtifact, ExportContext, ExportServices } from './context';

export async function exportCase(context: ExportContext, services: ExportServices): Promise<ExportArtifact> {
  const { document, scene, boardId } = context;
  const board = document.boards.find(board => board.id === boardId);
  if (!board) throw new Error('Select a board before export');
  if (!scene.boardReadiness.find(item => item.boardId === boardId)?.case) throw new Error('Resolve case findings before export');
  const prepared = await prepareCase(services.core(), caseAssembly(document, scene, boardId));
  context.assertCurrent(prepared.revision);
  const result = await services.cad().request(prepared);
  context.assertCurrent(result.revision);
  const suffix = document.boards.length === 1 ? '' : `-${board.name.replace(/[^a-zA-Z0-9_.-]+/g, '_')}`;
  return { filename: `${document.name}${suffix}-case.step`, bytes: result.step, mediaType: 'model/step' };
}

export async function exportMechanical(context: ExportContext, services: ExportServices): Promise<ExportArtifact> {
  const { instance, generation } = context.snapshot;
  const document = effectiveCaseDocument(context.document, instance);
  const scene = effectiveCaseScene(context.document, context.scene, instance);
  if (generation.draft || generation.status !== 'ready' || generation.revision !== document.revision) throw new Error('Update the current preview before export');
  const configuration = document.mechanical;
  if (!configuration || configuration.boardId !== context.boardId) throw new Error('Enable a mechanical assembly for the selected board before export');
  if (!context.isPreviewCurrent() || scene.revision !== document.revision) throw new Error('The committed scene is still resolving');
  const contours = scene.boardContours.find(entry => entry.boardId === configuration.boardId)?.contours ?? [];
  const files = await exportMechanicalAssembly({ document, contours, core: services.core(), cad: services.cad(), exporter: services.exporter(), isCurrent: context.isPreviewCurrent });
  context.assertCurrent();
  const bytes = await packHandoff(files, services.exporter());
  if (!context.isPreviewCurrent()) throw new Error('The assembly changed during export; export the current revision again');
  return { filename: `${document.name}${instance ? `-${instance.name.replace(/[^a-zA-Z0-9_-]/g, '-')}` : ''}-mechanical.zip`, bytes, mediaType: 'application/zip' };
}
