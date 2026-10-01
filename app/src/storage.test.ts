import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createHash } from 'node:crypto';
import 'fake-indexeddb/auto';
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import { catalogue, modelAssetIds } from '@boardstudio/v2-ergogen';
import { demoProject } from './demo';
import { deleteProject, loadProject, saveProject, activeProjectId, loadAsset, saveAsset, packProject, unpackProject, type ArchiveTransport } from './storage';
import { nativeArchiveTransport } from './test/nativeArchive';

const archiveTransport: ArchiveTransport = nativeArchiveTransport();

describe('v2 project archive', () => {
  it('round trips the document without old format conversion', async () => {
    const project = demoProject();
    const archive = await packProject(project, { embedUsedModels: false }, archiveTransport);

    expect(await unpackProject(archive, archiveTransport)).toEqual(JSON.parse(JSON.stringify(project)));
  });

  it('can omit bundled models for catalog backed projects', async () => {
    const project = demoProject();
    const archive = await packProject(project, { embedUsedModels: false }, archiveTransport);

    expect(await unpackProject(archive, archiveTransport)).toEqual(JSON.parse(JSON.stringify(project)));
  });

  it('embeds used bundled models by default', async () => {
    const definition = catalogue().find((item) => item.generator?.source === 'infused-kim/switch_reset');
    expect(definition).toBeDefined();
    const modelId = modelAssetIds(definition!)[0];
    expect(modelId).toMatch(/^ergogen:model:/);
    const project = demoProject();
    project.definitions = [definition!];
    project.parts[0].definitionId = definition!.id;
    const bytes = strToU8('model bytes');
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, arrayBuffer: async () => bytes.buffer })));

    try {
      const archive = await packProject(project, {}, archiveTransport);
      const files = unzipSync(archive);
      const saved = JSON.parse(strFromU8(files['project.json'])) as typeof project;
      const asset = saved.assets.find((item) => item.id === modelId);
      expect(asset).toBeDefined();
      expect(files[`assets/${asset!.sha256}`]).toEqual(bytes);

      const withoutBundle = await packProject(project, { embedUsedModels: false }, archiveTransport);
      const unbundled = JSON.parse(strFromU8(unzipSync(withoutBundle)['project.json'])) as typeof project;
      expect(unbundled.assets.some((item) => item.id === modelId)).toBe(false);

    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('carries mounted-module snapshots, connections and used model bytes', async () => {
    const source = catalogue().find(item => item.generator?.source === 'infused-kim/switch_reset')!;
    const modelId = modelAssetIds(source)[0];
    const project = demoProject();
    project.moduleDefinitions = [{
      id:'portable-module',name:'Portable module',family:'expansion',variant:'review fixture',
      source:{repository:'https://github.com/sadekbaroudi/vik',revision:'cd5d16e4cd9137a229fc673412a89d75f4e64553',path:'pcb/vik-splitter',license:'CERN-OHL-S-2.0'},
      board:{contours:[],thickness:1.6},mounts:[],volumes:[],openings:[],interfaces:[],constituents:[],gates:[],
      electrical:{protocol:'pass-through',requiredSignals:[],logicVoltage:3.3},
      models:[{assetId:modelId,offset:{x:2,y:3,z:4},rotation:{x:0,y:0,z:45},scale:{x:1,y:1,z:1}}],
    }];
    project.modules=[{id:'portable-placement',definitionId:'portable-module',hostBoardId:project.boards[0].id,hostFace:'back',facingFace:'front',at:{x:10,y:20},rotation:30,gap:4,attachment:'case',detached:false,serviceClearance:1,
      connection:{hostConnectorPartId:'host',modulePortId:'input',busId:'shared-i2c',assignments:{scl:'P0',sda:'P1'},cableType:'type-a-12-0.5',railVoltages:{v3v3:3.3}}}];
    const bytes=strToU8('module model bytes');
    vi.stubGlobal('fetch',vi.fn(async()=>({ok:true,arrayBuffer:async()=>bytes.buffer})));
    const archive=await packProject(project,{},archiveTransport);
    const restored=await unpackProject(archive,archiveTransport);
    expect(restored.moduleDefinitions).toEqual(project.moduleDefinitions);
    expect(restored.modules).toEqual(project.modules);
    const asset=restored.assets.find(item=>item.id===modelId);
    expect(asset,'Mounted models must travel with the project').toBeDefined();
    expect(await loadAsset(asset!.sha256)).toEqual(bytes);
  });

  it('rejects non-v2 documents', async () => {
    const archive = zipSync({
      'project.json': strToU8(JSON.stringify({ format: 'ergogen/v1', parts: [], boards: [] })),
    });

    await expect(unpackProject(archive, archiveTransport)).rejects.toThrow('Unsupported project format');
  });

  it('rejects an oversized project before expanding its archive', async () => {
    const archive = zipSync({ 'project.json': strToU8('x'.repeat(9 * 1024 * 1024)) });

    await expect(unpackProject(archive, archiveTransport)).rejects.toThrow('Project archive exceeds size limit');
  });

  it('does not persist any assets when validation returns an incomplete asset set', async () => {
    const project = demoProject();
    const asset = { id: 'asset', name: 'asset.step', mediaType: 'model/step', sha256: 'a'.repeat(64), source: 'test' };
    project.assets = [asset];
    const transport: ArchiveTransport = {
      async archive() {
        return { kind: 'archive', reply: { kind: 'unpacked', projectJson: JSON.stringify(project), assets: [] } };
      },
    };

    await expect(unpackProject(new Uint8Array([1]), transport)).rejects.toThrow('Archive has no asset');
    await expect(loadAsset(asset.sha256)).resolves.toBeUndefined();
  });
});

const assetFixture = (label: string) => {
  const bytes = strToU8(label);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  return { bytes, asset: { id: label, name: `${label}.step`, mediaType: 'model/step', sha256, source: 'test' } };
};

it('packs a single entry for shared asset references and preserves their identities', async () => {
  const { bytes, asset } = assetFixture('shared asset');
  await saveAsset(asset.sha256, bytes);
  const project = { ...demoProject(), assets: [asset, { ...asset, id: 'second identity' }] };
  const packed = await packProject(project, { embedUsedModels: false }, archiveTransport);
  expect(Object.keys(unzipSync(packed)).filter((path) => path.startsWith('assets/'))).toEqual([`assets/${asset.sha256}`]);
  expect(await unpackProject(packed, archiveTransport)).toEqual(JSON.parse(JSON.stringify(project)));
});

it('validates all hashes before persisting any imported asset', async () => {
  const first = assetFixture('verified but not persisted');
  const second = assetFixture('hash failure');
  const project = { ...demoProject(), assets: [first.asset, second.asset] };
  const packed = zipSync({ 'project.json': strToU8(JSON.stringify(project)), [`assets/${first.asset.sha256}`]: first.bytes, [`assets/${second.asset.sha256}`]: strToU8('corrupted') });
  await expect(unpackProject(packed, archiveTransport)).rejects.toThrow('Asset hash mismatch');
  await expect(loadAsset(first.asset.sha256)).resolves.toBeUndefined();
  await expect(loadAsset(second.asset.sha256)).resolves.toBeUndefined();
});

it('rolls back every asset if the IndexedDB import transaction aborts', async () => {
  const first = assetFixture('rollback first');
  const second = assetFixture('rollback second');
  const project = { ...demoProject(), assets: [first.asset, second.asset] };
  const packed = zipSync({ 'project.json': strToU8(JSON.stringify(project)), [`assets/${first.asset.sha256}`]: first.bytes, [`assets/${second.asset.sha256}`]: second.bytes });
  const put = IDBObjectStore.prototype.put;
  const spy = vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementation(function (this: IDBObjectStore, value: unknown, key?: IDBValidKey) {
    const request = put.call(this, value, key);
    if (key === second.asset.sha256) this.transaction.abort();
    return request;
  });
  try {
    await expect(unpackProject(packed, archiveTransport)).rejects.toThrow();
  } finally {
    spy.mockRestore();
  }
  await expect(loadAsset(first.asset.sha256)).resolves.toBeUndefined();
  await expect(loadAsset(second.asset.sha256)).resolves.toBeUndefined();
});

it('rejects duplicate raw filenames before ZIP indexing', async () => {
  const json = strToU8(JSON.stringify(demoProject()));
  const bytes = zipSync({ 'project.json': json, 'archive.json': json });
  const from = strToU8('archive.json');
  for (let i = 0; i <= bytes.length - from.length; i++) {
    if (from.every((value, j) => bytes[i + j] === value)) bytes.set(strToU8('project.json'), i);
  }
  await expect(unpackProject(bytes, archiveTransport)).rejects.toThrow('Duplicate');
});

it('round trips routed sources, mapped meshes, and saved assembly members', async () => {
  const project = demoProject();
  const source = assetFixture('routed-source');
  source.asset.name = 'routed.kicad_pcb';
  const mesh = assetFixture('mapped-mesh');
  mesh.asset.name = 'body.stl';
  for (const fixture of [source, mesh]) await saveAsset(fixture.asset.sha256, fixture.bytes);
  project.assets = [source.asset, mesh.asset];
  project.boardReferences = [{id:'reference',boardId:'main-board',assetId:source.asset.id,enabled:true,pose:{at:{x:4,y:5},rotation:30},elevation:7,modelAssets:{'body.stl':mesh.asset.id}}];
  project.assemblies = [{id:'assembly',name:'Visual assembly',members:[{id:'body',side:'back',pose:{at:{x:2,y:3},rotation:45},modelMode:'custom',models:[{assetId:mesh.asset.id,offset:{x:0,y:0,z:2},rotation:{x:0,y:0,z:30},scale:{x:1,y:1,z:1}}]}]}];
  const archive = await packProject(project, { embedUsedModels: false }, archiveTransport);
  const restored = await unpackProject(archive, archiveTransport);
  expect(restored).toEqual(JSON.parse(JSON.stringify(project)));
  expect(await loadAsset(mesh.asset.sha256)).toEqual(mesh.bytes);
  expect(await loadAsset(source.asset.sha256)).toEqual(source.bytes);
});


describe('saved keyboard deletion', () => {
  beforeEach(() => {
    const values = new Map<string,string>();
    vi.stubGlobal('localStorage', {
      getItem: (key:string) => values.get(key) ?? null,
      setItem: (key:string,value:string) => values.set(key,value),
      removeItem: (key:string) => values.delete(key),
    });
  });
  afterEach(() => vi.unstubAllGlobals());

it('deletes only the selected keyboard and keeps shared assets and active selection', async () => {
  const project = demoProject();
  project.id = 'delete-target';
  await saveProject(project);
  const other = {...project,id:'keep-target'};
  await saveProject(other);
  const bytes = new Uint8Array([1,2,3]);
  await saveAsset('shared-delete-test', bytes);
  await deleteProject(project.id);
  expect(await loadProject(project.id)).toBeUndefined();
  expect(await loadProject(other.id)).toEqual(other);
  expect(activeProjectId('fallback')).toBe(other.id);
  expect(await loadAsset('shared-delete-test')).toEqual(bytes);
});

it('clears a deleted active keyboard reference', async () => {
  const project = {...demoProject(),id:'delete-active'};
  await saveProject(project);
  await deleteProject(project.id);
  expect(activeProjectId('fallback')).toBe('fallback');
});

  it('keeps the saved keyboard when the deletion transaction aborts', async () => {
    const project = {...demoProject(),id:'delete-aborted'};
    await saveProject(project);
    const remove = IDBObjectStore.prototype.delete;
    const spy = vi.spyOn(IDBObjectStore.prototype, 'delete').mockImplementationOnce(function (this: IDBObjectStore, key) {
      const request = remove.call(this, key);
      this.transaction.abort();
      return request;
    });
    try {
      await expect(deleteProject(project.id)).rejects.toBeDefined();
      expect(await loadProject(project.id)).toEqual(project);
      expect(activeProjectId('fallback')).toBe(project.id);
    } finally { spy.mockRestore(); }
  });

});
