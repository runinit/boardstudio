import { useLayoutEffect } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { emptyProject } from '@boardstudio/v2-contracts';
import type { EditCommand, Matrix, Part, PartDefinition, ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';
import { matrixWithPreset } from './ui/matrixPresets';
import { CoreClient } from './CoreClient';
import { Workbench } from './ui/Workbench';

const WARMUP_SAMPLES = 10;
const MEASURED_SAMPLES = 100;
const PITCH_MM = 19.05;

type Size = 30 | 100 | 200 | 500;
type Scope = 'single' | 'row';
type StageCapture = 'off' | 'on';
type Result = { p50: number; p95: number; samples: number };
type Measurements = {
  worker: Result;
  painted: Result;
  parts: number;
  creation?: { workerMs: number; paintedMs: number };
  stages?: { wasm: Result; parse: Result; transportQueue: Result; react: Result; frame: Result };
};

const definitions: PartDefinition[] = [
  {
    id: 'bench-switch', name: 'MX switch', kind: 'switch',
    courtyard: [{ x: -7, y: -7 }, { x: 7, y: -7 }, { x: 7, y: 7 }, { x: -7, y: 7 }],
    pads: [
      { id: '1', number: '1', at: { x: -3, y: 0 }, size: { x: 2, y: 2 }, shape: 'circle' },
      { id: '2', number: '2', at: { x: 3, y: 0 }, size: { x: 2, y: 2 }, shape: 'circle' },
    ],
  },
  {
    id: 'bench-diode', name: 'SMD diode', kind: 'passive',
    courtyard: [{ x: -2, y: -1 }, { x: 2, y: -1 }, { x: 2, y: 1 }, { x: -2, y: 1 }],
    pads: [
      { id: '1', number: '1', at: { x: -1.5, y: 0 }, size: { x: 1, y: 1 }, shape: 'rect' },
      { id: '2', number: '2', at: { x: 1.5, y: 0 }, size: { x: 1, y: 1 }, shape: 'rect' },
    ],
  },
  {
    id: 'bench-led', name: 'RGB LED', kind: 'passive',
    courtyard: [{ x: -2, y: -2 }, { x: 2, y: -2 }, { x: 2, y: 2 }, { x: -2, y: 2 }],
    pads: [
      { id: '1', number: '1', at: { x: -1, y: -1 }, size: { x: 0.8, y: 0.8 }, shape: 'rect' },
      { id: '2', number: '2', at: { x: 1, y: -1 }, size: { x: 0.8, y: 0.8 }, shape: 'rect' },
      { id: '3', number: '3', at: { x: -1, y: 1 }, size: { x: 0.8, y: 0.8 }, shape: 'rect' },
      { id: '4', number: '4', at: { x: 1, y: 1 }, size: { x: 0.8, y: 0.8 }, shape: 'rect' },
    ],
  },
];

function fixture(keys: Size): ProjectDoc {
  const doc = emptyProject(`workbench-bench-${keys}`, `${keys} key assembly benchmark`);
  const columns = keys === 30 ? 5 : keys === 100 ? 10 : 20;
  const rows = keys / columns;
  const parts: Part[] = [];
  const pins = new Map<string, { partId: string; padId: string }[]>();
  const switches: string[] = [];

  const connect = (name: string, partId: string, padId: string) => {
    const list = pins.get(name) ?? [];
    list.push({ partId, padId });
    pins.set(name, list);
  };

  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < columns; col += 1) {
      const index = row * columns + col;
      const x = col * PITCH_MM;
      const y = -row * PITCH_MM;
      const keyId = `key-${row}-${col}`;
      const diodeId = `diode-${row}-${col}`;
      const ledId = `led-${row}-${col}`;

      switches.push(keyId);
      parts.push({ id: keyId, definitionId: 'bench-switch', reference: `SW${index + 1}`, pose: { at: { x, y }, rotation: 0 }, side: 'front' });
      parts.push({ id: diodeId, definitionId: 'bench-diode', reference: `D${index + 1}`, pose: { at: { x: x + 7, y: y + 5 }, rotation: 0 }, side: 'front' });
      parts.push({ id: ledId, definitionId: 'bench-led', reference: `LED${index + 1}`, pose: { at: { x: x - 5, y: y + 5 }, rotation: 0 }, side: 'front' });

      connect(`ROW${row}`, keyId, '1');
      connect(`KEY${index}`, keyId, '2');
      connect(`KEY${index}`, diodeId, '1');
      connect(`COL${col}`, diodeId, '2');
      connect('VDD', ledId, '1');
      connect('GND', ledId, '2');
      connect(`LED${index}`, ledId, '3');
      connect(`LED${index + 1}`, ledId, '4');
    }
  }

  doc.definitions = definitions;
  doc.parts = parts;
  doc.nets = [...pins].map(([name, netPins]) => ({ id: name, name, pins: netPins }));
  doc.outline = [{ id: 'bench-outline', kind: 'part-envelope', partIds: switches, margin: 4, operation: 'add' }];
  doc.boards = [{ id: 'bench-board', name: 'Benchmark board', outlineIds: ['bench-outline'], partIds: parts.map((part) => part.id), netIds: doc.nets.map((net) => net.id), thickness: 1.6 }];

  return doc;
}

function percentiles(samples: number[]): Result {
  const sorted = [...samples].sort((a, b) => a - b);
  return {
    p50: sorted[Math.ceil(sorted.length * 0.5) - 1],
    p95: sorted[Math.ceil(sorted.length * 0.95) - 1],
    samples: sorted.length,
  };
}

function paintedFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
}

function firstPaintedFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));
}

function View({ document, scene, committed, onEdit }: { document: ProjectDoc; scene: SceneDelta; committed: () => void; onEdit: (command: EditCommand) => void }) {
  useLayoutEffect(committed, [scene, committed]);

  return <Workbench
    document={document}
    scene={scene}
    selectedBoardId="bench-board"
    onEdit={onEdit}
    onUndo={() => {}}
    onRedo={() => {}}
    onExport={() => {}}
    compileFootprints={async () => []}
  />;
}

async function render(root: Root, document: ProjectDoc, scene: SceneDelta, onEdit: (command: EditCommand) => void = () => {}, wait: 'settled' | 'first-paint' = 'settled'): Promise<{ layoutAt: number; paintedAt: number }> {
  let committed = () => {};
  const mounted = new Promise<void>((resolve) => { committed = resolve; });
  root.render(<View document={document} scene={scene} committed={committed} onEdit={onEdit} />);
  await mounted;
  const layoutAt = performance.now();
  await (wait === 'first-paint' ? firstPaintedFrame() : paintedFrame());
  return { layoutAt, paintedAt: performance.now() };
}

async function runWorkbenchBenchmark(keys: Size, scope: Scope, captureStages: StageCapture = 'off'): Promise<Measurements> {
  const core = new CoreClient();
  const root = createRoot(document.getElementById('root')!);
  const project = fixture(keys);
  const first = await core.request({ id: crypto.randomUUID(), kind: 'open', document: project });

  if (first.kind !== 'scene') {
    core.close();
    root.unmount();
    throw new Error(first.kind === 'error' ? first.message : 'Expected project snapshot');
  }

  const targets = scope === 'row'
    ? project.parts.filter((part) => /^(key|diode|led)-0-/.test(part.id)).map((part) => part.id)
    : [`key-0-0`];
  const origins = new Map(project.parts.map((part) => [part.id, part.pose.at]));
  const worker: number[] = [];
  const painted: number[] = [];
  const stages = { wasm: [] as number[], parse: [] as number[], transportQueue: [] as number[], react: [] as number[], frame: [] as number[] };

  try {
    await render(root, first.document, first.scene);

    for (let index = 0; index < WARMUP_SAMPLES + MEASURED_SAMPLES; index += 1) {
      const start = performance.now();
      const reply = await core.request({
        id: crypto.randomUUID(), kind: 'edit', diagnostics: captureStages === 'on' ? true : undefined, command: {
          baseRevision: 0, transactionId: `workbench-bench-${index}`, phase: 'preview', targetIds: targets,
          operation: { kind: 'move-parts', positions: targets.map((id) => ({ id, at: { x: origins.get(id)!.x + index * 0.01, y: origins.get(id)!.y } })) },
        },
      });

      if (reply.kind === 'error') {
        throw new Error(reply.message);
      }
      if (reply.kind === 'matrix-projections') {
        throw new Error('Unexpected matrix projection reply during benchmark');
      }
      if (reply.kind !== 'scene' && reply.kind !== 'preview') {
        throw new Error('Unexpected case preparation reply during edit benchmark');
      }

      const workerDone = performance.now();
      const rendered = await render(root, reply.kind === 'scene' ? reply.document : first.document, reply.scene);

      if (index >= WARMUP_SAMPLES) {
        worker.push(workerDone - start);
        painted.push(rendered.paintedAt - start);
        if (captureStages === 'on' && reply.timing) {
          stages.wasm.push(reply.timing.wasmMs);
          stages.parse.push(reply.timing.parseMs);
          stages.transportQueue.push(Math.max(0, workerDone - start - reply.timing.wasmMs - reply.timing.parseMs));
          stages.react.push(Math.max(0, rendered.layoutAt - workerDone));
          stages.frame.push(Math.max(0, rendered.paintedAt - rendered.layoutAt));
        }
      }
    }
  } finally {
    root.unmount();
    core.close();
  }

  return {
    worker: percentiles(worker), painted: percentiles(painted), parts: project.parts.length,
    ...(captureStages === 'on' ? { stages: {
      wasm: percentiles(stages.wasm),
      parse: percentiles(stages.parse),
      transportQueue: percentiles(stages.transportQueue),
      react: percentiles(stages.react),
      frame: percentiles(stages.frame),
    } } : {}),
  };
}

function matrixFixture(keys: Size): { project: ProjectDoc; matrix: Matrix } {
  const project = emptyProject(`matrix-bench-${keys}`, `${keys} key matrix benchmark`);
  const columns = keys === 30 ? 5 : keys === 100 ? 10 : 20;
  const rows = keys / columns;
  const matrix: Matrix = {
    id: 'benchmark', boardId: 'bench-board', rows, columns,
    pitch: { x: PITCH_MM, y: PITCH_MM }, edgeGap: { x: 1, y: 1 },
    origin: { x: 0, y: 0 }, definitionId: 'ergogen:ceoloide/switch_mx', partIds: [],
    cells: Array.from({ length: keys }, (_, index) => ({
      row: Math.floor(index / columns), column: index % columns, enabled: true,
    })),
  };

  const prepared = matrixWithPreset(matrix, 'mx-rgb');
  project.definitions = prepared.definitions;
  project.outline = [{ id: 'bench-outline', kind: 'part-envelope', partIds: [], margin: 4, operation: 'add' }];
  project.boards = [{ id: 'bench-board', name: 'Benchmark board', outlineIds: ['bench-outline'], partIds: [], netIds: [], thickness: 1.6 }];
  return { project, matrix: prepared.matrix };
}

async function runMatrixBenchmark(keys: Size, scope: 'matrix' | 'row' | 'column'): Promise<Measurements> {
  const core = new CoreClient();
  const root = createRoot(document.getElementById('root')!);
  const { project, matrix } = matrixFixture(keys);
  const opened = await core.request({ id: crypto.randomUUID(), kind: 'open', document: project });

  if (opened.kind !== 'scene') {
    root.unmount();
    core.close();
    throw new Error(opened.kind === 'error' ? opened.message : 'Expected project snapshot');
  }

  const creationStart = performance.now();
  const committed = await core.request({
    id: crypto.randomUUID(), kind: 'edit', command: {
      baseRevision: 0, transactionId: 'create-matrix', phase: 'commit', targetIds: [matrix.id],
      operation: { kind: 'set-matrix', matrix },
    },
  });

  if (committed.kind !== 'scene') {
    root.unmount();
    core.close();
    throw new Error(committed.kind === 'error' ? committed.message : 'Expected matrix snapshot');
  }
  const creationWorkerDone = performance.now();

  const worker: number[] = [];
  const painted: number[] = [];
  let creationPaintedAt = creationWorkerDone;

  try {
    await render(root, committed.document, committed.scene);
    creationPaintedAt = performance.now();

    for (let index = 0; index < WARMUP_SAMPLES + MEASURED_SAMPLES; index += 1) {
      const delta = index * 0.01;
      const moved: Matrix = scope === 'matrix'
        ? { ...matrix, origin: { x: delta, y: 0 } }
        : scope === 'row'
          ? { ...matrix, rowOffsets: [{ x: delta, y: 0 }] }
          : { ...matrix, columnOffsets: [{ x: 0, y: delta }] };
      const start = performance.now();
      const reply = await core.request({
        id: crypto.randomUUID(), kind: 'edit', command: {
          baseRevision: committed.document.revision, transactionId: `matrix-bench-${index}`,
          phase: 'preview', targetIds: [matrix.id], operation: { kind: 'set-matrix', matrix: moved },
        },
      });

      if (reply.kind === 'error') {
        throw new Error(reply.message);
      }
      if (reply.kind === 'matrix-projections') {
        throw new Error('Unexpected matrix projection reply during benchmark');
      }
      if (reply.kind !== 'scene' && reply.kind !== 'preview') {
        throw new Error('Unexpected case preparation reply during matrix benchmark');
      }

      const workerDone = performance.now();
      await render(root, committed.document, reply.scene);

      if (index >= WARMUP_SAMPLES) {
        worker.push(workerDone - start);
        painted.push(performance.now() - start);
      }
    }
  } finally {
    root.unmount();
    core.close();
  }

  return {
    worker: percentiles(worker), painted: percentiles(painted), parts: committed.document.parts.length,
    creation: { workerMs: creationWorkerDone - creationStart, paintedMs: creationPaintedAt - creationStart },
  };
}

type PointerSample = {
  start?: number;
  resolve: (elapsed: number) => void;
  reject: (error: Error) => void;
};

let pointerSample: PointerSample | undefined;
let pointerResult: Promise<number> | undefined;
let closePointer: (() => void) | undefined;
const longTasks: number[] = [];
let frameTimes: number[] = [];
let frameId = 0;

try {
  new PerformanceObserver((entries) => {
    longTasks.push(...entries.getEntries().map((entry) => entry.duration));
  }).observe({ type: 'longtask', buffered: true });
} catch {
  // Long-task entries are optional browser diagnostics.
}

document.addEventListener('pointermove', () => {
  if (pointerSample && pointerSample.start === undefined) {
    pointerSample.start = performance.now();
  }
}, true);

async function preparePointerBenchmark(keys: Size): Promise<{ parts: number }> {
  closePointer?.();

  const core = new CoreClient();
  const root = createRoot(document.getElementById('root')!);
  const project = fixture(keys);
  const first = await core.request({ id: crypto.randomUUID(), kind: 'open', document: project });

  if (first.kind !== 'scene') {
    root.unmount();
    core.close();
    throw new Error(first.kind === 'error' ? first.message : 'Expected project snapshot');
  }

  let queue = Promise.resolve();
  let current = first.document;
  const onEdit = (command: EditCommand) => {
    queue = queue.then(async () => {
      const reply = await core.request({ id: crypto.randomUUID(), kind: 'edit', command });

      if (reply.kind === 'error') {
        throw new Error(reply.message);
      }
      if (reply.kind === 'matrix-projections') {
        throw new Error('Unexpected matrix projection reply during benchmark');
      }
      if (reply.kind !== 'scene' && reply.kind !== 'preview') {
        throw new Error('Unexpected case preparation reply during pointer benchmark');
      }

      if (reply.kind === 'scene') {
        current = reply.document;
      }

      await render(root, current, reply.scene, onEdit, 'first-paint');

      if (command.phase === 'preview' && pointerSample?.start !== undefined) {
        pointerSample.resolve(performance.now() - pointerSample.start);
        pointerSample = undefined;
      }
    }).catch((cause) => {
      pointerSample?.reject(cause instanceof Error ? cause : new Error(String(cause)));
      pointerSample = undefined;
    });
  };

  await render(root, first.document, first.scene, onEdit);
  closePointer = () => {
    pointerSample?.reject(new Error('Pointer benchmark closed'));
    pointerSample = undefined;
    root.unmount();
    core.close();
    closePointer = undefined;
  };

  return { parts: project.parts.length };
}

function beginPointerSample(): void {
  pointerResult = new Promise<number>((resolve, reject) => {
    pointerSample = { resolve, reject };
  });
}

function waitPointerSample(): Promise<number> {
  if (!pointerResult) {
    throw new Error('No pointer sample was started');
  }

  return pointerResult;
}

function closePointerBenchmark(): void {
  closePointer?.();
}

function benchmarkDiagnostics() {
  const memory = performance as Performance & { memory?: { usedJSHeapSize: number } };
  const resources = performance.getEntriesByType('resource') as PerformanceResourceTiming[];

  return {
    heapBytes: memory.memory?.usedJSHeapSize ?? null,
    transferredBytes: resources.reduce((sum, entry) => sum + entry.transferSize, 0),
    longestTaskMs: Math.max(0, ...longTasks),
    longTasksOver100Ms: longTasks.filter((duration) => duration > 100).length,
  };
}

function startFrameTrace(): void {
  cancelAnimationFrame(frameId);
  frameTimes = [];

  const sample = (time: number) => {
    frameTimes.push(time);
    frameId = requestAnimationFrame(sample);
  };

  frameId = requestAnimationFrame(sample);
}

function stopFrameTrace(): { p95GapMs: number; gapsOver50Ms: number } {
  cancelAnimationFrame(frameId);
  const gaps = frameTimes.slice(1).map((time, index) => time - frameTimes[index]);
  return { p95GapMs: gaps.length ? percentiles(gaps).p95 : 0, gapsOver50Ms: gaps.filter((gap) => gap > 50).length };
}

declare global {
  interface Window {
    runWorkbenchBenchmark: typeof runWorkbenchBenchmark;
    runMatrixBenchmark: typeof runMatrixBenchmark;
    preparePointerBenchmark: typeof preparePointerBenchmark;
    beginPointerSample: typeof beginPointerSample;
    waitPointerSample: typeof waitPointerSample;
    closePointerBenchmark: typeof closePointerBenchmark;
    benchmarkDiagnostics: typeof benchmarkDiagnostics;
    startFrameTrace: typeof startFrameTrace;
    stopFrameTrace: typeof stopFrameTrace;
  }
}

window.runWorkbenchBenchmark = runWorkbenchBenchmark;
window.runMatrixBenchmark = runMatrixBenchmark;
window.preparePointerBenchmark = preparePointerBenchmark;
window.beginPointerSample = beginPointerSample;
window.waitPointerSample = waitPointerSample;
window.closePointerBenchmark = closePointerBenchmark;
window.benchmarkDiagnostics = benchmarkDiagnostics;
window.startFrameTrace = startFrameTrace;
window.stopFrameTrace = stopFrameTrace;
