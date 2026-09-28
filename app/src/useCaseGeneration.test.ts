import { expect, test, vi } from 'vitest';
import type { CaseAssemblyIR, CoreReply, PreparedCaseAssemblyIR } from '@boardstudio/v2-contracts';
import type { BodyPreview } from '../../cad/src/preview';
import { generateCasePreview } from './useCaseGeneration';

const input = (revision: number): CaseAssemblyIR => ({ revision, bodies: [] });
const prepared = (revision: number): PreparedCaseAssemblyIR => ({ revision, bodies: [] });
const previewResult = (revision: number): BodyPreview => ({
  revision,
  bodies: [{ id: 'body', name: 'Body', positions: new Float32Array([revision]), normals: new Float32Array([0, 0, 1]) }],
});

function harness() {
  const core = { request: vi.fn() };
  const cad = { preview: vi.fn() };
  return { core, cad };
}

test('a late preparation reply for an older revision never starts CAD', async () => {
  const { core, cad } = harness();
  const replies = new Map<string, (reply: CoreReply) => void>();
  core.request.mockImplementation((request: { id: string; kind: string }) => new Promise((resolve) => replies.set(request.id, resolve)));
  cad.preview.mockResolvedValue(previewResult(2));
  let currentRevision = 1;

  const older = generateCasePreview(core as never, () => cad as never, input(1), () => currentRevision === 1, vi.fn());
  const oldId = core.request.mock.calls[0][0].id;
  currentRevision = 2;
  const newer = generateCasePreview(core as never, () => cad as never, input(2), () => currentRevision === 2, vi.fn());
  const newId = core.request.mock.calls[1][0].id;

  replies.get(newId)!({ id: newId, kind: 'case-prepared', ir: prepared(2) });
  await expect(newer).resolves.toEqual({ ...previewResult(2), prepared: prepared(2) });
  replies.get(oldId)!({ id: oldId, kind: 'case-prepared', ir: prepared(1) });
  await expect(older).resolves.toBeUndefined();
  expect(cad.preview).toHaveBeenCalledTimes(1);
  expect(cad.preview).toHaveBeenCalledWith(prepared(2), expect.any(Function));
});

test('a late CAD reply cannot replace a newer completed preview', async () => {
  const { core, cad } = harness();
  const requests = core.request.mockImplementation((request: { id: string; ir: CaseAssemblyIR }) => Promise.resolve({
    id: request.id,
    kind: 'case-prepared',
    ir: prepared(request.ir.revision),
  } satisfies CoreReply));
  const cadReplies = new Map<number, (value: BodyPreview) => void>();
  cad.preview.mockImplementation((ir: PreparedCaseAssemblyIR) => new Promise((resolve) => cadReplies.set(ir.revision, resolve)));
  let currentRevision = 1;
  let visible: BodyPreview | undefined;

  const older = generateCasePreview(core as never, () => cad as never, input(1), () => currentRevision === 1, vi.fn()).then((value) => {
    if (value) visible = value;
  });
  await vi.waitFor(() => expect(cadReplies.has(1)).toBe(true));

  currentRevision = 2;
  const newer = generateCasePreview(core as never, () => cad as never, input(2), () => currentRevision === 2, vi.fn()).then((value) => {
    if (value) visible = value;
  });
  await vi.waitFor(() => expect(cadReplies.has(2)).toBe(true));
  cadReplies.get(2)!(previewResult(2));
  await newer;
  cadReplies.get(1)!(previewResult(1));
  await older;

  expect(visible).toEqual({ ...previewResult(2), prepared: prepared(2) });
  expect(requests).toHaveBeenCalledTimes(2);
});

test('rejects a CAD payload tagged with a different captured revision', async () => {
  const { core, cad } = harness();
  core.request.mockResolvedValue({ id: 'prepare', kind: 'case-prepared', ir: prepared(7) });
  cad.preview.mockResolvedValue(previewResult(6));
  await expect(generateCasePreview(core as never, () => cad as never, input(7), () => true, vi.fn())).rejects.toThrow('CAD returned a different case revision');
});

test('forwards CAD progress and propagates preparation and CAD failures', async () => {
  const { core, cad } = harness();
  core.request.mockResolvedValue({ id: 'prepare', kind: 'case-prepared', ir: prepared(3) });
  const progress = { revision: 3, stage: 'building', completed: 1, total: 2 } as const;
  cad.preview.mockImplementation(async (_ir: PreparedCaseAssemblyIR, onProgress: (value: typeof progress) => void) => {
    onProgress(progress);
    return previewResult(3);
  });
  const received: typeof progress[] = [];
  await expect(generateCasePreview(core as never, () => cad as never, input(3), () => true, value => received.push(value as typeof progress))).resolves.toEqual({ ...previewResult(3), prepared: prepared(3) });
  expect(received).toEqual([progress]);

  core.request.mockRejectedValueOnce(new Error('prepare failed'));
  await expect(generateCasePreview(core as never, () => cad as never, input(3), () => true, vi.fn())).rejects.toThrow('prepare failed');
  cad.preview.mockRejectedValueOnce(new Error('CAD failed'));
  await expect(generateCasePreview(core as never, () => cad as never, input(3), () => true, vi.fn())).rejects.toThrow('CAD failed');
});

test('retains authoritative prepared regions without materializing the combined mesh', async () => {
  const { core, cad } = harness();
  const regions = prepared(3);
  core.request.mockResolvedValue({ id: 'prepare', kind: 'case-prepared', ir: regions });
  const mesh = vi.fn(() => { throw new Error('Combined mesh must stay lazy'); });
  cad.preview.mockResolvedValue(Object.defineProperty(previewResult(3), 'mesh', { enumerable: true, get: mesh }));
  const result = await generateCasePreview(core as never, () => cad as never, input(3), () => true, vi.fn());
  expect(result).toHaveProperty('prepared', regions);
  expect(mesh).not.toHaveBeenCalled();
});

test('reuses a completed exact draft only when every prepared body matches', async () => {
  const { core, cad } = harness();
  const body = { revision: 1, body: { id: 'body', name: 'Body', boardId: 'b', clearance: 0, kind: 'plate' as const, thickness: 2 }, regions: [] };
  const draft = { ...previewResult(1), prepared: { revision: 1, bodies: [body] } };
  const committed = { revision: 2, bodies: [{ ...body, revision: 2 }] };
  core.request.mockResolvedValue({ id: 'prepare', kind: 'case-prepared', ir: committed });
  cad.preview.mockResolvedValue(previewResult(2));
  const reused = await generateCasePreview(core as never, () => cad as never, input(2), () => true, vi.fn(), undefined, draft);
  expect(reused?.bodies).toBe(draft.bodies);
  expect(reused?.prepared).toBe(committed);
  expect(reused?.revision).toBe(2);
  expect(cad.preview).not.toHaveBeenCalled();
  for (const changed of [
    { ...body, revision: 2, body: { ...body.body, thickness: 3 } },
    { ...body, revision: 2, body: { ...body.body, id: 'other' } },
    { ...body, revision: 2, body: { ...body.body, name: 'Renamed' } },
    { ...body, revision: 2, regions: [{ outer: [], holes: [], cavities: [], gaskets: [], mounts: [] }] },
  ]) {
    core.request.mockResolvedValue({ id: 'prepare', kind: 'case-prepared', ir: { revision: 2, bodies: [changed] } });
    cad.preview.mockResolvedValue(previewResult(2));
    await generateCasePreview(core as never, () => cad as never, input(2), () => true, vi.fn(), undefined, draft);
  }
  expect(cad.preview).toHaveBeenCalledTimes(4);
});
