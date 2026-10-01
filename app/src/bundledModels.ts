/** Resolution for the read-only models shipped with the Ergogen library. */
type BundledModel = { id: string; url: string; filename: string };

const modelFiles = import.meta.glob('../../ergogen/library/vendor/*/3d_models/**/*.{step,stp,wrl,stl}', {
  eager: true,
  import: 'default',
  query: '?url',
}) as Record<string, string>;

const entries = Object.entries(modelFiles).map(([source, url]) => {
  const match = source.match(/vendor\/([^/]+)\/3d_models\/(.+)$/u);
  if (!match) return undefined;
  const [, vendor, sourceFilename] = match;
  // URL fragments in upstream filenames cannot be imported by Vite. Preserve saved IDs.
  const thqwgd001Names: Record<string, string> = {
    'THQWGD001-rotation.stp': 'THQWGD001 #1.stp',
    'THQWGD001C-2pin.stp': 'THQWGD001C [2pin] #1.stp',
    'THQWGD001C-4pin.stp': 'THQWGD001C [4pin] #1.stp',
  };
  const filename = vendor === 'thqwgd001' ? thqwgd001Names[sourceFilename] ?? sourceFilename : sourceFilename;
  return { id: `ergogen:model:${vendor}/${filename}`, url, filename };
}).filter((entry): entry is BundledModel => Boolean(entry));

const byId = new Map(entries.map((entry) => [entry.id, entry]));

export function bundledModel(id: string): BundledModel | undefined {
  return byId.get(id);
}

export async function bundledModelBytes(id: string): Promise<Uint8Array> {
  const model = bundledModel(id);
  if (!model) throw new Error(`Bundled Ergogen model is unavailable: ${id}`);
  const response = await fetch(model.url);
  if (!response.ok) throw new Error(`Could not load bundled model ${model.filename}`);
  return new Uint8Array(await response.arrayBuffer());
}

export function bundledModels(): readonly BundledModel[] { return entries; }
