import { describe, expect, it, vi } from 'vitest';
import { emptyProject, type CoreReply, type CoreRequest } from '@boardstudio/v2-contracts';
import type { CoreClient } from './CoreClient';
import type { ExportClient } from './ExportClient';
import { createProjectActions } from './createProjectActions';
import { deleteProject, listProjects, loadProject, saveAsset, unpackProject, resetLocalProjects } from './storage';
import { catalogue } from '@boardstudio/v2-ergogen';

vi.mock('./storage', () => ({ deleteProject: vi.fn(), listProjects: vi.fn(), loadProject: vi.fn(), saveAsset: vi.fn(), unpackProject: vi.fn(), resetLocalProjects: vi.fn() }));

function harness() {
  const projectRef = { current: emptyProject('existing', 'Existing project') };
  const request = vi.fn(async (input: CoreRequest) => {
    if (input.kind !== 'open') throw new Error('Expected open');
    return { kind: 'scene', document: input.document, scene: { revision: input.document.revision } } as CoreReply;
  });
  const client = { current: { request } as unknown as CoreClient | null };
  const accept = vi.fn(async () => {});
  const onProjectCreated = vi.fn();
  let work: (() => Promise<void>) | undefined;
  const actions = createProjectActions({
    projectRef, client, exportClient: { current: null }, selectedInstance: undefined,
    schedule: next => { work = next; }, accept,
    ensureExportClient: () => null as unknown as ExportClient,
    onProjectCreated,
  });
  return { actions, client, request, accept, projectRef, onProjectCreated, run: () => work!() };
}

describe('saved keyboards', () => {
  it('loads the saved document when its queued open runs', async () => {
    const test = harness();
    const saved = emptyProject('saved', 'My keyboard');
    vi.mocked(loadProject).mockResolvedValueOnce(saved);
    test.actions.openSavedProject(saved.id);
    expect(test.request).not.toHaveBeenCalled();
    await test.run();
    expect(test.request).toHaveBeenCalledWith(expect.objectContaining({ kind: 'open', document: saved }));
    expect(test.accept).toHaveBeenCalledWith(expect.objectContaining({ document: saved }), 'open');
    expect(test.onProjectCreated).not.toHaveBeenCalled();
  });

  it('leaves the working document alone when storage is unavailable or a keyboard is missing', async () => {
    const test = harness();
    test.actions.openSavedProject('missing');
    vi.mocked(loadProject).mockResolvedValueOnce(undefined);
    await expect(test.run()).rejects.toThrow('no longer saved');
    vi.mocked(loadProject).mockRejectedValueOnce(new Error('Storage unavailable'));
    test.actions.openSavedProject('saved');
    await expect(test.run()).rejects.toThrow('Storage unavailable');
    expect(test.request).not.toHaveBeenCalled();
    expect(test.accept).not.toHaveBeenCalled();
  });

  it('restores the working core document when accepting a saved keyboard fails', async () => {
    const test = harness();
    const original = test.projectRef.current;
    vi.mocked(loadProject).mockResolvedValueOnce(emptyProject('saved', 'Saved keyboard'));
    test.accept.mockRejectedValueOnce(new Error('Storage full'));
    test.actions.openSavedProject('saved');
    await expect(test.run()).rejects.toThrow('Storage full');
    expect(test.request).toHaveBeenCalledTimes(2);
    expect(test.request).toHaveBeenLastCalledWith(expect.objectContaining({ kind: 'open', document: original }));
    expect(test.projectRef.current).toBe(original);
  });

  it('does not reopen the active keyboard and reset its history', async () => {
    const test = harness();
    test.actions.openSavedProject(test.projectRef.current.id);
    await test.run();
    expect(test.request).not.toHaveBeenCalled();
    expect(test.accept).not.toHaveBeenCalled();
  });
});

describe('file import limits', () => {
  const mib = 1024 * 1024;

  function file(name: string, size: number) {
    // Model the metadata independently so boundary tests do not allocate 128 MiB.
    const arrayBuffer = vi.fn(async () => new Uint8Array([1, 2, 3]).buffer);
    return { name, size, arrayBuffer } as unknown as File & { arrayBuffer: typeof arrayBuffer };
  }

  it.each([0, 128 * mib + 1])('rejects a %i-byte project before reading it', async size => {
    vi.mocked(unpackProject).mockClear();
    const test = harness();
    const original = test.projectRef.current;
    const input = file('project.boardstudio', size);
    test.actions.importProject(input);
    await expect(test.run()).rejects.toThrow(/128 MiB/);
    expect(input.arrayBuffer).not.toHaveBeenCalled();
    expect(unpackProject).not.toHaveBeenCalled();
    expect(test.request).not.toHaveBeenCalled();
    expect(test.accept).not.toHaveBeenCalled();
    expect(test.projectRef.current).toBe(original);
  });

  it.each([1, 128 * mib])('allows a %i-byte project through archive validation', async size => {
    const test = harness();
    const document = emptyProject('imported', 'Imported');
    vi.mocked(unpackProject).mockResolvedValueOnce(document);
    const input = file('project.boardstudio', size);
    test.actions.importProject(input);
    await test.run();
    expect(input.arrayBuffer).toHaveBeenCalledOnce();
    expect(test.request).toHaveBeenCalledWith(expect.objectContaining({ kind: 'open', document }));
  });

  it.each(['step', 'stp', 'stl', 'wrl'])('checks %s model size before reading or saving', async extension => {
    for (const size of [0, 32 * mib + 1]) {
      vi.mocked(saveAsset).mockClear();
      const test = harness();
      const original = test.projectRef.current;
      const input = file(`part.${extension}`, size);
      test.actions.importModel(input, catalogue()[0].id);
      await expect(test.run()).rejects.toThrow(/32 MiB/);
      expect(input.arrayBuffer).not.toHaveBeenCalled();
      expect(saveAsset).not.toHaveBeenCalled();
      expect(test.request).not.toHaveBeenCalled();
      expect(test.accept).not.toHaveBeenCalled();
      expect(test.projectRef.current).toBe(original);
    }
  });

  it.each([1, 32 * mib])('allows a %i-byte model through the existing import flow', async size => {
    vi.mocked(saveAsset).mockClear();
    const test = harness();
    test.request.mockResolvedValueOnce({ kind: 'scene', document: test.projectRef.current } as CoreReply);
    const input = file('part.step', size);
    test.actions.importModel(input, catalogue()[0].id);
    await test.run();
    expect(input.arrayBuffer).toHaveBeenCalledOnce();
    expect(saveAsset).toHaveBeenCalledOnce();
    expect(test.request).toHaveBeenCalledWith(expect.objectContaining({ kind: 'edit' }));
  });
});

describe('new-project setup entry', () => {
  it('opens the guide only after the new project has been accepted and saved', async () => {
    const test = harness();
    let finish!: () => void;
    test.accept.mockImplementation(() => new Promise<void>(resolve => { finish = resolve; }));
    test.actions.newProject();
    expect(test.onProjectCreated).not.toHaveBeenCalled();
    const pending = test.run();
    await vi.waitFor(() => expect(test.accept).toHaveBeenCalledOnce());
    expect(test.onProjectCreated).not.toHaveBeenCalled();
    finish();
    await pending;
    const opened = test.request.mock.calls[0][0];
    if (opened.kind !== 'open') throw new Error('Expected open');
    expect(test.onProjectCreated).toHaveBeenCalledExactlyOnceWith(opened.document.id);
    expect(opened.document.boards).toHaveLength(1);
    expect(opened.document.parts).toHaveLength(0);
  });

  it('does not open setup when creating or saving the project fails', async () => {
    const test = harness();
    test.actions.newProject();
    test.request.mockRejectedValueOnce(new Error('Worker unavailable'));
    await expect(test.run()).rejects.toThrow('Worker unavailable');
    test.accept.mockRejectedValueOnce(new Error('Storage full'));
    test.actions.newProject();
    await expect(test.run()).rejects.toThrow('Storage full');
    expect(test.onProjectCreated).not.toHaveBeenCalled();
  });

  it('does not open setup before the core is ready', async () => {
    const test = harness();
    test.client.current = null;
    test.actions.newProject();
    await test.run();
    expect(test.request).not.toHaveBeenCalled();
    expect(test.onProjectCreated).not.toHaveBeenCalled();
  });
});

it('restores the working project when a demo cannot be wired', async () => {
  const test = harness();
  const previous = test.projectRef.current;
  test.request.mockImplementation(async (request: CoreRequest): Promise<CoreReply> => {
    if (request.kind === 'open') return { kind: 'scene', document: request.document } as CoreReply;
    return { id: request.id, kind: 'error', message: 'Demo wiring failed' } as CoreReply;
  });
  test.actions.openDemo('rgb');
  await expect(test.run()).rejects.toThrow('Demo wiring failed');
  expect(test.request).toHaveBeenLastCalledWith(expect.objectContaining({ kind: 'open', document: previous }));
  expect(test.accept).not.toHaveBeenCalled();
  expect(test.projectRef.current).toBe(previous);
});

it('rebases queued instance mechanical edits onto the latest committed revision', async () => {
  const projectRef = { current: emptyProject('mechanical', 'Mechanical') };
  const instance = { id:'left',name:'Left',boardId:'board',half:'left',role:'central',flipped:false,constructionLinked:true,controllerPartId:null,mechanical:null };
  projectRef.current.hardware = { topology:'split',transport:'wired',boards:[],instances:[instance],sharedConstruction:null };
  let work: (() => Promise<void>) | undefined;
  const request = vi.fn(async () => ({kind:'ack'} as unknown as CoreReply));
  const actions = createProjectActions({projectRef,client:{current:{request} as unknown as CoreClient},exportClient:{current:null},selectedInstance:instance,
    schedule:next => { work = next; },accept:async () => {},ensureExportClient:() => null as unknown as ExportClient});
  actions.edit({transactionId:'edit',targetIds:[],baseRevision:0,phase:'commit',operation:{kind:'set-mechanical',configuration:null}});
  projectRef.current = {...projectRef.current,revision:1};
  await work!();
  expect(request).toHaveBeenCalledWith(expect.objectContaining({command:expect.objectContaining({baseRevision:1})}));
});


describe('deleting saved keyboards', () => {
  it('deletes another keyboard without changing the active document', async () => {
    const t = harness();
    t.actions.deleteSavedProject('other');
    await t.run();
    expect(deleteProject).toHaveBeenCalledWith('other');
    expect(t.request).not.toHaveBeenCalled();
  });

  it('opens a remaining keyboard before removing the current one', async () => {
    const t = harness();
    const remaining = emptyProject('remaining', 'Remaining keyboard');
    vi.mocked(listProjects).mockResolvedValueOnce([t.projectRef.current, remaining]);
    vi.mocked(deleteProject).mockImplementationOnce(async id => {
      expect(id).toBe('existing');
      expect(t.accept).toHaveBeenCalledWith(expect.objectContaining({document:remaining}), 'open');
    });
    t.actions.deleteSavedProject('existing');
    await t.run();
  });

  it('opens a new empty project before removing the last keyboard', async () => {
    const t = harness();
    vi.mocked(listProjects).mockResolvedValueOnce([t.projectRef.current]);
    t.actions.deleteSavedProject('existing');
    await t.run();
    const opened = t.request.mock.calls[0][0];
    expect(opened.kind).toBe('open');
    if (opened.kind === 'open') {
      expect(opened.document.id).not.toBe('existing');
      expect(opened.document.parts).toEqual([]);
      expect(opened.document.boards).toHaveLength(1);
    }
  });

  it('does not delete the current keyboard if its replacement cannot be saved', async () => {
    const t = harness();
    vi.mocked(listProjects).mockResolvedValueOnce([]);
    t.accept.mockRejectedValueOnce(new Error('Storage unavailable'));
    const before = vi.mocked(deleteProject).mock.calls.length;
    t.actions.deleteSavedProject('existing');
    await expect(t.run()).rejects.toThrow('Storage unavailable');
    expect(vi.mocked(deleteProject).mock.calls).toHaveLength(before);
  });

});

it('restores the working core document if resetting storage fails', async () => {
  const test = harness();
  vi.mocked(resetLocalProjects).mockRejectedValueOnce(new Error('Storage unavailable'));
  test.actions.newProject('reset');
  await expect(test.run()).rejects.toThrow('Storage unavailable');
  expect(test.request).toHaveBeenLastCalledWith(expect.objectContaining({ kind: 'open', document: test.projectRef.current }));
  expect(test.accept).not.toHaveBeenCalled();
  expect(test.onProjectCreated).not.toHaveBeenCalled();
});

it('resolves default closure screws and commits them with case settings in one edit', async () => {
  const { demoProject } = await import('./demo');
  const { createMechanicalConfiguration } = await import('./mechanicalPresets');
  const test = harness();
  const document = demoProject();
  test.projectRef.current = document;
  test.request.mockImplementation(async (input: CoreRequest) => {
    if (input.kind === 'snapshot') return { kind: 'scene', document, scene: { boardContours: [], transforms: [] } } as unknown as CoreReply;
    if (input.kind === 'resolve-mechanical') return { kind: 'mechanical-resolved', assembly: { suggestedMounts: [{ id: 'one', at: { x: 30, y: 20 }, kind: 'hole', holeDiameter: 3, bossDiameter: 6 }] } } as unknown as CoreReply;
    return { kind: 'scene', document, scene: {} } as CoreReply;
  });
  test.actions.edit({ baseRevision: document.revision, transactionId: 'case', phase: 'commit', targetIds: [], operation: { kind: 'set-mechanical', configuration: createMechanicalConfiguration(document) } });
  await test.run();
  const edits = test.request.mock.calls.map(([request]) => request).filter(request => request.kind === 'edit');
  expect(edits).toHaveLength(1);
  const operation = edits[0].kind === 'edit' ? edits[0].command.operation : undefined;
  expect(operation?.kind).toBe('replace-document');
  if (operation?.kind === 'replace-document') {
    expect(operation.document.mechanical?.closureMounts?.[0].kind).toBe('boss');
    expect(operation.document.parts.some(part => part.id.startsWith('case-closure/'))).toBe(true);
  }
});

it('bases queued mechanical edits on the document used to construct their replacement', async () => {
  const test = harness();
  test.request.mockImplementation(async () => ({ kind: 'scene', document: test.projectRef.current, scene: { revision: 8 } }) as CoreReply);
  test.actions.edit({ baseRevision: 2, transactionId: 'queued-case', phase: 'commit', targetIds: [], operation: { kind: 'set-mechanical', configuration: null } });
  test.projectRef.current = { ...test.projectRef.current, revision: 8 };
  await test.run();
  expect(test.request).toHaveBeenCalledWith(expect.objectContaining({ command: expect.objectContaining({ baseRevision: 8 }) }));
});

it('retains stale-revision protection for caller-authored document replacements', async () => {
  const test = harness();
  test.request.mockImplementation(async () => ({ kind: 'error', message: 'Stale base revision', revision: 8 }) as CoreReply);
  const document = test.projectRef.current;
  test.actions.edit({ baseRevision: 2, transactionId: 'authored-replacement', phase: 'commit', targetIds: [], operation: { kind: 'replace-document', document } });
  test.projectRef.current = { ...document, revision: 8 };
  await test.run();
  expect(test.request).toHaveBeenCalledWith(expect.objectContaining({ command: expect.objectContaining({ baseRevision: 2 }) }));
});
