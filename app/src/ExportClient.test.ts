import { afterEach, expect, test, vi } from 'vitest';
import type { PartDefinition } from '@boardstudio/v2-contracts';
import { emptyProject } from '@boardstudio/v2-contracts';
import { ExportClient } from './ExportClient';
import type { ExportWorkerRequest, ExportWorkerReply } from './export.worker';

const workers: FakeWorker[] = [];

class FakeWorker {
  onmessage: ((event: MessageEvent<ExportWorkerReply>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  sent: ExportWorkerRequest[] = [];

  constructor() { workers.push(this); }
  postMessage(message: ExportWorkerRequest, transfer: Transferable[] = []): void { this.sent.push(structuredClone(message, { transfer })); }
  terminate(): void {}
  reply(reply: ExportWorkerReply): void { this.onmessage?.({ data: reply } as MessageEvent<ExportWorkerReply>); }
}

const definition = (id: string): PartDefinition => ({ id, name: id, kind: 'custom', pads: [], courtyard: [] });
const job = (id: string) => ({ id, definition: definition(id), side: 'front' as const });

afterEach(() => {
  workers.length = 0;
  vi.unstubAllGlobals();
});

test('deduplicates in-flight footprint compilation and resolves out-of-order replies by request id', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  vi.stubGlobal('crypto', { randomUUID: vi.fn().mockReturnValueOnce('a').mockReturnValueOnce('b') });
  const client = new ExportClient();
  const first = client.compile([job('first')]);
  const duplicate = client.compile([job('first')]);
  const second = client.compile([job('second')]);

  expect(workers[0].sent).toHaveLength(2);
  workers[0].reply({ id: 'b', kind: 'compile-footprints', result: [{ definition: definition('second'), geometry: { side: 'front', pads: [], courtyard: [], traces: [], vias: [] }, diagnostics: [], previewSvg: null }] });
  workers[0].reply({ id: 'a', kind: 'compile-footprints', result: [{ definition: definition('first'), geometry: { side: 'front', pads: [], courtyard: [], traces: [], vias: [] }, diagnostics: [], previewSvg: null }] });
  await expect(second).resolves.toMatchObject([{ definition: { id: 'second' } }]);
  await expect(first).resolves.toMatchObject([{ definition: { id: 'first' } }]);
  await expect(duplicate).resolves.toMatchObject([{ definition: { id: 'first' } }]);
  client.close();
});

test('rejects pending artifact promises when the client closes', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  vi.stubGlobal('crypto', { randomUUID: () => 'pending' });
  const client = new ExportClient();
  const pending = client.compile([job('first')]);
  client.close();
  await expect(pending).rejects.toThrow('Export worker was closed');
});

test('copies archive buffers before transfer and resolves archive replies', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  vi.stubGlobal('crypto', { randomUUID: () => 'archive' });
  const client = new ExportClient();
  const bytes = new Uint8Array([1, 2, 3]);
  const pending = client.archive({ kind: 'archive', request: { kind: 'pack-files', entries: [{ path: 'a', bufferIndex: 0 }] }, buffers: [bytes] });
  expect(workers[0].sent[0]).toMatchObject({ buffers: [new Uint8Array([1, 2, 3])] });
  expect(bytes.byteLength).toBe(3);
  expect(bytes).toEqual(new Uint8Array([1, 2, 3]));
  workers[0].reply({ id: 'archive', kind: 'archive', reply: { kind: 'packed', bytes } });
  await expect(pending).resolves.toMatchObject({ reply: { kind: 'packed' } });
  client.close();
});

test('closed workers cannot restart or accept new exports', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new ExportClient();
  client.close();
  workers[0].onerror?.({ preventDefault() {} } as ErrorEvent);
  expect(workers).toHaveLength(1);
  await expect(client.artifact({ kind: 'preview-board', source: '', revision: 0 })).rejects.toThrow('closed');
  await expect(client.preview({ document: emptyProject('closed', 'Closed'), boardId: 'board', contours: [], paths: [] })).rejects.toThrow('closed');
  await expect(client.archive({ kind: 'archive', request: { kind: 'pack-files', entries: [] }, buffers: [] })).rejects.toThrow('closed');
  await expect(client.request({ kind: 'footprints', document: emptyProject('closed', 'Closed'), paths: [], files: {} })).rejects.toThrow('closed');
  expect(workers[0].sent).toHaveLength(0);
});
