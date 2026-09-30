import { firmwareRequest } from '../firmwareHandoff';
import { packHandoff } from './assets';
import type { ExportArtifact, ExportContext, ExportServices } from './context';

export async function exportFirmware(context: ExportContext, services: ExportServices): Promise<ExportArtifact> {
  const { document, boardId } = context;
  if (!document.boards.some(board => board.id === boardId)) throw new Error('Select a board before export');
  const instances = document.hardware?.instances ?? [];
  const split = document.hardware?.topology === 'split';
  const central = instances.find(instance => instance.role === 'central');
  const peripheral = instances.find(instance => instance.role === 'peripheral');
  if (split && (instances.length !== 2 || !central || !peripheral || central.half !== 'left' || peripheral.half !== 'right')) {
    throw new Error('Split firmware requires a left central and a right peripheral assembly');
  }
  const primary = await services.resolveWiring(document, central?.boardId ?? boardId, central?.id ?? null);
  context.assertCurrent(primary.revision);
  const secondary = peripheral ? await services.resolveWiring(document, peripheral.boardId, peripheral.id) : undefined;
  context.assertCurrent(secondary?.revision);
  const handoff = firmwareRequest(document, primary, secondary);
  const reply = await services.core().request({ id: crypto.randomUUID(), kind: 'generate-firmware', request: handoff.request });
  context.assertCurrent();
  if (reply.kind !== 'firmware-generated') throw new Error(reply.kind === 'error' ? reply.message : 'Expected generated firmware');
  const files = { ...reply.package.files, 'electrical-plan.json': JSON.stringify({ central: primary, peripheral: secondary }, null, 2) };
  const bytes = await packHandoff(files, services.exporter());
  return { filename: `${document.name}-zmk.zip`, bytes, mediaType: 'application/zip' };
}
