import { afterEach, expect, test, vi } from 'vitest';
import { recordCadMeasure, recordCadRequest, scheduleCadPaint, trackCadPreview } from './cadPerformance';

afterEach(() => { vi.unstubAllGlobals(); performance.clearMeasures(); });

function preview() {
  vi.stubGlobal('location', { search: '?cadMetrics=1' });
  vi.stubGlobal('document', { visibilityState: 'visible' });
  const frames = new Map<number, FrameRequestCallback>();
  let id = 0;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { frames.set(++id, callback); return id; });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
  const mesh = { positions: new Float32Array(9), normals: new Float32Array(9) };
  const result = { revision: 7, mesh, bodies: [{ id: 'plate', name: 'Plate', ...mesh }] };
  const scene = { bodies: [{ id: 'plate', mesh }] };
  const flush = () => { const pending = [...frames.values()]; frames.clear(); pending.forEach(callback => callback(0)); };
  return { result, scene, frames, flush };
}

test('records the first paint opportunity only after drawing a tracked visible preview', () => {
  const { result, scene, flush } = preview();
  trackCadPreview(result, performance.now(), () => true);
  scheduleCadPaint(scene, [], () => true);
  expect(performance.getEntriesByName('boardstudio.cad.preview.paint-opportunity')).toHaveLength(0);
  flush();
  const entries = performance.getEntriesByName('boardstudio.cad.preview.paint-opportunity') as PerformanceMeasure[];
  expect(entries).toHaveLength(1);
  expect(entries[0].detail).toMatchObject({ revision: 7, method: 'frame-after-draw' });
  scheduleCadPaint(scene, [], () => true);
  flush();
  expect(performance.getEntriesByName('boardstudio.cad.preview.paint-opportunity')).toHaveLength(1);
});

test('never reports stale, hidden, disposed or cancelled previews as painted', () => {
  const { result, scene, frames, flush } = preview();
  let current = true;
  trackCadPreview(result, performance.now(), () => current);
  scheduleCadPaint(scene, ['plate'], () => true);
  expect(frames.size).toBe(0);
  scheduleCadPaint(scene, [], () => true);
  current = false;
  flush();
  current = true;
  scheduleCadPaint(scene, [], () => false);
  flush();
  const cancel = scheduleCadPaint(scene, [], () => true);
  cancel();
  expect(frames.size).toBe(0);
  vi.stubGlobal('document', { visibilityState: 'hidden' });
  scheduleCadPaint(scene, [], () => true);
  flush();
  expect(performance.getEntriesByName('boardstudio.cad.preview.paint-opportunity')).toHaveLength(0);
});

test('profiling is opt-in and worker delivery uses timestamps normalized to their time origins', () => {
  const { result, scene, frames } = preview();
  vi.stubGlobal('location', { search: '' });
  trackCadPreview(result, performance.now(), () => true);
  scheduleCadPaint(scene, [], () => true);
  expect(frames.size).toBe(0);
  vi.stubGlobal('location', { search: '?cadMetrics=1' });
  const now = performance.timeOrigin + performance.now();
  recordCadRequest('request', 'preview', {
    stages: {}, counters: {}, wasmAllocatedBytes: { start: null, end: null, peak: null },
    requestSentAt: now - 30, receivedAt: now - 25, startedAt: now - 20, replySentAt: now - 5,
    transferredBytes: 144, requestTransferredBytes: 0, triangles: 1,
  });
  const [entry] = performance.getEntriesByName('boardstudio.cad.request') as PerformanceMeasure[];
  expect(entry.detail).toMatchObject({ requestDeliveryMs: 5, workerQueueMs: 5, workerExecutionMs: 15, transferredBytes: 144 });
  expect(entry.detail.responseDeliveryMs).toBeGreaterThanOrEqual(5);
});

test('bounds opt-in performance measures', () => {
  vi.stubGlobal('location', { search: '?cadMetrics=1' });
  for (let index = 0; index < 600; index += 1) {
    recordCadMeasure('boardstudio.test.measure', { start: 0, end: 0, detail: { index } });
  }
  const entries = performance.getEntriesByName('boardstudio.test.measure') as PerformanceMeasure[];
  expect(entries.length).toBeGreaterThan(0);
  expect(entries.length).toBeLessThanOrEqual(512);
  expect(entries.at(-1)?.detail).toEqual({ index: 599 });
  performance.clearMeasures('boardstudio.test.measure');
  recordCadMeasure('boardstudio.test.measure', { start: 0, end: 0, detail: { index: 600 } });
  expect(performance.getEntriesByName('boardstudio.test.measure')).toHaveLength(1);
  expect((performance.getEntriesByName('boardstudio.test.measure')[0] as PerformanceMeasure).detail).toEqual({ index: 600 });
});
