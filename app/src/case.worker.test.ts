import { afterEach, expect, test, vi } from 'vitest';
import { cadStage } from '../../cad/src/metrics';

const preview = vi.hoisted(() => vi.fn());
const key = vi.hoisted(() => vi.fn(() => 'geometry'));
const build = vi.hoisted(() => vi.fn());
vi.mock('@boardstudio/v2-cad', () => ({ buildAssembly: build, readStepModel: vi.fn() }));
vi.mock('../../cad/src/preview', () => ({ previewBodies: preview, bodyKey: key }));

afterEach(() => { vi.unstubAllGlobals(); vi.resetModules(); vi.clearAllMocks(); });

test('worker diagnostics count transferred storage before detachment and reset for queued requests', async () => {
  const replies: any[] = [];
  const worker = { onmessage: undefined as ((event: MessageEvent) => void) | undefined,
    postMessage: (reply: unknown, transfer: Transferable[] = []) => replies.push(structuredClone(reply, { transfer })) };
  vi.stubGlobal('self', worker);
  await import('./case.worker');
  let sentMesh: Float32Array | undefined;
  preview.mockImplementation(async (ir, progress) => {
    const finish = cadStage('testBuild');
    const storage = new ArrayBuffer(72);
    const mesh = { positions: new Float32Array(storage, 0, 9), normals: new Float32Array(storage, 36, 9) };
    sentMesh = mesh.positions;
    progress({ revision: ir.revision, stage: 'building', total: 1, completed: 0 });
    finish();
    return { revision: ir.revision, bodies: [{ id: 'body', name: 'Body', ...mesh }] };
  });
  const send = (id: string, enabled: boolean, name = 'Body') => worker.onmessage!({ data: {
    id, kind: 'preview', ir: { revision: 7, bodies: [{ revision: 7, body: { id: 'body', name, kind: 'plate' }, regions: [] }] },
    ...(enabled ? { metricsSentAt: performance.timeOrigin + performance.now() } : {}),
  } } as MessageEvent);
  send('first', true);
  send('second', true);
  send('disabled', false);
  await vi.waitFor(() => expect(replies.filter(reply => reply.kind === 'preview')).toHaveLength(3));
  const results = replies.filter(reply => reply.kind === 'preview');
  expect(results[0].metrics.transferredBytes).toBe(72);
  expect(results[0].metrics.triangles).toBe(1);
  expect(results[0].metrics.stages.testBuild.calls).toBe(1);
  expect(results[1].metrics.transferredBytes).toBe(0);
  expect(results[1].metrics.triangles).toBe(0);
  expect(results[1].result.bodyIds).toEqual(['body']);
  for (const reply of results.slice(0, 1)) {
    expect(reply.result.bodies[0].positions.byteLength).toBe(36);
  }
  expect(results[2].metrics).toBeUndefined();
  expect(sentMesh!.byteLength).toBe(0);
  expect(replies.filter(reply => reply.kind === 'progress').every(reply => !reply.metrics)).toBe(true);
  expect((globalThis as any).__boardstudioCadMetrics).toBeUndefined();
});

test('failed requests return partial diagnostics and do not poison the next request', async () => {
  const replies: any[] = [];
  const worker = { onmessage: undefined as ((event: MessageEvent) => void) | undefined,
    postMessage: (reply: unknown) => replies.push(reply) };
  vi.stubGlobal('self', worker);
  await import('./case.worker');
  build.mockRejectedValueOnce(new Error('invalid geometry')).mockResolvedValueOnce({
    revision: 1, step: new Uint8Array(5), mesh: { positions: new Float32Array(9), normals: new Float32Array(9) },
  });
  for (const id of ['bad', 'good']) worker.onmessage!({ data: {
    id, kind: 'case', ir: { revision: 1, bodies: [] }, metricsSentAt: performance.timeOrigin + performance.now(),
  } } as MessageEvent);
  await vi.waitFor(() => expect(replies).toHaveLength(2));
  expect(replies[0]).toMatchObject({ kind: 'error', message: 'invalid geometry', metrics: { transferredBytes: 0, triangles: 0 } });
  expect(replies[1]).toMatchObject({ kind: 'case', metrics: { transferredBytes: 77, triangles: 1 } });
  expect((globalThis as any).__boardstudioCadMetrics).toBeUndefined();
});

test('cancels queued and active previews without advancing the completed delta cache', async () => {
  const replies: any[] = [];
  const worker = { onmessage: undefined as ((event: MessageEvent) => void) | undefined,
    postMessage: (reply: unknown) => replies.push(reply) };
  vi.stubGlobal('self', worker);
  await import('./case.worker');
  const send = (data: unknown) => worker.onmessage!({ data } as MessageEvent);
  const ir = { revision: 1, bodies: [{ revision: 1, body: { id: 'body', name: 'Body' }, regions: [] }] };
  let release!: () => void;
  preview.mockImplementationOnce(async (_ir, _progress, cancelled) => {
    await new Promise<void>(resolve => { release = resolve; });
    if (cancelled()) throw new Error('Preview superseded');
    return { revision: 1, bodies: [] };
  }).mockResolvedValue({ revision: 1, bodies: [{ id: 'body', name: 'Body', positions: new Float32Array(9), normals: new Float32Array(9) }] });
  send({ id: 'active', kind: 'preview', ir });
  await vi.waitFor(() => expect(release).toBeDefined());
  send({ id: 'queued', kind: 'preview', ir });
  send({ id: 'queued', kind: 'cancel-preview' });
  send({ id: 'active', kind: 'cancel-preview' });
  release();
  await vi.waitFor(() => expect(replies).toHaveLength(2));
  expect(replies.map(reply => reply.kind)).toEqual(['error', 'error']);
  expect(preview).toHaveBeenCalledTimes(1);
  send({ id: 'fresh', kind: 'preview', ir });
  await vi.waitFor(() => expect(replies).toHaveLength(3));
  expect(preview).toHaveBeenCalledTimes(2);
  expect(replies[2]).toMatchObject({ kind: 'preview', result: { bodyIds: ['body'], bodies: [{ id: 'body' }] } });
  send({ id: 'fresh', kind: 'cancel-preview' });
  send({ id: 'reuse', kind: 'preview', ir });
  await vi.waitFor(() => expect(replies).toHaveLength(4));
  expect(preview).toHaveBeenCalledTimes(2);
  expect(replies[3]).toMatchObject({ kind: 'preview', result: { bodies: [] } });
});

test('coalesces progress bursts but always delivers initial and terminal progress', async () => {
  const replies: any[] = [];
  const worker = { onmessage: undefined as ((event: MessageEvent) => void) | undefined,
    postMessage: (reply: unknown) => replies.push(reply) };
  vi.stubGlobal('self', worker);
  vi.spyOn(performance, 'now').mockReturnValue(0);
  await import('./case.worker');
  preview.mockImplementation(async (_ir, progress) => {
    progress({ revision: 1, stage: 'loading', total: 30, completed: 0 });
    progress({ revision: 1, stage: 'building', total: 30, completed: 0 });
    for (let completed = 0; completed <= 30; completed++) {
      progress({ revision: 1, stage: 'tessellating', total: 30, completed });
    }
    return { revision: 1, bodies: [] };
  });
  worker.onmessage!({ data: { id: 'progress', kind: 'preview', ir: { revision: 1,
    bodies: [{ revision: 1, body: { id: 'body', name: 'Body' }, regions: [] }] } } } as MessageEvent);
  await vi.waitFor(() => expect(replies.at(-1)?.kind).toBe('preview'));
  const progress = replies.filter(reply => reply.kind === 'progress');
  expect(progress).toHaveLength(3);
  expect(progress[0].progress.stage).toBe('loading');
  expect(progress[1].progress.stage).toBe('building');
  expect(progress.at(-1).progress.completed).toBe(30);
  vi.restoreAllMocks();
});
