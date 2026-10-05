import type { ProjectDoc } from '@boardstudio/v2-contracts';
import { isErgogen, modelAssetIds } from '@boardstudio/v2-ergogen';
import { bundledModel, bundledModelBytes } from './bundledModels';

type ArchiveTransport = {
  archive(input: {
    kind: 'archive';
    request: { kind: 'pack-project'; projectJson: string; archiveJson: string; assets: { path: string; bufferIndex: number }[] };
    buffers: Uint8Array[];
  }): Promise<{ kind: 'archive'; reply: { kind: 'packed'; bytes: Uint8Array } }>;
};

/** Package the demo project through Core's normal portable archive operation. */
export async function packProject(doc: ProjectDoc, _options: unknown = {}, archiveClient: ArchiveTransport): Promise<Uint8Array> {
  const embeddedAssets = [...doc.assets];
  const files = new Map<string, Uint8Array>();
  const bundledIds = new Set(doc.definitions.flatMap(definition => {
    if (!isErgogen(definition.generator?.source)) return [];
    return doc.parts.filter(part => part.definitionId === definition.id).flatMap(part => modelAssetIds(definition, part));
  }));

  for (const definition of doc.definitions) {
    for (const model of definition.models ?? []) if (bundledModel(model.assetId)) bundledIds.add(model.assetId);
  }
  for (const definition of doc.moduleDefinitions ?? []) {
    for (const model of definition.models) if (bundledModel(model.assetId)) bundledIds.add(model.assetId);
    for (const part of definition.circuit?.definitions ?? []) for (const model of part.models ?? []) if (bundledModel(model.assetId)) bundledIds.add(model.assetId);
  }
  for (const assembly of doc.assemblies ?? []) for (const member of assembly.members) {
    for (const model of member.models) if (bundledModel(model.assetId)) bundledIds.add(model.assetId);
    const definition = doc.definitions.find(item => item.id === member.definitionId);
    if (definition && isErgogen(definition.generator?.source)) {
      const configured = { ...definition, generator: { ...definition.generator!, parameters: { ...definition.generator!.parameters, ...member.parameters } } };
      bundledIdsForMember(configured, member).forEach(id => bundledIds.add(id));
    }
  }
  for (const reference of doc.boardReferences ?? []) for (const id of Object.values(reference.modelAssets)) if (bundledModel(id)) bundledIds.add(id);
  for (const id of bundledIds) {
    if (embeddedAssets.some(asset => asset.id === id)) continue;
    const bundled = bundledModel(id);
    if (!bundled) throw new Error(`Bundled Ergogen model is unavailable: ${id}`);
    const bytes = await bundledModelBytes(id);
    const digest = await crypto.subtle.digest('SHA-256', new Uint8Array(bytes));
    const sha256 = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, '0')).join('');
    embeddedAssets.push({ id, name: bundled.filename, mediaType: /\.wrl$/i.test(bundled.filename) ? 'model/vrml' : /\.stl$/i.test(bundled.filename) ? 'model/stl' : 'model/step', sha256, source: 'bundled Ergogen library' });
    files.set(`assets/${sha256}`, bytes);
  }
  if (doc.assets.length) throw new Error('Demo project fixture unexpectedly requires browser-local project assets');
  const project = embeddedAssets.length === doc.assets.length ? doc : { ...doc, assets: embeddedAssets };
  const entries = [...files].map(([path, bytes]) => ({ path, bytes }));
  const result = await archiveClient.archive({
    kind: 'archive',
    request: {
      kind: 'pack-project', projectJson: JSON.stringify(project),
      archiveJson: JSON.stringify({ embedUsedModels: true }),
      assets: entries.map((entry, bufferIndex) => ({ path: entry.path, bufferIndex })),
    },
    buffers: entries.map(entry => entry.bytes),
  });
  if (result.kind !== 'archive' || result.reply.kind !== 'packed') throw new Error('Expected packed project archive');
  return result.reply.bytes;
}

function bundledIdsForMember(definition: Parameters<typeof modelAssetIds>[0], member: { id: string; definitionId?: string; pose: { at: { x: number; y: number }; rotation: number }; side: 'front' | 'back' }): string[] {
  return modelAssetIds(definition, { id: member.id, definitionId: definition.id, reference: member.id, pose: member.pose, side: member.side });
}
