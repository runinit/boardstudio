import type { BodyPreview } from '../../cad/src/preview';
import type { CadMetrics } from '../../cad/src/metrics';

export type CadWorkerMetrics = CadMetrics & {
  requestSentAt: number;
  receivedAt: number;
  startedAt: number;
  replySentAt: number;
  transferredBytes: number;
  requestTransferredBytes: number;
  revision?: number;
  triangles: number;
};

export function cadProfilingEnabled(): boolean {
  return typeof location !== 'undefined' && new URLSearchParams(location.search).get('cadMetrics') === '1';
}

export function cadTimestamp(): number {
  return performance.timeOrigin + performance.now();
}

const MAX_CAD_MEASURES = 512;

export function recordCadMeasure(name: string, options: PerformanceMeasureOptions): void {
  if (!cadProfilingEnabled()) return;
  if (performance.getEntriesByName(name).length >= MAX_CAD_MEASURES) {
    performance.clearMeasures(name);
  }
  performance.measure(name, options);
}

export function recordCadRequest(id: string, kind: string, metrics: CadWorkerMetrics): void {
  const received = cadTimestamp();
  recordCadMeasure('boardstudio.cad.request', {
    start: Math.max(0, metrics.requestSentAt - performance.timeOrigin),
    end: performance.now(),
    detail: {
      id, kind, ...metrics,
      requestDeliveryMs: Math.max(0, metrics.receivedAt - metrics.requestSentAt),
      workerQueueMs: Math.max(0, metrics.startedAt - metrics.receivedAt),
      workerExecutionMs: Math.max(0, metrics.replySentAt - metrics.startedAt),
      responseDeliveryMs: Math.max(0, received - metrics.replySentAt),
    },
  });
}

type PreviewTiming = { start: number; revision: number; isCurrent: () => boolean; recorded: boolean };
const previews = new WeakMap<Float32Array, PreviewTiming>();

export function trackCadPreview(result: BodyPreview, start: number, isCurrent: () => boolean): void {
  if (!cadProfilingEnabled()) return;
  const timing = { start, revision: result.revision, isCurrent, recorded: false };
  for (const mesh of result.bodies) previews.set(mesh.positions, timing);
}

// Called after a successful draw or confirmation that identical geometry is already displayed.
export function scheduleCadPaint(scene: unknown, hidden: string[], isCurrent: () => boolean): () => void {
  if (!cadProfilingEnabled()) return () => {};
  const bodies = (scene as { bodies?: { id: string; mesh: { positions: Float32Array } }[] })?.bodies ?? [];
  const timings = new Set(bodies.filter(body => !hidden.includes(body.id) && body.mesh.positions.length > 0)
    .map(body => previews.get(body.mesh.positions)).filter((value): value is PreviewTiming => Boolean(value)));
  if (![...timings].some(timing => !timing.recorded && timing.isCurrent())) return () => {};
  const drawn = performance.now();
  // The next frame follows a paint opportunity. This is not a GPU/display timestamp.
  const frame = requestAnimationFrame(() => {
    if (!isCurrent() || typeof document === 'undefined' || document.visibilityState !== 'visible') return;
    for (const timing of timings) {
      if (timing.recorded || !timing.isCurrent()) continue;
      timing.recorded = true;
      recordCadMeasure('boardstudio.cad.preview.paint-opportunity', {
        start: timing.start, end: performance.now(),
        detail: { revision: timing.revision, drawnMs: drawn - timing.start, method: 'frame-after-draw' },
      });
    }
  });
  return () => cancelAnimationFrame(frame);
}
