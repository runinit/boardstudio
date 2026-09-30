import type { CoreReply, CoreRequest, Matrix, ProjectDoc } from '@boardstudio/v2-contracts';

export class CoreClient {
  private worker: Worker;
  private closed = false;

  private pending = new Map<string, { resolve: (reply: CoreReply) => void; persist: boolean }>();
  private snapshot?: ProjectDoc;

  constructor() {
    this.worker = this.start();
  }

  private start(): Worker {
    const worker = new Worker(new URL('./core.worker.ts', import.meta.url), { type: 'module' });

    worker.onmessage = (event: MessageEvent<CoreReply>) => {
      if (this.closed || worker !== this.worker) return;
      const pending = this.pending.get(event.data.id);

      if (!pending) {
        return;
      }

      this.pending.delete(event.data.id);

      if (pending.persist && event.data.kind === 'scene') {
        this.snapshot = event.data.document;
      }

      pending.resolve(event.data);
    };

    worker.onerror = (event) => {
      event.preventDefault();
      if (this.closed || worker !== this.worker) return;

      for (const [id, pending] of this.pending) {
        pending.resolve({ id, kind: 'error', message: 'Core worker failed and restarted', revision: this.snapshot?.revision ?? 0 });
      }

      this.pending.clear();
      worker.terminate();
      this.worker = this.start();

      if (this.snapshot) {
        this.worker.postMessage({ id: crypto.randomUUID(), kind: 'open', document: this.snapshot });
      }
    };

    return worker;
  }

  request(request: CoreRequest): Promise<CoreReply> {
    if (this.closed) return Promise.resolve({ id: request.id, kind: 'error', message: 'Core client is closed', revision: this.snapshot?.revision ?? 0 });
    return new Promise((resolve) => {
      const persist = request.kind !== 'prepare-case' && request.kind !== 'project-matrices' && request.kind !== 'resolve-mechanical' && request.kind !== 'mechanical-profile'
        && (request.kind !== 'edit' || request.command.phase === 'commit');

      this.pending.set(request.id, { resolve, persist });
      this.worker.postMessage(request);
    });
  }

  projectMatrices(id: string, baseRevision: number, matrices: Matrix[]): Promise<CoreReply> {
    return this.request({ kind: 'project-matrices', id, baseRevision, matrices });
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.worker.terminate();
    for (const [id, pending] of this.pending) {
      pending.resolve({ id, kind: 'error', message: 'Core client is closed', revision: this.snapshot?.revision ?? 0 });
    }
    this.pending.clear();
  }
}
