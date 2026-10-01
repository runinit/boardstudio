/** @vitest-environment jsdom */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, test, vi } from 'vitest';
import { emptyProject, type ProjectDoc, type SceneDelta } from '@boardstudio/v2-contracts';
import type { BodyPreview } from '../../cad/src/preview';
import { useCaseGeneration } from './useCaseGeneration';

vi.mock('./caseAssembly', () => ({ caseAssembly: (doc: ProjectDoc) => ({ revision: doc.revision,
  bodies: doc.caseBodies.map(body => ({ revision: doc.revision, body, regions: [] })),
}) }));
const result = (revision: number): BodyPreview => ({ revision, bodies: [{ id: 'tray', name: 'Tray', positions: new Float32Array([revision]), normals: new Float32Array([1]) }] });
const scene = (revision: number): SceneDelta => ({ revision, transforms: [], boardContours: [{ boardId: 'board', contours: [] }], boardReadiness: [{ boardId: 'board', outline: true, case: true }] } as unknown as SceneDelta);
const roots: ReturnType<typeof createRoot>[] = [];
const settle = async () => { await act(async () => { await new Promise(resolve => setTimeout(resolve, 80)); }); };

function harness() {
  let project = { ...emptyProject('p', 'P'), revision: 1 };
  let activeMode = 'Case', projectSession = 1;
  const projectRef = { current: project }, committedScene = { current: scene(1) };
  const request = vi.fn(async (message: any) => ({ id: message.id, kind: 'case-prepared', ir: message.ir }));
  const preview = vi.fn(async (ir: any, _progress?: unknown, _signal?: AbortSignal) => result(ir.revision));
  const cancel = vi.fn();
  const client = { current: { request } } as any, caseClient = { current: { preview, cancel } } as any;
  const previewCache = { current: new Map() };
  let output!: ReturnType<typeof useCaseGeneration>;
  function Harness() {
    output = useCaseGeneration({ project, scene: committedScene.current, selectedBoardId: 'board', selectedInstance: undefined,
      projectSession, projectRef, committedScene, client, caseClient, previewCache, activeMode, ready: true, setError: vi.fn() });
    return null;
  }
  const root = createRoot(document.createElement('div')); roots.push(root);
  async function render(revision = project.revision, options: { mode?: string; session?: number; id?: string; blockedMechanical?: boolean; boardSupportOnly?: boolean; unrelatedBlocker?: boolean } = {}) {
    project = { ...project, revision, id: options.id ?? project.id, caseBodies: [{ id: 'tray', name: 'Tray', boardId: 'board', kind: 'tray', thickness: revision, clearance: 0.5, materialId: 'pla' }] };
    projectRef.current = project;
    if (committedScene.current.revision !== revision || options.session || options.id) committedScene.current = scene(revision);
    if (options.blockedMechanical) {
      project.mechanical = { boardId: 'board' } as ProjectDoc['mechanical'];
      committedScene.current = { ...committedScene.current, boardReadiness: [{ boardId: 'board', outline: false, case: false }] } as SceneDelta;
      request.mockImplementation(async message => message.kind === 'resolve-mechanical'
        ? ({ id: message.id, kind: 'mechanical-resolved', assembly: {
          revision, case: { revision, bodies: [] }, generationBlocked: true, diagnostics: [
            ...(options.boardSupportOnly ? [{
              id: 'module/review/splitter-above/board-support/PART0', scope: 'pcb', severity: 'error', targetIds: ['review/splitter-above', 'board'], message: 'Configure a host drill',
            }] : []),
            ...(options.unrelatedBlocker ? [{
              id: 'mechanical:module/review/splitter-above/assembled-envelope', scope: 'case', severity: 'error', targetIds: ['review/splitter-above', 'board'], message: 'Geometry remains unqualified',
            }] : []),
          ], stack: [],
          gasketSupports: [], gasketTracks: [], generatedHardware: [], suggestedMounts: [], nominalPlateContours: [], plateContours: [],
        } } as any)
        : ({ id: message.id, kind: 'case-prepared', ir: message.ir } as any));
    }
    activeMode = options.mode ?? activeMode; projectSession = options.session ?? projectSession;
    await act(async () => root.render(<Harness />));
  }
  return { render, preview, cancel, previewCache, get value() { return output; }, get document() { return project; } };
}

afterEach(async () => { for (const root of roots.splice(0)) await act(async () => root.unmount()); vi.restoreAllMocks(); });

test('generates on entering Case and refreshes the latest committed revision', async () => {
  const h = harness(); await h.render(1, { mode: 'Design' }); await settle(); expect(h.preview).not.toHaveBeenCalled();
  await h.render(1, { mode: 'Case' }); await settle(); expect(h.value.generation.status).toBe('ready');
  await h.render(2); await settle(); expect(h.preview.mock.calls.map(([ir]) => ir.revision)).toEqual([1, 2]);
  expect(h.value.visibleCasePreview?.revision).toBe(2); expect(h.cancel).not.toHaveBeenCalled();
});

test('keeps one active request and replaces queued edits without terminating the worker', async () => {
  const h = harness(); let release!: (value: BodyPreview) => void;
  h.preview.mockImplementationOnce(() => new Promise(resolve => { release = resolve; }));
  await h.render(); await settle(); await h.render(2); await settle(); await h.render(3); await settle();
  expect(h.preview).toHaveBeenCalledTimes(1);
  expect(h.preview.mock.calls[0][2]?.aborted).toBe(true);
  await act(async () => release(result(1))); await settle();
  expect(h.preview.mock.calls.map(([ir]) => ir.revision)).toEqual([1, 3]);
  expect(h.value.visibleCasePreview?.revision).toBe(3); expect(h.cancel).not.toHaveBeenCalled();
});

test('draft geometry never enters committed cache or ready state, and Escape restores it', async () => {
  const h = harness(); await h.render(); await settle(); const committed = h.value.visibleCasePreview;
  const cache = [...h.previewCache.current.values()];
  const prepared = h.value.preparedCase;
  await act(async () => h.value.setPreviewDraft({ ...h.document, name: 'Transient' })); await settle();
  expect(h.value.preparedCase).toBe(prepared);
  expect(h.value.generation.draft).toBe(true); expect(h.value.generation.status).not.toBe('ready');
  expect([...h.previewCache.current.values()]).toEqual(cache); expect(h.value.visibleCasePreview).not.toBe(committed);
  await act(async () => h.value.setPreviewDraft(null));
  expect(h.value.visibleCasePreview).toBe(committed); expect(h.value.generation.draft).toBe(false); expect(h.value.generation.status).toBe('ready');
});

test('rejects a draft reply after its gesture ends', async () => {
  const h = harness(); await h.render(); await settle(); const committed = h.value.visibleCasePreview;
  let release!: (value: BodyPreview) => void;
  h.preview.mockImplementationOnce(() => new Promise(resolve => { release = resolve; }));
  await act(async () => h.value.setPreviewDraft({ ...h.document })); await settle();
  await act(async () => h.value.setPreviewDraft(null));
  await act(async () => release(result(1))); expect(h.value.visibleCasePreview).toBe(committed);
});

test('keeps a released draft visible until acknowledgement and reuses matching committed inputs', async () => {
  const h = harness(); await h.render(); await settle();
  const draft = { ...h.document, caseBodies: h.document.caseBodies.map(body => ({ ...body, thickness: 2 })) };
  await act(async () => h.value.setPreviewDraft(draft)); await settle();
  const visible = h.value.visibleCasePreview;
  await act(async () => h.value.setPreviewDraft(draft, 'commit'));
  expect(h.value.visibleCasePreview).toBe(visible);
  expect(h.value.generation.status).not.toBe('ready');
  await h.render(2); await settle();
  expect(h.preview).toHaveBeenCalledTimes(2);
  expect(h.value.visibleCasePreview?.bodies).toBe(visible?.bodies);
  expect(h.value.preparedCase?.revision).toBe(2);
  expect(h.value.preparedCase?.bodies[0].revision).toBe(2);
  expect(h.value.generation.status).toBe('ready');
});

test('discarding a failed commit restores the authoritative preview', async () => {
  const h = harness(); await h.render(); await settle(); const committed = h.value.visibleCasePreview;
  const draft = { ...h.document, name: 'Draft' };
  await act(async () => h.value.setPreviewDraft(draft)); await settle();
  await act(async () => h.value.setPreviewDraft(draft, 'commit'));
  await act(async () => h.value.setPreviewDraft(null));
  expect(h.value.visibleCasePreview).toBe(committed);
  expect(h.value.generation.draft).toBe(false);
});

test('pauses reactive controls, drops stale work, and still permits a manual update', async () => {
  const h = harness(); await h.render(); await settle();
  await act(async () => h.value.setLivePreview(false)); await h.render(2); await settle();
  expect(h.value.livePreview).toBe(false); expect(h.preview).toHaveBeenCalledTimes(1);
  expect(h.value.preparedCase).toBeUndefined();
  await act(async () => h.value.generateCase()); expect(h.value.visibleCasePreview?.revision).toBe(2);
  expect(h.value.preparedCase?.revision).toBe(2);
  await act(async () => h.value.cancelGeneration()); expect(h.value.livePreview).toBe(false); expect(h.cancel).toHaveBeenCalledOnce();
});

test.each([{ mode: 'Design' }, { session: 2 }, { id: 'other' }])('rejects old replies after changing context %j', async options => {
  const h = harness(); let release!: (value: BodyPreview) => void;
  let releaseCurrent!: (value: BodyPreview) => void;
  h.preview.mockImplementationOnce(() => new Promise(resolve => { release = resolve; }));
  h.preview.mockImplementationOnce(() => new Promise(resolve => { releaseCurrent = resolve; }));
  await h.render(); await settle(); await h.render(1, options);
  await act(async () => release(result(1)));
  expect(h.value.visibleCasePreview).toBeUndefined();
  expect(h.previewCache.current.size).toBe(0);
  if (!options.mode) { await settle(); await act(async () => releaseCurrent(result(1))); expect(h.value.generation.status).toBe('ready'); }
});

test('recovers from a failed CAD update on the next edit', async () => {
  const h = harness(); h.preview.mockRejectedValueOnce(new Error('geometry failed'));
  await h.render(); await settle(); expect(h.value.generation.status).toBe('failed');
  await h.render(2); await settle(); expect(h.value.visibleCasePreview?.revision).toBe(2); expect(h.value.generation.status).toBe('ready');
});


test('resolves mechanical diagnostics without a valid outline and never submits blocked CAD', async () => {
  const h = harness(); await h.render(1, { blockedMechanical: true }); await settle();
  expect(h.value.generation.status).toBe('blocked');
  expect(h.value.visibleMechanicalAssembly?.generationBlocked).toBe(true);
  expect(h.preview).not.toHaveBeenCalled();
});

test('previews safe case solids when only unconfigured board mounting drills block fabrication', async () => {
  const h = harness(); await h.render(1, { blockedMechanical: true, boardSupportOnly: true }); await settle();
  expect(h.preview).toHaveBeenCalledOnce();
  expect(h.value.visibleMechanicalAssembly?.generationBlocked).toBe(true);
  expect(h.value.visibleCasePreview?.revision).toBe(1);
  expect(h.value.generation.status).toBe('blocked');
  expect(h.value.generation.status).not.toBe('ready');
});

test('keeps blocking CAD preview when another case geometry error accompanies a missing drill', async () => {
  const h = harness(); await h.render(1, { blockedMechanical: true, boardSupportOnly: true, unrelatedBlocker: true }); await settle();
  expect(h.preview).not.toHaveBeenCalled();
  expect(h.value.visibleMechanicalAssembly?.generationBlocked).toBe(true);
  expect(h.value.generation.status).toBe('blocked');
});
