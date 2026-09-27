import type { Contour, EditCommand, ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';
import { CoreClient } from './CoreClient';
import { demoProject } from './demo';

const WARMUP_SAMPLES = 10;
const MEASURED_SAMPLES = 100;
const PITCH_MM = 19.05;

function fixture(keys: number): ProjectDoc {
  const document = demoProject();
  const definitionId = document.definitions[0].id;
  const columns = keys === 100 ? 10 : 20;
  const rows = keys / columns;

  document.id = `benchmark-${keys}`;
  document.name = `${keys} key benchmark`;
  document.parts = [];

  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < columns; col += 1) {
      const id = `key-${row}-${col}`;

      document.parts.push({
        id,
        definitionId,
        reference: `SW${document.parts.length + 1}`,
        pose: { at: { x: col * PITCH_MM, y: -row * PITCH_MM }, rotation: 0 },
        side: 'front',
      });
    }
  }

  document.matrices = [];
  document.nets = [];
  document.outline = document.parts.map((part) => ({
    id: `envelope-${part.id}`,
    kind: 'part-envelope' as const,
    partIds: [part.id],
    margin: 2,
    operation: 'add' as const,
  }));
  document.boards[0].partIds = document.parts.map((part) => part.id);
  document.boards[0].outlineIds = document.outline.map((feature) => feature.id);
  document.boards[0].netIds = [];

  return document;
}

function paint(contours: Contour[]): void {
  const canvas = document.getElementById('outline') as HTMLCanvasElement;
  const context = canvas.getContext('2d')!;

  context.clearRect(0, 0, canvas.width, canvas.height);
  context.beginPath();

  for (const contour of contours) {
    const [first, ...rest] = contour.points;

    context.moveTo(first.x * 2 + 50, -first.y * 2 + 50);

    for (const point of rest) {
      context.lineTo(point.x * 2 + 50, -point.y * 2 + 50);
    }

    context.closePath();
  }

  context.stroke();
}

function paintedFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
}

async function runOutlineBenchmark(keys: 100 | 200): Promise<{ p50: number; p95: number; samples: number }> {
  const core = new CoreClient();
  const document = fixture(keys);
  const first = await core.request({ id: crypto.randomUUID(), kind: 'open', document });

  if (first.kind === 'error') {
    throw new Error(first.message);
  }

  const last = document.parts.at(-1)!;
  const durations: number[] = [];

  try {
    for (let index = 0; index < WARMUP_SAMPLES + MEASURED_SAMPLES; index += 1) {
      const command: EditCommand = {
        baseRevision: 0,
        transactionId: `bench-${index}`,
        phase: 'preview',
        targetIds: [last.id],
        operation: { kind: 'move-parts', positions: [{ id: last.id, at: { x: last.pose.at.x + index * 0.01, y: last.pose.at.y } }] },
      };
      const start = performance.now();
      const reply = await core.request({ id: crypto.randomUUID(), kind: 'edit', command });

      if (reply.kind === 'error') {
        throw new Error(reply.message);
      }
      if (reply.kind === 'matrix-projections') {
        throw new Error('Unexpected matrix projection reply during benchmark');
      }
      if (reply.kind !== 'scene' && reply.kind !== 'preview') {
        throw new Error('Unexpected case preparation reply during outline benchmark');
      }

      paint((reply.scene as SceneDelta).contours);
      await paintedFrame();

      if (index >= WARMUP_SAMPLES) {
        durations.push(performance.now() - start);
      }
    }
  } finally {
    core.close();
  }

  durations.sort((a, b) => a - b);
  return {
    p50: durations[Math.ceil(durations.length * 0.5) - 1],
    p95: durations[Math.ceil(durations.length * 0.95) - 1],
    samples: durations.length,
  };
}

declare global {
  interface Window { runOutlineBenchmark: typeof runOutlineBenchmark }
}

window.runOutlineBenchmark = runOutlineBenchmark;
