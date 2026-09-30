import { afterEach, expect, test, vi } from 'vitest';
import type { CaseResult, PreparedCaseAssemblyIR } from '@boardstudio/v2-contracts';
import type { StepModel } from '@boardstudio/v2-cad';
import { CaseClient } from './CaseClient';

const workers: FakeWorker[] = [];

class FakeWorker {
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  sent: { message: unknown; transfer?: Transferable[] }[] = [];

  constructor() { workers.push(this); }
  postMessage(message: unknown, transfer?: Transferable[]): void { this.sent.push({ message, transfer }); }
  terminate(): void {}
  reply(data: unknown): void { this.onmessage?.({ data } as MessageEvent); }
  fail(): void { this.onerror?.({ preventDefault() {} } as ErrorEvent); }
}

afterEach(() => {
  workers.length = 0;
  vi.unstubAllGlobals();
});

test('keeps the prepared case worker protocol and transfers STEP and mesh buffers', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const ir = { revision: 3, bodies: [] } as PreparedCaseAssemblyIR;
  const pending = client.request(ir);
  const request = workers[0].sent[0].message as { id: string; kind: string; ir: PreparedCaseAssemblyIR };
  expect(request).toMatchObject({ kind: 'case', ir });

  const result: CaseResult = {
    revision: 3,
    step: new Uint8Array([1, 2]),
    mesh: { positions: new Float32Array([0, 1, 2]), normals: new Float32Array([0, 0, 1]) },
  };
  workers[0].reply({ id: request.id, kind: 'case', result });
  await expect(pending).resolves.toBe(result);
  client.close();
});

test('transfers a private copy when importing a STEP model', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const source = new Uint8Array([1, 2, 3]);
  const pending = client.requestModel(source);
  const sent = workers[0].sent[0];
  const request = sent.message as { id: string; kind: string; bytes: Uint8Array };
  expect(request.kind).toBe('model');
  expect(request.bytes).not.toBe(source);
  expect(sent.transfer).toEqual([request.bytes.buffer]);

  const result = { mesh: { positions: new Float32Array([0]), normals: new Float32Array([1]) }, bounds: { min: [0, 0, 0], max: [0, 0, 0] } } as StepModel;
  workers[0].reply({ id: request.id, kind: 'model', result });
  await expect(pending).resolves.toBe(result);
  expect(source).toEqual(new Uint8Array([1, 2, 3]));
  client.close();
});

test('rejects pending CAD work when its worker fails and serves later work on the replacement', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const pending = client.request({ revision: 4, bodies: [] } as PreparedCaseAssemblyIR);
  workers[0].fail();

  await expect(pending).rejects.toThrow('CAD worker failed and restarted');
  expect(workers).toHaveLength(2);

  const later = client.request({ revision: 5, bodies: [] } as PreparedCaseAssemblyIR);
  const nextRequest = workers[1].sent[0].message as { id: string; kind: string; ir: PreparedCaseAssemblyIR };
  expect(nextRequest).toMatchObject({ kind: 'case', ir: { revision: 5 } });
  const result = { revision: 5 } as CaseResult;
  workers[1].reply({ id: nextRequest.id, kind: 'case', result });
  await expect(later).resolves.toBe(result);
  client.close();
});

test('does not settle a request when progress arrives before its result', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const pending = client.request({ revision: 4, bodies: [] } as PreparedCaseAssemblyIR);
  const request = workers[0].sent[0].message as { id: string };
  let settled = false;
  void pending.then(() => { settled = true; }, () => { settled = true; });
  workers[0].reply({ id: request.id, kind: 'progress', progress: { stage: 'building', completed: 0, total: 2, revision: 4 } });
  await Promise.resolve();
  expect(settled).toBe(false);
  workers[0].reply({ id: request.id, kind: 'case', result: { revision: 4 } });
  await expect(pending).resolves.toMatchObject({ revision: 4 });
  client.close();
});

test('cancellation discards the old worker and ignores its late errors', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const pending = client.preview({ revision: 4, bodies: [] }, () => {});
  const old = workers[0];
  client.cancel();
  await expect(pending).rejects.toThrow('cancelled');
  old.fail();
  expect(workers).toHaveLength(2);
  const later = client.preview({ revision: 5, bodies: [] }, () => {});
  const request = workers[1].sent[0].message as { id: string };
  workers[1].reply({ id: request.id, kind: 'preview', result: { revision: 5 } });
  await expect(later).resolves.toMatchObject({ revision: 5 });
  client.close();
});

test('stale progress is ignored and stale terminal revisions are rejected', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const progress = vi.fn();
  const pending = client.preview({ revision: 6, bodies: [] }, progress);
  const request = workers[0].sent[0].message as { id: string };
  workers[0].reply({ id: request.id, kind: 'progress', progress: { revision: 5 } });
  expect(progress).not.toHaveBeenCalled();
  workers[0].reply({ id: request.id, kind: 'preview', result: { revision: 5 } });
  await expect(pending).rejects.toThrow('stale');
  client.close();
});

test('disposal rejects pending work and cannot restart the worker', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const pending = client.preview({ revision: 6, bodies: [] }, () => {});
  client.close();
  await expect(pending).rejects.toThrow('closed');
  workers[0].fail();
  expect(workers).toHaveLength(1);
  await expect(client.preview({ revision: 7, bodies: [] }, () => {})).rejects.toThrow('closed');
});

test('cooperative supersession waits for acknowledgement and keeps the warm worker', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const controller = new AbortController();
  const pending = client.preview({ revision: 6, bodies: [] }, () => {}, controller.signal);
  const request = workers[0].sent[0].message as { id: string };
  let settled = false;
  void pending.then(() => { settled = true; }, () => { settled = true; });
  controller.abort();
  expect(workers[0].sent[1].message).toEqual({ id: request.id, kind: 'cancel-preview' });
  await Promise.resolve();
  expect(settled).toBe(false);
  workers[0].reply({ id: request.id, kind: 'error', message: 'Preview superseded', revision: 6 });
  await expect(pending).rejects.toThrow('superseded');
  expect(workers).toHaveLength(1);
  client.close();
});

test('consumes a completed delta racing cancellation so the next delta can reuse it', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const controller = new AbortController();
  const pending = client.preview({ revision: 6, bodies: [] }, () => {}, controller.signal);
  const request = workers[0].sent[0].message as { id: string };
  const body = { id: 'body', name: 'Body', positions: new Float32Array([1]), normals: new Float32Array([1]) };
  controller.abort();
  workers[0].reply({ id: request.id, kind: 'preview', result: { revision: 6, bodyIds: ['body'], bodies: [body] } });
  await expect(pending).resolves.toMatchObject({ bodies: [body] });
  const later = client.preview({ revision: 7, bodies: [] }, () => {});
  const next = workers[0].sent.at(-1)!.message as { id: string };
  workers[0].reply({ id: next.id, kind: 'preview', result: { revision: 7, bodyIds: ['body'], bodies: [] } });
  await expect(later).resolves.toMatchObject({ bodies: [body] });
  client.close();
});

test('does not post already aborted previews or retain abort listeners after completion', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const aborted = new AbortController(); aborted.abort();
  await expect(client.preview({ revision: 1, bodies: [] }, () => {}, aborted.signal)).rejects.toThrow('superseded');
  expect(workers[0].sent).toHaveLength(0);
  const controller = new AbortController();
  const pending = client.preview({ revision: 1, bodies: [] }, () => {}, controller.signal);
  const request = workers[0].sent[0].message as { id: string };
  workers[0].reply({ id: request.id, kind: 'preview', result: { revision: 1, bodyIds: [], bodies: [] } });
  await pending;
  controller.abort();
  expect(workers[0].sent).toHaveLength(1);
  client.close();
});

test('keycap requests reject stale replies and cancelled late results without restarting CAD', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CaseClient();
  const stale = client.keycaps(7, []);
  const request = workers[0].sent[0].message as { id: string };
  workers[0].reply({ id: request.id, kind: 'case', result: { revision: 6 } });
  await expect(stale).rejects.toThrow('stale');
  const controller = new AbortController();
  const superseded = client.keycaps(8, [], false, controller.signal);
  const next = workers[0].sent[1].message as { id: string };
  controller.abort();
  expect(workers[0].sent[2].message).toEqual({ id: next.id, kind: 'cancel-preview' });
  workers[0].reply({ id: next.id, kind: 'case', result: { revision: 8 } });
  await expect(superseded).rejects.toThrow('superseded');
  expect(workers).toHaveLength(1);
  client.close();
});
