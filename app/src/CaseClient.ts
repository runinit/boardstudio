import type { CaseResult, KeycapSpec, PreparedCaseAssemblyIR } from '@boardstudio/v2-contracts';
import type { StepModel, CadProgress, CasePreviewResult } from '@boardstudio/v2-cad';
import { combinedPreview, type BodyPreview } from '../../cad/src/preview';
import { cadProfilingEnabled, cadTimestamp, recordCadRequest, type CadWorkerMetrics } from './cadPerformance';

type CaseReply =
  | { id: string; kind: 'progress'; progress: CadProgress }
  | { id: string; kind: 'preview'; result: PreviewDelta | BodyPreview }
  | { id: string; kind: 'case'; result: CaseResult }
  | { id: string; kind: 'model'; result: StepModel }
  | { id: string; kind: 'error'; message: string; revision: number };
type PreviewDelta = { revision: number; bodyIds: string[]; bodies: NonNullable<BodyPreview['bodies']> };

export class CaseClient {
  private worker: Worker;

  private progress = new Map<string, (value: CadProgress) => void>();
  private closed = false;

  private pending = new Map<string, (reply: CaseReply) => void>();
  private previewBodies = new Map<string, NonNullable<BodyPreview['bodies']>[number]>();

  constructor() {
    this.worker = this.start();
  }

  private start(): Worker {
    const worker = new Worker(new URL('./case.worker.ts', import.meta.url), { type: 'module' });

    worker.onmessage = (event: MessageEvent<CaseReply & { metrics?: CadWorkerMetrics }>) => {
      const resolve = this.pending.get(event.data.id);

      if (!resolve) {
        return;
      }

      if (event.data.kind === 'progress') {
        this.progress.get(event.data.id)?.(event.data.progress);
        return;
      }
      this.progress.delete(event.data.id);
      this.pending.delete(event.data.id);
      if (event.data.metrics) recordCadRequest(event.data.id, event.data.kind, event.data.metrics);
      resolve(event.data);
    };

    worker.onerror = (event) => {
      event.preventDefault();
      if (this.closed || worker !== this.worker) return;

      for (const [id, resolve] of this.pending) {
        resolve({ id, kind: 'error', message: 'CAD worker failed and restarted', revision: 0 });
      }

      this.pending.clear();
      this.progress.clear();
      this.previewBodies.clear();
      worker.terminate();
      this.worker = this.start();
    };

    return worker;
  }

  keycaps(revision: number, specs: KeycapSpec[], exportStep = false, signal?: AbortSignal): Promise<CaseResult> {
    if (this.closed) return Promise.reject(new Error('CAD client is closed'));
    if (signal?.aborted) return Promise.reject(new Error('Keycap preview superseded'));
    const id = crypto.randomUUID();
    const abort = () => this.worker.postMessage({ id, kind: 'cancel-preview' });
    return new Promise((resolve, reject) => {
      this.pending.set(id, reply => {
        signal?.removeEventListener('abort', abort);
        if (signal?.aborted) { reject(new Error('Keycap preview superseded')); return; }
        if (reply.kind === 'error') reject(new Error(reply.message));
        else if (reply.kind === 'case' && reply.result.revision === revision) resolve(reply.result);
        else reject(new Error('Unexpected or stale keycap CAD response'));
      });
      signal?.addEventListener('abort', abort, { once: true });
      this.worker.postMessage({ id, kind: 'keycaps', revision, specs, exportStep });
    });
  }

  request(ir: PreparedCaseAssemblyIR): Promise<CaseResult> {
    if (this.closed) return Promise.reject(new Error('CAD client is closed'));
    const id = crypto.randomUUID();

    return new Promise((resolve, reject) => {
      this.pending.set(id, (reply) => {
        if (reply.kind === 'error') {
          reject(new Error(reply.message));
          return;
        }

        if (reply.kind !== 'case') {
          reject(new Error('Unexpected CAD worker response'));
          return;
        }

        resolve(reply.result);
      });

      this.worker.postMessage({ id, kind: 'case', ir, ...(cadProfilingEnabled() ? { metricsSentAt: cadTimestamp() } : {}) });
    });
  }

  preview(ir: PreparedCaseAssemblyIR, onProgress: (value: CadProgress) => void, signal?: AbortSignal): Promise<BodyPreview & CasePreviewResult> {
    if (this.closed) return Promise.reject(new Error('CAD client is closed'));
    if (signal?.aborted) return Promise.reject(new Error('Preview superseded'));
    const id = crypto.randomUUID();
    const abort = () => this.worker.postMessage({ id, kind: 'cancel-preview' });
    this.progress.set(id, progress => { if (!signal?.aborted && progress.revision === ir.revision) onProgress(progress); });
    return new Promise((resolve, reject) => {
      this.pending.set(id, reply => {
        signal?.removeEventListener('abort', abort);
        // A complete reply may race cancellation. Consume its delta so both
        // sides keep the same cache; the generation owner rejects stale results.
        if (reply.kind === 'error') reject(new Error(reply.message));
        else if (reply.kind === 'preview' && reply.result.revision === ir.revision && !('bodyIds' in reply.result)) {
          if (reply.result.bodies) this.previewBodies = new Map(reply.result.bodies.map(body => [body.id, body]));
          resolve(combinedPreview(reply.result));
        }
        else if (reply.kind === 'preview' && reply.result.revision === ir.revision) {
          const delta = reply.result as PreviewDelta;
          const changed = new Map(delta.bodies.map(body => [body.id, body]));
          const bodies = delta.bodyIds.map(id => changed.get(id) ?? this.previewBodies.get(id)).filter((body): body is NonNullable<BodyPreview['bodies']>[number] => Boolean(body));
          if (bodies.length !== delta.bodyIds.length) {
            this.previewBodies.clear();
            reject(new Error('CAD preview delta is missing a body'));
            return;
          }
          this.previewBodies = new Map(bodies.map(body => [body.id, body]));
          resolve(combinedPreview({ revision: delta.revision, bodies }));
        }
        else reject(new Error('Unexpected or stale CAD preview response'));
      });
      signal?.addEventListener('abort', abort, { once: true });
      this.worker.postMessage({ id, kind: 'preview', ir, ...(cadProfilingEnabled() ? { metricsSentAt: cadTimestamp() } : {}) });
    });
  }

  cancel(): void {
    if (this.closed) return;
    this.worker.terminate();
    this.previewBodies.clear();
    for (const [id, settle] of this.pending) settle({ id, kind: 'error', message: 'CAD generation cancelled; retry any pending model import', revision: 0 });
    this.pending.clear();
    this.progress.clear();
    this.worker = this.start();
  }

  requestModel(bytes: Uint8Array): Promise<StepModel> {
    if (this.closed) return Promise.reject(new Error('CAD client is closed'));
    const id = crypto.randomUUID();
    const owned = new Uint8Array(bytes);

    return new Promise((resolve, reject) => {
      this.pending.set(id, (reply) => {
        if (reply.kind === 'error') {
          reject(new Error(reply.message));
          return;
        }

        if (reply.kind !== 'model') {
          reject(new Error('Unexpected CAD worker response'));
          return;
        }

        resolve(reply.result);
      });

      this.worker.postMessage({ id, kind: 'model', bytes: owned, ...(cadProfilingEnabled() ? { metricsSentAt: cadTimestamp() } : {}) }, [owned.buffer]);
    });
  }

  close(): void {
    this.closed = true;
    this.previewBodies.clear();
    this.worker.terminate();
    for (const [id, settle] of this.pending) settle({ id, kind: 'error', message: 'CAD client closed', revision: 0 });
    this.pending.clear();
    this.progress.clear();
  }
}
