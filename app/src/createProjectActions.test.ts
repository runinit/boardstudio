import { describe, expect, it, vi } from 'vitest';
import { emptyProject, type CoreReply, type CoreRequest } from '@boardstudio/v2-contracts';
import type { CoreClient } from './CoreClient';
import type { ExportClient } from './ExportClient';
import { createProjectActions } from './createProjectActions';

vi.mock('./storage', () => ({ saveAsset: vi.fn(), unpackProject: vi.fn() }));

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
  return { actions, client, request, accept, onProjectCreated, run: () => work!() };
}

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
