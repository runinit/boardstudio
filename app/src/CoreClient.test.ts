import { afterEach, expect, test, vi } from 'vitest';
import { emptyProject } from '@boardstudio/v2-contracts';
import type { CoreReply, SceneDelta } from '@boardstudio/v2-contracts';
import { CoreClient } from './CoreClient';

const workers: FakeWorker[] = [];

class FakeWorker {
  onmessage: ((event: MessageEvent<CoreReply>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  sent: unknown[] = [];

  constructor() {
    workers.push(this);
  }

  postMessage(request: unknown): void {
    this.sent.push(request);
  }

  terminate(): void {}

  reply(reply: CoreReply): void {
    this.onmessage?.({ data: reply } as MessageEvent<CoreReply>);
  }

  fail(): void {
    this.onerror?.({ preventDefault() {} } as ErrorEvent);
  }
}

const scene: SceneDelta = {
  revision: 0,
  transactionId: 'test',
  changedIds: [],
  transforms: [],
  matrixScenes: [],
  contours: [],
  boardContours: [],
  boardReadiness: [],
  findings: [],
  readiness: { layout: true, outline: false, pcb: false, case: false },
};

afterEach(() => {
  workers.length = 0;
  vi.unstubAllGlobals();
});

test('restores the committed document after a preview crashes the worker', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CoreClient();
  const committed = emptyProject('project', 'Committed');
  const preview = { ...committed, name: 'Uncommitted preview' };

  const opening = client.request({ id: 'open', kind: 'open', document: committed });
  workers[0].reply({ id: 'open', kind: 'scene', scene, document: committed });
  await opening;

  const preparing = client.request({ id: 'prepare', kind: 'prepare-case', ir: { revision: 0, bodies: [] } });
  workers[0].reply({ id: 'prepare', kind: 'case-prepared', ir: { revision: 0, bodies: [] } });
  await preparing;

  const pending = client.request({
    id: 'preview',
    kind: 'edit',
    command: { baseRevision: 0, phase: 'preview', transactionId: 'drag', targetIds: [], operation: { kind: 'replace-document', document: preview } },
  });
  workers[0].reply({ id: 'preview', kind: 'preview', scene });
  await pending;
  workers[0].fail();

  expect(workers[1].sent).toEqual([{ id: expect.any(String), kind: 'open', document: committed }]);
  client.close();
});

test('projects matrix drafts without changing the client snapshot', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CoreClient();
  const matrix = emptyProject('project', 'Draft').matrices;
  const pending = client.projectMatrices('draft', 7, matrix);
  expect(workers[0].sent).toEqual([{ id: 'draft', kind: 'project-matrices', baseRevision: 7, matrices: matrix }]);
  const reply: CoreReply = { id: 'draft', kind: 'matrix-projections', revision: 7, matrixScenes: [] };
  workers[0].reply(reply);
  await expect(pending).resolves.toEqual(reply);
  client.close();
});

test('closing settles pending requests and rejects subsequent work without restarting', async () => {
  vi.stubGlobal('Worker', FakeWorker);
  const client = new CoreClient();
  let reply: CoreReply | undefined;
  void client.request({ id: 'pending', kind: 'snapshot' }).then(result => { reply = result; });
  client.close();
  await Promise.resolve();
  expect(reply?.kind).toBe('error');
  await expect(client.request({ id: 'closed', kind: 'snapshot' })).resolves.toMatchObject({ id: 'closed', kind: 'error' });
  workers[0].fail();
  expect(workers).toHaveLength(1);
});
