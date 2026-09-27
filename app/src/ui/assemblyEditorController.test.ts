import { describe, expect, it } from 'vitest';
import type { AssemblyDefinition, AssemblyMember, Asset, PartDefinition, PartModel, ProjectDoc } from '@boardstudio/v2-contracts';
import { emptyProject } from '@boardstudio/v2-contracts';
import { attachAssemblyAsset, saveAssembly, updateAssemblyMember, updateAssemblyModel } from './assemblyEditorController';

const model: PartModel = { assetId: 'model', offset: { x: 0, y: 0, z: 0 }, rotation: { x: 0, y: 0, z: 0 }, scale: { x: 1, y: 1, z: 1 } };
const member: AssemblyMember = { id: 'member', definitionId: 'part', pose: { at: { x: 0, y: 0 }, rotation: 0 }, side: 'front', models: [model] };
const draft: AssemblyDefinition = { id: 'assembly', name: 'Assembly', members: [member] };
const definition = (id: string): PartDefinition => ({ id, name: id, kind: 'custom', courtyard: [], pads: [] });
const document = (overrides: Partial<ProjectDoc> = {}): ProjectDoc => ({ ...emptyProject('project', 'Project'), ...overrides });

describe('assembly editor controller', () => {
  it('validates, deduplicates definitions, and replaces an existing assembly', () => {
    const old: AssemblyDefinition = { id: draft.id, name: 'Old', members: [] };
    const existing = definition('part');
    const result = saveAssembly(document({ definitions: [existing], assemblies: [old] }), draft, [existing, definition('unused')]);
    expect(typeof result).toBe('object');
    expect(result).toMatchObject({ definitions: [existing], assemblies: [draft] });
    expect(saveAssembly(document(), { ...draft, name: ' ', members: [] }, [])).toBeTypeOf('string');
  });

  it('updates members and a real model immutably', () => {
    const changed = updateAssemblyMember(draft, member.id, { definitionId: 'other' });
    expect(changed.members[0].definitionId).toBe('other');
    const updated = updateAssemblyModel(draft, member, 0, { assetId: 'replacement' });
    expect(updated.members[0].models[0].assetId).toBe('replacement');
    expect(draft.members[0].models[0].assetId).toBe('model');
  });

  it('rejects unsupported files and a document changed while import is pending', async () => {
    const current = document();
    const store = async (): Promise<Asset> => ({ id: 'asset', name: 'model.stl', mediaType: 'model/stl', sha256: 'hash' });
    await expect(attachAssemblyAsset(new File(['x'], 'model.txt'), () => current, current, store)).rejects.toThrow('Choose a STEP');
    let release!: (asset: Asset) => void;
    const pending = new Promise<Asset>((resolve) => { release = resolve; });
    let latest = current;
    const importing = attachAssemblyAsset(new File(['x'], 'model.stl'), () => latest, current, () => pending);
    latest = document({ name: 'Reloaded' });
    release({ id: 'asset', name: 'model.stl', mediaType: 'model/stl', sha256: 'hash' });
    await expect(importing).rejects.toThrow('project changed');
  });
});
