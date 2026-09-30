import type { Part, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';
import { isErgogen, modelBindings } from '@boardstudio/v2-ergogen';
import type { ExportClient } from '../ExportClient';
import { bundledModel, bundledModelBytes } from '../bundledModels';
import { loadAsset } from '../storage';

function bindings(definition: PartDefinition, part?: Part): NonNullable<PartDefinition['models']> {
  let generated: NonNullable<PartDefinition['models']> = [];
  if (isErgogen(definition.generator?.source)) {
    try {
      generated = modelBindings(definition, part);
    } catch (cause) {
      generated = [{ assetId: `invalid-generator-model:${encodeURIComponent(String(cause))}`, offset: { x: 0, y: 0, z: 0 }, rotation: { x: 0, y: 0, z: 0 }, scale: { x: 1, y: 1, z: 1 } }];
    }
  }
  return [
    ...(definition.models ?? []),
    ...generated,
  ];
}
export async function modelFiles(document: ProjectDoc, definitions: PartDefinition[], parts: Part[] = []): Promise<{ paths: Map<string, string>; files: Record<string, Uint8Array> }> {
  const paths = new Map<string, string>();
  const files: Record<string, Uint8Array> = {};

  const modelIds = definitions.flatMap((definition) => {
    const instances = parts.filter((part) => part.definitionId === definition.id);
    return (instances.length ? instances : [undefined]).flatMap((part) => bindings(definition, part).map((model) => model.assetId));
  });

  for (const id of new Set(modelIds)) {
    const asset = document.assets.find((item) => item.id === id);
    const bundled = !asset ? bundledModel(id) : undefined;
    const name = asset?.name ?? bundled?.filename;
    const extension = name?.match(/\.(step|stp|stl|wrl)$/i)?.[1]?.toLowerCase();

    if (!asset || !extension) {
      if (!bundled || !extension) throw new Error(`Model asset ${id} needs a STEP, STP, or WRL filename`);
    }

    const bytes = asset ? await loadAsset(asset.sha256) : await bundledModelBytes(id);

    if (!bytes) {
      throw new Error(`Model asset ${name ?? id} is missing`);
    }

    const digest = asset?.sha256 ?? Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', new Uint8Array(bytes))),
      (byte) => byte.toString(16).padStart(2, '0')).join('');
    const path = `models/${digest}.${extension}`;

    paths.set(id, path);
    files[path] = bytes;
  }

  return { paths, files };
}

export async function packHandoff(files: Record<string, string | Uint8Array>, exporter: ExportClient): Promise<Uint8Array> {
  const entries = Object.keys(files).map((path, bufferIndex) => ({ path, bufferIndex }));
  const buffers = Object.values(files).map(value => typeof value === 'string' ? new TextEncoder().encode(value) : value);
  const packed = await exporter.archive({ kind: 'archive', request: { kind: 'pack-files', entries }, buffers });
  if (packed.reply.kind !== 'packed') throw new Error('Could not package the handoff');
  return packed.reply.bytes;
}
