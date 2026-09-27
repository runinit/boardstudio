import type { AssemblyDefinition, AssemblyMember, Asset, PartDefinition, PartModel, ProjectDoc } from '@boardstudio/v2-contracts';

export function updateAssemblyMember(draft: AssemblyDefinition, id: string, patch: Partial<AssemblyMember>): AssemblyDefinition {
  return { ...draft, members: draft.members.map((member) => member.id === id ? { ...member, ...patch } : member) };
}

export function updateAssemblyModel(draft: AssemblyDefinition, member: AssemblyMember, index: number, patch: Partial<PartModel>): AssemblyDefinition {
  return updateAssemblyMember(draft, member.id, {
    modelMode: 'custom',
    models: member.models.map((model, modelIndex) => modelIndex === index ? { ...model, ...patch } : model),
  });
}

export function saveAssembly(document: ProjectDoc, draft: AssemblyDefinition, definitions: PartDefinition[]): ProjectDoc | string {
  if (!draft.name.trim() || !draft.members.length) return 'Name the assembly and add at least one member';
  const needed = new Set(draft.members.map((member) => member.definitionId));
  return {
    ...document,
    definitions: [...document.definitions, ...definitions.filter((definition) => needed.has(definition.id) && !document.definitions.some((existing) => existing.id === definition.id))],
    assemblies: [...(document.assemblies ?? []).filter((assembly) => assembly.id !== draft.id), draft],
  };
}

export async function attachAssemblyAsset(
  file: File,
  currentDocument: () => ProjectDoc,
  documentAtStart: ProjectDoc,
  store: (file: File) => Promise<Asset>,
): Promise<{ asset: Asset; document: ProjectDoc }> {
  if (!/\.(step|stp|stl|wrl)$/i.test(file.name)) throw new Error('Choose a STEP, STL, or WRL file');
  const asset = await store(file);
  const document = currentDocument();
  if (document !== documentAtStart) throw new Error('The project changed while importing. Please import the model again.');
  return { asset, document: { ...document, assets: [...document.assets, asset] } };
}
