// Internal diagnostics shared by the CAD adapter and worker, not a CAD entry point.
type CadMetrics = {
  stages: Record<string, { durationMs: number; calls: number }>;
  counters: Record<string, number>;
  bodies?: { id: string; name: string; durationMs: number; stages: Record<string, number> }[];
  wasmAllocatedBytes: { start: number | null; end: number | null; peak: number | null };
};

type MetricsGlobal = typeof globalThis & { __boardstudioCadMetrics?: CadMetrics };
const scope = globalThis as MetricsGlobal;

export function beginCadMetrics(): CadMetrics {
  const metrics: CadMetrics = { stages: {}, counters: {}, wasmAllocatedBytes: { start: null, end: null, peak: null } };
  scope.__boardstudioCadMetrics = metrics;
  return metrics;
}

export function endCadMetrics(): void {
  delete scope.__boardstudioCadMetrics;
}

export function cadStage(name: string): () => void {
  const metrics = scope.__boardstudioCadMetrics;
  if (!metrics) return () => {};
  const start = performance.now();
  return () => {
    const stage = metrics.stages[name] ??= { durationMs: 0, calls: 0 };
    stage.durationMs += performance.now() - start;
    stage.calls++;
  };
}

export function countCadMetric(name: string, value: number): void {
  const metrics = scope.__boardstudioCadMetrics;
  if (metrics) metrics.counters[name] = (metrics.counters[name] ?? 0) + value;
}

export function cadBody(id: string, name: string): () => void {
  const metrics = scope.__boardstudioCadMetrics;
  if (!metrics) return () => {};
  const start = performance.now();
  const before = Object.fromEntries(Object.entries(metrics.stages).map(([key, value]) => [key, value.durationMs]));
  return () => {
    const bodies = metrics.bodies ??= [];
    if (bodies.length >= 128) {
      countCadMetric('omittedBodyTimings', 1);
      return;
    }
    const stages = Object.fromEntries(Object.entries(metrics.stages)
      .map(([key, value]) => [key, value.durationMs - (before[key] ?? 0)]).filter(([, duration]) => Number(duration) > 0));
    bodies.push({ id, name, durationMs: performance.now() - start, stages });
  };
}

export function sampleCadMemory(memory: WebAssembly.Memory | undefined): void {
  const metrics = scope.__boardstudioCadMetrics;
  if (!metrics || !memory) return;
  // Read a fresh buffer: memory growth invalidates earlier buffer references.
  const bytes = memory.buffer.byteLength;
  metrics.wasmAllocatedBytes.start ??= bytes;
  metrics.wasmAllocatedBytes.end = bytes;
  metrics.wasmAllocatedBytes.peak = Math.max(metrics.wasmAllocatedBytes.peak ?? 0, bytes);
}
