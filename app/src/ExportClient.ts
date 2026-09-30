import type { ArtifactReply, ArtifactRequest, CompiledFootprint, FootprintCompileJob } from '@boardstudio/v2-contracts';
import type { ExportReply, ExportRequest, ExportWorkerReply } from './export.worker';
import type { ArchiveRequest, ArchiveReply } from './export.worker';

type WithoutId<T> = T extends { id: string } ? Omit<T, 'id'> : never;
type ArtifactOperation = WithoutId<ArtifactRequest>;

export class ExportClient {
  private worker: Worker;
  private pending = new Map<string, {
    resolve: (reply: ExportWorkerReply) => void;
    reject: (error: Error) => void;
  }>();
  private compiled = new Map<string, Promise<CompiledFootprint[]>>();
  private closed = false;

  constructor() {
    this.worker = this.start();
  }

  private start(): Worker {
    const worker = new Worker(new URL('./export.worker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (event: MessageEvent<ExportWorkerReply>) => {
      if (this.closed || worker !== this.worker) return;
      const reply = event.data;
      const pending = this.pending.get(reply.id);
      if (!pending) return;
      this.pending.delete(reply.id);
      if (reply.kind === 'error') {
        pending.reject(new Error('message' in reply ? reply.message : reply.error.message));
        return;
      }
      pending.resolve(reply);
    };
    worker.onerror = event => {
      event.preventDefault();
      if (this.closed || worker !== this.worker) return;
      for (const pending of this.pending.values()) pending.reject(new Error('Export worker failed and restarted'));
      this.pending.clear();
      worker.terminate();
      this.worker = this.start();
    };
    return worker;
  }

  preview(input: Omit<ExportRequest, 'id' | 'kind' | 'files'>): Promise<Extract<ArtifactReply, { kind: 'preview-board' }>['result']> {
    if (this.closed) return Promise.reject(new Error('Export client is closed'));
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve: (reply) => reply.kind === 'preview-board' ? resolve(reply.result) : reject(new Error('Expected PCB preview')), reject });
      this.worker.postMessage({ ...input, id, kind: 'pcb-preview', files: {} });
    });
  }

  request(input: Omit<ExportRequest, 'id'>): Promise<Extract<ExportReply, { kind: 'file' }>> {
    if (this.closed) return Promise.reject(new Error('Export client is closed'));
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (reply: ExportWorkerReply) => void, reject });
      const files = Object.fromEntries(Object.entries(input.files).map(([name, bytes]) => [name, new Uint8Array(bytes)]));
      const transfers = Object.values(files).map((bytes) => bytes.buffer);
      try {
        this.worker.postMessage({ ...input, files, id }, transfers);
      } catch (error) {
        this.pending.delete(id);
        reject(error);
      }
    });
  }

  archive(input: Omit<ArchiveRequest, 'id'>): Promise<Extract<ArchiveReply, { kind: 'archive' }>> {
    if (this.closed) return Promise.reject(new Error('Export client is closed'));
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (reply: ExportWorkerReply) => void, reject });
      const buffers = input.buffers.map((bytes) => new Uint8Array(bytes));
      try {
        this.worker.postMessage({ ...input, id, buffers }, buffers.map((bytes) => bytes.buffer));
      } catch (error) {
        this.pending.delete(id);
        reject(error);
      }
    });
  }

  artifact(input: ArtifactOperation): Promise<ArtifactReply> {
    if (this.closed) return Promise.reject(new Error('Export client is closed'));
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (reply: ExportWorkerReply) => void, reject });
      this.worker.postMessage({ id, kind: 'artifact', request: { ...input, id } });
    });
  }

  compile(jobs: FootprintCompileJob[]): Promise<CompiledFootprint[]> {
    const key = JSON.stringify(jobs);
    const cached = this.compiled.get(key);
    if (cached) return cached;
    const result = this.artifact({ kind: 'compile-footprints', jobs }).then((reply) => {
      if (reply.kind !== 'compile-footprints') throw new Error('Expected Rust footprint compilation');
      return reply.result;
    }).catch((error: unknown) => {
      this.compiled.delete(key);
      throw error;
    });
    this.compiled.set(key, result);
    if (this.compiled.size > 64) this.compiled.delete(this.compiled.keys().next().value!);
    return result;
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.worker.terminate();
    for (const pending of this.pending.values()) pending.reject(new Error('Export worker was closed'));
    this.pending.clear();
    this.compiled.clear();
  }
}
