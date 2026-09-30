import { ExportContext, type ExportArtifact, type ExportKind, type ExportServices, type ExportSnapshot } from './exports/context';
import { exportDocument } from './exports/documents';
import { exportKeycaps } from './exports/keycaps';
import { exportFirmware } from './exports/firmware';
import { exportPcb } from './exports/pcb';
import { exportCase, exportMechanical as buildMechanical } from './exports/cases';

function downloadArtifact({ filename: name, bytes, mediaType: type }: ExportArtifact): void {
  const content = typeof bytes === 'string' ? bytes : new Uint8Array(bytes);
  const url = URL.createObjectURL(new Blob([content], { type }));
  const anchor = document.createElement('a');

  anchor.href = url;
  anchor.download = name;
  anchor.click();

  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
type Inputs = {
  readSnapshot: () => ExportSnapshot;
  services: ExportServices;
  schedule: (work: () => Promise<void>) => void;
  deliver?: (artifact: ExportArtifact) => void;
};

/** Coordinates queued snapshot capture and delivery; workflows own artifact construction. */
export function createProjectExporter({ readSnapshot, services, schedule, deliver = downloadArtifact }: Inputs) {
  const enqueue = (kind: ExportKind | 'mechanical', boardId?: string): void => {
    const requestedScope = readSnapshot().previewContext;
    schedule(async () => {
      const context = new ExportContext(readSnapshot(), readSnapshot, boardId, requestedScope);
      let artifact: ExportArtifact;
      switch (kind) {
        case 'project': case 'footprints': case 'svg': case 'dxf':
          artifact = await exportDocument(kind, context, services); break;
        case 'keycaps-step': artifact = await exportKeycaps(context, services); break;
        case 'firmware': artifact = await exportFirmware(context, services); break;
        case 'kicad': case 'kicad-draft': artifact = await exportPcb(context, services, kind === 'kicad-draft'); break;
        case 'case-step': artifact = await exportCase(context, services); break;
        case 'mechanical': artifact = await buildMechanical(context, services); break;
      }
      context.assertCurrent();
      deliver(artifact);
    });
  };
  return {
    exportFile: (kind: ExportKind, boardId?: string) => enqueue(kind, boardId),
    exportMechanical: () => enqueue('mechanical'),
  };
}
