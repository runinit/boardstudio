import { afterEach, expect, test, vi } from 'vitest';
import { previewBodies } from '../../cad/src/preview';
import { beginCadMetrics, endCadMetrics } from '../../cad/src/metrics';

const kernel = vi.hoisted(() => ({ preview_body: vi.fn() }));
vi.mock('../../cad/src/kernel', () => ({ getKernel: async () => kernel }));
const assembly = (count: number) => ({ revision: 1, bodies: Array.from({ length: count }, (_, index) => ({
  revision: 1, body: { id: String(index), name: String(index), boardId: 'b', clearance: 0, kind: 'plate' as const, thickness: 1 }, regions: [],
})) });

afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); endCadMetrics(); });

test('batches cheap bodies within a bounded work slice and reports terminal progress', async () => {
  vi.useFakeTimers();
  let now = 0;
  vi.spyOn(performance, 'now').mockImplementation(() => now);
  kernel.preview_body.mockImplementation(() => {
    now += 3;
    return { positions: new Float32Array(9), normals: new Float32Array(9) };
  });
  const progress = vi.fn();
  const metrics = beginCadMetrics();
  const pending = previewBodies(assembly(30), progress);
  await vi.runAllTimersAsync();
  expect((await pending).bodies).toHaveLength(30);
  expect(metrics.stages.bodyYield.calls).toBeLessThan(15);
  expect(metrics.stages.bodyYield.calls).toBeGreaterThan(3);
  expect(progress).toHaveBeenLastCalledWith(expect.objectContaining({ completed: 30, total: 30 }));
});

test('yields after an expensive body so cancellation prevents the next body', async () => {
  vi.useFakeTimers();
  let now = 0, cancelled = false;
  vi.spyOn(performance, 'now').mockImplementation(() => now);
  kernel.preview_body.mockImplementation(() => {
    now += 20;
    return { positions: new Float32Array(9), normals: new Float32Array(9) };
  });
  kernel.preview_body.mockClear();
  const pending = previewBodies(assembly(30), progress => {
    if (progress.completed === 1) setTimeout(() => { cancelled = true; }, 0);
  }, () => cancelled);
  const rejected = expect(pending).rejects.toThrow('superseded');
  await vi.runAllTimersAsync();
  await rejected;
  expect(kernel.preview_body).toHaveBeenCalledTimes(1);
});
