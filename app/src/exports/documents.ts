import { packProject } from '../storage';
import { modelFiles } from './assets';
import type { ExportArtifact, ExportContext, ExportServices } from './context';

export async function exportDocument(kind: 'project' | 'footprints' | 'svg' | 'dxf', context: ExportContext, services: ExportServices): Promise<ExportArtifact> {
  const { document, scene } = context;
  if (kind === 'project') {
    const bytes = await packProject(document, { embedUsedModels: context.snapshot.embedUsedModels }, services.exporter());
    return { filename: `${document.name}.boardstudio`, bytes, mediaType: 'application/zip' };
  }
  if (kind === 'footprints') {
    const { paths, files } = await modelFiles(document, document.definitions);
    context.assertCurrent();
    return services.exporter().request({ kind: 'footprints', document, paths: [...paths], files });
  }
  const board = document.boards.find(board => board.id === context.boardId);
  if (!board) throw new Error('Select a board before export');
  if (!scene.boardReadiness.find(item => item.boardId === board.id)?.outline) throw new Error('Resolve outline findings before export');
  const contours = scene.boardContours.find(entry => entry.boardId === board.id)?.contours ?? [];
  return services.exporter().request({ kind: 'outline', document, boardId: board.id, contours, outlineFormat: kind, paths: [], files: {} });
}
