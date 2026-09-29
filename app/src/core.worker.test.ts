import { afterEach, expect, test, vi } from 'vitest';

const { request } = vi.hoisted(() => ({ request: vi.fn() }));
vi.mock('../../core/pkg/boardstudio_core.js', () => ({
  default: vi.fn(),
  CoreEngine: class { request = request; },
}));
afterEach(() => { vi.unstubAllGlobals(); vi.resetModules(); vi.clearAllMocks(); });

test('correlates document validation errors with the opening request', async () => {
  request.mockReturnValue(JSON.stringify({ id: '', kind: 'error', message: 'unknown field diodes', revision: 0 }));
  const worker = { onmessage: null as null | ((event: MessageEvent) => Promise<void>), postMessage: vi.fn() };
  vi.stubGlobal('self', worker);
  await import('./core.worker');
  await worker.onmessage!({ data: { id: 'open-saved', kind: 'open', document: {} } } as MessageEvent);
  expect(worker.postMessage).toHaveBeenCalledWith({ id: 'open-saved', kind: 'error', message: 'unknown field diodes', revision: 0 });
});
