type PreviewRun<T> = (request: T) => Promise<void>;
type Outcome = 'completed' | 'superseded' | 'cancelled';
type Job<T> = { request: T; settle: (outcome: Outcome) => void };

/** One active kernel call and one replaceable request; every caller settles. */
export class LatestPreviewScheduler<T> {
  private active = false;
  private pending: Job<T> | undefined;
  private disposed = false;

  constructor(private readonly run: PreviewRun<T>) {}

  enqueue(request: T): Promise<Outcome> {
    if (this.disposed) return Promise.resolve('cancelled');
    this.pending?.settle('superseded');
    const result = new Promise<Outcome>(settle => { this.pending = { request, settle }; });
    void this.drain();
    return result;
  }

  private async drain(): Promise<void> {
    if (this.active || this.disposed) return;
    this.active = true;
    try {
      while (!this.disposed && this.pending) {
        const job = this.pending;
        this.pending = undefined;
        try { await this.run(job.request); } catch { /* The consumer reports failures; later edits may recover. */ }
        finally { job.settle('completed'); }
      }
    } finally { this.active = false; }
  }

  cancel(): void { this.pending?.settle('cancelled'); this.pending = undefined; }
  dispose(): void { this.disposed = true; this.cancel(); }
  get running(): boolean { return this.active; }
  get waiting(): boolean { return this.pending !== undefined; }
}
