import type { CaseAssemblyIR } from '@boardstudio/v2-contracts';
import { CaseClient } from './CaseClient';
import { CoreClient } from './CoreClient';
import { prepareCase } from './prepareCase';
import { createRendererCanvas } from './renderClient';
import { cadProfilingEnabled, trackCadPreview } from './cadPerformance';

type Input = { assembly?: CaseAssemblyIR; step?: number[] };
type Scenario = 'cold' | 'warm-uncached' | 'cache-hit' | 'region-edit' | 'preview-export' | 'direct-export' | 'cancel-retry' | 'undo-redo' | 'board-switch' | 'edit-burst' | 'export-edit' | 'editing-soak';
const core = new CoreClient();
const renderer = await createRendererCanvas(document.querySelector('canvas')!);
let cad = new CaseClient();
let sceneRevision = 0;
let fitted = false;
let input: Input;

async function configure(value: Input) {
  cad.close();
  cad = new CaseClient();
  input = value;
  fitted = false;
  performance.clearMeasures();
}

async function sample({ scenario, index }: { scenario: Scenario; index: number }) {
  performance.clearMeasures();
  const start = performance.now();
  if (scenario === 'cold') { cad.close(); cad = new CaseClient(); }
  let result;
  let cancellationMs: number | undefined;
  {
    if (input.step) {
      result = { revision: ++sceneRevision, ...(await cad.requestModel(new Uint8Array(input.step))) };
    } else {
    const raw = structuredClone(input.assembly!);
    raw.revision = index + 1;
    raw.bodies.forEach(body => {
      body.revision = raw.revision;
      if (['warm-uncached', 'direct-export', 'preview-export'].includes(scenario)) body.body.z = (body.body.z ?? 0) + index * 0.001;
      if (scenario === 'editing-soak') {
        // Return to the original (Undo), restore an edit, and alternate board
        // contexts while pressuring the same warm worker's geometry caches.
        body.body.z = (body.body.z ?? 0) + (index % 8 === 0 ? 0 : Math.ceil(index / 8) * 0.001);
        if (index % 8 === 3) body.body.boardId = 'alternate-board';
      }
    });
    if (['region-edit', 'export-edit'].includes(scenario)) {
      const body = raw.bodies[0].body;
      body.openings = [...(body.openings ?? []), {
        points: [{ x: 4 + index * 0.001, y: -1 }, { x: 6 + index * 0.001, y: -1 }, { x: 6 + index * 0.001, y: 5 }, { x: 4 + index * 0.001, y: 5 }],
        z: body.z ?? 0, height: body.thickness + (body.wallHeight ?? 0),
      }];
    }
    if (['undo-redo', 'board-switch', 'edit-burst', 'export-edit'].includes(scenario)) {
      const before = await prepareCase(core, input.assembly!);
      await cad.preview(before, () => {});
    }
    if (['undo-redo', 'board-switch', 'edit-burst'].includes(scenario)) {
      const jobs = [];
      for (let edit = 1; edit <= 3; edit++) {
        const changed = structuredClone(raw);
        changed.revision += edit;
        for (const body of changed.bodies) {
          body.revision = changed.revision;
          body.body.z = (body.body.z ?? 0) + edit * 0.01;
          if (scenario === 'board-switch') body.body.boardId = `board-${edit % 2}`;
        }
        const ir = await prepareCase(core, changed);
        const job = cad.preview(ir, () => {});
        jobs.push(job);
        if (scenario !== 'edit-burst') await job;
      }
      await Promise.all(jobs);
    }
    const prepared = await prepareCase(core, raw);
    if (scenario === 'editing-soak' && index % 8 === 6) {
      const obsolete = structuredClone(prepared);
      obsolete.bodies.forEach(body => { body.body.z = (body.body.z ?? 0) + 0.007; });
      const controller = new AbortController();
      let superseded = false;
      try {
        await cad.preview(obsolete, progress => {
          if (progress.stage === 'building') { superseded = true; controller.abort(); }
        }, controller.signal);
      } catch (error) {
        if (!(error instanceof Error) || !error.message.includes('superseded')) throw error;
      }
      if (!superseded) throw new Error('Soak did not exercise cooperative cancellation');
    }
    if (scenario === 'cancel-retry') {
      let cancelled = false;
      try {
        await cad.preview(prepared, progress => {
          if (!cancelled && progress.stage === 'building') { cancelled = true; cad.cancel(); }
        });
        throw new Error('Cancellation failed to interrupt the request');
      } catch (error) {
        if (!(error instanceof Error) || !error.message.includes('CAD generation cancelled')) throw error;
      }
      cancellationMs = performance.now() - start;
    }
    result = ['direct-export', 'export-edit'].includes(scenario) || scenario === 'editing-soak' && index % 8 === 4
      ? await cad.request(prepared) : await cad.preview(prepared, () => {});
    if (scenario === 'preview-export') result = await cad.request(prepared);
    }
  }
  const completedMs = performance.now() - start;
  const bodies = 'bodies' in result && result.bodies?.length ? result.bodies : [{ id: 'import', name: 'Import', ...result.mesh }];
  if ('bodies' in result && result.bodies) trackCadPreview({ revision: result.revision, bodies: result.bodies }, start, () => true);
  else trackCadPreview({ revision: result.revision, bodies: [{ id: 'import', name: 'Import', ...result.mesh }] }, start, () => true);
  const revision = ++sceneRevision;
  if (!await renderer.setScene({ revision, kind: 'assembly', theme: 'dark', keepCamera: fitted,
    board: { contours: [], thickness: 1.6 }, bodies: bodies.map(body => ({ id: body.id, name: body.name, mesh: body })) })) {
    throw new Error('Renderer rejected benchmark scene');
  }
  fitted = true;
  if (cadProfilingEnabled()) await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(() => { cancelAnimationFrame(frame); reject(new Error('No paint opportunity within 10 seconds')); }, 10_000);
    let frame = 0;
    const check = () => {
      if (performance.getEntriesByName('boardstudio.cad.preview.paint-opportunity').length) { clearTimeout(timeout); resolve(); }
      else frame = requestAnimationFrame(check);
    };
    frame = requestAnimationFrame(check);
  });
  // Match profiled frame pacing without claiming an uninstrumented paint timestamp.
  if (!cadProfilingEnabled()) await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  const entries = performance.getEntriesByType('measure').map(entry => ({ name: entry.name, durationMs: entry.duration, detail: (entry as PerformanceMeasure).detail }));
  const paint = entries.find(entry => entry.name === 'boardstudio.cad.preview.paint-opportunity')!;
  return { scenario, index, completedMs, paintedMs: paint?.durationMs ?? null, cancellationMs, entries };
}

declare global { interface Window { cadBenchmark: { configure: typeof configure; sample: typeof sample } } }
window.cadBenchmark = { configure, sample };
