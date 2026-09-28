import { expect, test, vi } from 'vitest';
import { LatestPreviewScheduler } from './livePreviewScheduler';

const flush = () => new Promise<void>(resolve => setTimeout(resolve, 0));

test('keeps one active run and replaces burst edits', async () => {
  const gates: Array<() => void> = [];
  const seen: number[] = [];
  const scheduler = new LatestPreviewScheduler<number>(async value => {
    seen.push(value);
    await new Promise<void>(resolve => gates.push(resolve));
  });
  scheduler.enqueue(1); await flush();
  scheduler.enqueue(2); scheduler.enqueue(3); scheduler.enqueue(4);
  gates.shift()!(); await flush();
  expect(seen).toEqual([1, 4]);
  gates.shift()!(); await flush();
  expect(scheduler.running).toBe(false);
});

test('a failed run does not strand later edits', async () => {
  const seen: number[] = [];
  const run = vi.fn(async (value: number) => { seen.push(value); if (value === 1) throw new Error('failed'); });
  const scheduler = new LatestPreviewScheduler(run);
  scheduler.enqueue(1); await flush(); scheduler.enqueue(2); await flush(); await flush();
  expect(seen).toEqual([1, 2]);
});

test('cancel drops pending work but leaves active work alone', async () => {
  let release!: () => void;
  const seen: number[] = [];
  const scheduler = new LatestPreviewScheduler<number>(async value => {
    seen.push(value);
    await new Promise<void>(resolve => { release = resolve; });
  });
  scheduler.enqueue(1); await flush(); scheduler.enqueue(2); scheduler.cancel(); release(); await flush();
  expect(seen).toEqual([1]);
  expect(scheduler.waiting).toBe(false);
});


test('settles superseded and cancelled callers while preserving the active request', async () => {
  let release!: () => void;
  const scheduler = new LatestPreviewScheduler<number>(async () => new Promise<void>(resolve => { release = resolve; }));
  const active = scheduler.enqueue(1);
  const replaced = scheduler.enqueue(2);
  const pending = scheduler.enqueue(3);
  await expect(replaced).resolves.toBe('superseded');
  scheduler.dispose();
  await expect(pending).resolves.toBe('cancelled');
  await expect(scheduler.enqueue(4)).resolves.toBe('cancelled');
  release(); await expect(active).resolves.toBe('completed');
});
