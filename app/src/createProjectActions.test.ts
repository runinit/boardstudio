import { describe, expect, it, vi } from 'vitest';
import { emptyProject, type CoreReply, type CoreRequest } from '@boardstudio/v2-contracts';
import type { CoreClient } from './CoreClient';
import type { ExportClient } from './ExportClient';
import { createProjectActions } from './createProjectActions';
import { saveAsset, unpackProject, resetLocalProjects } from './storage';
import { catalogue } from '@boardstudio/v2-ergogen';

vi.mock('./storage', () => ({ saveAsset: vi.fn(), unpackProject: vi.fn(), resetLocalProjects: vi.fn() }));

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
