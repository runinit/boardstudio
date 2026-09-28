import { auditSceneBounds, projectScenePoint, sceneProjection } from './scene-bounds';
import { expect, test, type Page } from '@playwright/test';
import { splitFixture } from './splitMechanicalFixture';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { cpus } from 'node:os';

type Sample = { actionToWorkerReplyMs: number | null; releaseToPaintMs: number; previewStartToPaintMs: number; attribution: unknown };
const distribution = (values: number[]) => {
  const sorted = [...values].sort((a, b) => a - b);
  return { samples: values, p50: sorted[Math.floor(sorted.length / 2)], p95: sorted[Math.max(0, Math.ceil(sorted.length * 0.95) - 1)] };
};

async function diagnostics(page: Page, since = 0) {
  return page.evaluate(since => Object.fromEntries(['request', 'job', 'persistence', 'preview.paint-opportunity'].map(name => [name,
    performance.getEntriesByName(`boardstudio.cad.${name}`).filter(entry => entry.startTime >= since)
      .map(entry => ({ startAt: entry.startTime, durationMs: entry.duration, detail: (entry as PerformanceMeasure).detail })),
  ])), since);
}

async function setup(page: Page, mode: 'numeric' | 'gasket' | 'mount' = 'numeric') {
  await auditSceneBounds(page);
  await page.addInitScript(capturePrepared => {
    if ((window as any).__livePreviewAudit) return;
    const audit = (window as any).__livePreviewAudit = { jobs: [], assemblies: [], bounds: [], events: [], core: [], longTasks: [], prepared: [] };
    new PerformanceObserver(list => audit.longTasks.push(...list.getEntries().map(entry => ({ startAt: entry.startTime, durationMs: entry.duration })))).observe({ type: 'longtask', buffered: true });
    const targetName = (target: EventTarget | null) => {
      const element = target instanceof Element ? target : null;
      if (element instanceof HTMLInputElement && element.labels?.length) {
        return [...element.labels].map(label => label.innerText.replace(/\s+/g, ' ').trim()).join(' ');
      }
      const control = element?.closest('button') ?? element;
      return control?.getAttribute('aria-label') ?? control?.getAttribute('title') ?? '';
    };
    document.addEventListener('pointerup', event => audit.events.push({ type: 'pointerup', at: performance.now(), target: targetName(event.target) }), true);
    document.addEventListener('click', event => audit.events.push({ type: 'click', at: performance.now(), target: targetName(event.target) }), true);
    document.addEventListener('blur', event => audit.events.push({ type: 'blur', at: performance.now(), target: targetName(event.target) }), true);
    const original = Worker.prototype.postMessage;
    const observed = new WeakSet<Worker>();
    Worker.prototype.postMessage = function (message: any, ...rest: any[]) {
      if (capturePrepared && message?.kind === 'preview' && audit.prepared.length < 32) audit.prepared.push(message.ir);
      if (message?.kind && ['edit', 'undo', 'resolve-mechanical', 'prepare-case'].includes(message.kind)) {
        message = { ...message, diagnostics: true };
        audit.core.push({ kind: message.kind, sentAt: performance.now(), id: message.id });
      }
      if (!observed.has(this)) {
        observed.add(this);
        this.addEventListener('message', event => {
          if (event.data.kind === 'preview') audit.jobs.push({ revision: event.data.result.revision, at: performance.now() });
          if (event.data.kind === 'mechanical-resolved') audit.assemblies.push(event.data.assembly);
          if (event.data.timing) {
            const request = audit.core.find((entry: any) => entry.id === event.data.id);
            if (request) Object.assign(request, { repliedAt: performance.now(), timing: event.data.timing });
          }
          if (!event.data.patch && event.data.prepared?.bounds) audit.bounds.push(event.data.prepared.bounds);
        });
      }
      return original.call(this, message, ...rest);
    };
  }, Boolean(process.env.BOARDSTUDIO_LIVE_PREPARED));
  await page.goto('/?cadMetrics=1');
  await expect(page.locator('.wb-root')).toBeVisible();
  const fixture = splitFixture();
  fixture.id = `live-preview-${mode}`;
  if (mode === 'gasket') fixture.mechanical = { ...fixture.mechanical!, mount: 'gasket', integratedPlateFrame: false, gasketTravel: 0.3, bottomStyle: 'shell' };
  if (mode === 'mount') {
    fixture.mechanical = undefined;
    fixture.caseBodies = [{ id: 'authored-tray', name: 'Authored tray', boardId: 'main-board', kind: 'tray',
      thickness: 3, clearance: 0.5, z: -10, wallHeight: 14, wallThickness: 2,
      mounts: [{ id: 'mount', kind: 'hole', at: { x: 170, y: 5 }, holeDiameter: 2.5 }] }];
  }
  await page.evaluate(async document => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('boardstudio-v2', 1);
      request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
    });
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction('projects', 'readwrite');
      transaction.objectStore('projects').put(document);
      transaction.oncomplete = () => resolve(); transaction.onerror = () => reject(transaction.error);
    });
    db.close(); localStorage.setItem('boardstudio-v2-active-project', document.id);
  }, fixture);
  await page.reload();
  await page.getByRole('button', { name: /^Collapse left half$/i }).click();
  await page.getByRole('button', { name: /^Collapse right half$/i }).click();
  await page.getByRole('tab', { name: 'Case', exact: true }).click();
  await expect(page.locator('.wb-case-generation')).toHaveAttribute('data-state', 'ready', { timeout: 30_000 });
  await expect(page.getByRole('switch', { name: 'Live preview', exact: true })).toBeChecked();
  await expect(page.getByText('Preparing 3D geometry…', { exact: true })).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => performance.getEntriesByName('boardstudio.cad.preview.paint-opportunity').length)).toBeGreaterThan(0);
  return diagnostics(page);
}

async function completed(page: Page, started: number, previousRevision: number): Promise<Sample> {
  await expect.poll(async () => Number(await page.locator('.wb-root').getAttribute('data-revision'))).toBeGreaterThan(previousRevision);
  const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
  await expect(page.locator('.wb-case-generation')).toHaveAttribute('data-state', 'ready', { timeout: 30_000 });
  await expect.poll(() => page.evaluate(revision => performance.getEntriesByName('boardstudio.cad.preview.paint-opportunity')
    .some(entry => (entry as PerformanceMeasure).detail?.revision === revision), revision), { timeout: 30_000 }).toBe(true);
  return page.evaluate(({ revision, started }) => {
    const paint = performance.getEntriesByName('boardstudio.cad.preview.paint-opportunity').find(entry => (entry as PerformanceMeasure).detail?.revision === revision) as PerformanceMeasure;
    const paintedAt = paint.startTime + paint.duration;
    const entries = (name: string) => performance.getEntriesByName(name).filter(entry => entry.startTime >= started && entry.startTime <= paintedAt) as PerformanceMeasure[];
    const audit = (window as any).__livePreviewAudit;
    const cad = entries('boardstudio.cad.request').filter(entry => entry.detail?.revision === revision);
    const core = entries('boardstudio.cad.core-preparation').filter(entry => entry.detail?.revision === revision);
    const sum = (name: string) => entries(name).reduce((total, entry) => total + entry.duration, 0);
    const job = (window as any).__livePreviewAudit.jobs.find((entry: any) => entry.revision === revision);
    return {
      actionToWorkerReplyMs: job ? job.at - started : null,
      releaseToPaintMs: paint.startTime + paint.duration - started,
      previewStartToPaintMs: paint.duration,
      attribution: {
        actionStartAt: started,
        actionEndAt: paint.startTime + paint.duration,
        paintMs: paint.duration,
        cadRequests: cad.map(entry => ({ startAt: entry.startTime, durationMs: entry.duration, detail: entry.detail })),
        corePreparationMs: core.reduce((total, entry) => total + entry.duration, 0),
        persistenceMs: sum('boardstudio.cad.persistence'),
        exactDraftReused: entries('boardstudio.cad.draft-reuse').length > 0,
        coreRequests: audit.core.filter((entry: any) => entry.sentAt >= started && entry.repliedAt <= paintedAt),
        longTasks: audit.longTasks.filter((entry: any) => entry.startAt >= started && entry.startAt <= paintedAt),
        renderer: { prepareMs: sum('boardstudio.renderer.prepare'), uploadMs: sum('boardstudio.renderer.upload'), bodyPatchMs: sum('boardstudio.renderer.body-patch') },
      },
    };
  }, { revision, started });
}

async function eventTime(page: Page, type: string, target: string | undefined, after: number): Promise<number> {
  return page.evaluate(({ type, target, after }) => {
    const event = [...(window as any).__livePreviewAudit.events].reverse().find((entry: any) => entry.at > after && entry.type === type && (!target || entry.target === target || entry.target.includes(target)));
    if (!event) throw new Error(`Missing ${type} event after ${after}${target ? ` for ${target}` : ''}`);
    return event.at;
  }, { type, target, after });
}

test('records numeric, undo, gasket and mount release, and retained pose performance', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const cpuRate = Number(process.env.BOARDSTUDIO_LIVE_CPU_RATE ?? 1);
  if (cpuRate > 1) {
    const emulation = await page.context().newCDPSession(page);
    await emulation.send('Emulation.setCPUThrottlingRate', { rate: cpuRate });
  }
  const initial: Record<string, unknown> = { numeric: await setup(page) };
  const prepared: Record<string, unknown> = {};
  const profiler = process.env.BOARDSTUDIO_LIVE_CPU_PROFILE ? await page.context().newCDPSession(page) : undefined;
  const timeline: unknown[] = [];
  if (profiler) {
    profiler.on('Tracing.dataCollected', event => timeline.push(...event.value));
    await profiler.send('Tracing.start', { categories: 'devtools.timeline,disabled-by-default-devtools.timeline,toplevel,v8,gpu,cc', transferMode: 'ReportEvents' });
    await profiler.send('Profiler.enable'); await profiler.send('Profiler.start');
  }
  const stopProfile = async () => {
    const { profile } = await profiler!.send('Profiler.stop');
    const path = process.env.BOARDSTUDIO_LIVE_CPU_PROFILE!;
    await writeFile(path, JSON.stringify(profile));
    const stopped = new Promise<void>(resolve => profiler!.once('Tracing.tracingComplete', () => resolve()));
    await profiler!.send('Tracing.end'); await stopped;
    await writeFile(`${path}.timeline.json`, JSON.stringify({ traceEvents: timeline }));
    const system = await browser.newBrowserCDPSession();
    const hardware = await system.send('SystemInfo.getInfo');
    await writeFile(`${path}.system.json`, JSON.stringify({ ...hardware, cpuRate, cpu: cpus()[0]?.model, viewport: page.viewportSize(), chromium: browser.version() }));
    await system.detach(); await profiler!.detach();
  };
  const numeric: Sample[] = [], undo: Sample[] = [], gaskets: Sample[] = [];
  const thickness = page.getByRole('spinbutton', { name: 'Wall thickness mm', exact: true });
  for (let index = 0; index < (profiler ? 1 : 5); index += 1) {
    const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
    await thickness.fill(String(2.2 + index * 0.1));
    const blurBefore = await page.evaluate(() => performance.now());
    await thickness.press('Tab');
    const started = await eventTime(page, 'blur', 'Wall thickness mm', blurBefore);
    numeric.push(await completed(page, started, revision));
    const edited = Number(await page.locator('.wb-root').getAttribute('data-revision'));
    const undoBefore = await page.evaluate(() => performance.now());
    await page.getByRole('button', { name: 'Undo', exact: true }).click();
    const undoStarted = await eventTime(page, 'click', 'Undo', undoBefore);
    undo.push(await completed(page, undoStarted, edited));
    await expect(thickness).toHaveValue('2');
  }
  if (profiler && process.env.BOARDSTUDIO_LIVE_PROFILE_SCENARIO !== 'gaskets') {
    await stopProfile();
    return;
  }
  const rapidStarted = await page.evaluate(() => performance.now());
  const rapidRevision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
  for (let index = 0; index < 6; index++) {
    await thickness.fill(String(2.7 + index * 0.05));
    await thickness.press('Tab');
  }
  const rapidSample = await completed(page, await eventTime(page, 'blur', 'Wall thickness mm', rapidStarted), rapidRevision + 5);
  const rapid = { finalEdit: rapidSample, diagnostics: await diagnostics(page, rapidStarted) };
  if (process.env.BOARDSTUDIO_LIVE_PREPARED) prepared.numeric = await page.evaluate(() => (window as any).__livePreviewAudit.prepared);
  initial.gasket = await setup(page, 'gasket');
  await page.getByRole('button', { name: 'Edit gaskets', exact: true }).click();
  await page.getByRole('button', { name: 'Fit', exact: true }).click();
  await page.getByRole('button', { name: 'Top', exact: true }).click();
  const canvas = page.locator('.wb-assembly-scene canvas');
  const box = (await canvas.boundingBox())!;
  const pointerStart = await page.evaluate(() => performance.now());
  for (let index = 0; index < 5; index += 1) {
    const { assembly, bounds } = await page.evaluate(() => ({ assembly: (window as any).__livePreviewAudit.assemblies.at(-1), bounds: (window as any).__fitBounds }));
    const support = assembly.gasketSupports.find((item: any) => item.regionId === 'left');
    const retainer = assembly.stack.find((layer: any) => layer.id === 'retainer');
    const vertical = 17 * Math.PI / 180, horizontal = Math.atan(Math.tan(vertical) * box.width / box.height);
    const distance = bounds[3] / Math.sin(Math.min(vertical, horizontal)) * 1.16;
    const screen = (x: number, y: number) => {
      const dx = x - bounds[0], dy = y - bounds[1], dz = retainer.z + retainer.thickness + 1 - bounds[2];
      const depth = distance - dx * Math.cos(1.56) - dz * Math.sin(1.56);
      const scale = box.height / (2 * depth * Math.tan(vertical));
      return { x: box.x + box.width / 2 + (dx * Math.sin(1.56) - dz * Math.cos(1.56)) * scale, y: box.y + box.height / 2 - dy * scale };
    };
    const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
    const from = screen(support.at.x, support.at.y);
    const delta = index % 2 === 0 ? 2 : -2;
    const to = screen(support.at.x + support.tangent.x * delta, support.at.y + support.tangent.y * delta);
    await page.mouse.move(from.x, from.y); await page.mouse.down(); await page.mouse.move(to.x, to.y, { steps: 12 });
    await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', String(revision));
    const pointerBefore = await page.evaluate(() => performance.now());
    await page.mouse.up();
    const released = await eventTime(page, 'pointerup', undefined, pointerBefore);
    gaskets.push(await completed(page, released, revision));
  }
  const renderer = await page.evaluate(start => {
    const entries = (name: string) => performance.getEntriesByName(name).filter(entry => entry.startTime >= start) as PerformanceMeasure[];
    return {
      pointerWork: entries('boardstudio.renderer.pointer-work').map(entry => entry.duration),
      pointerDraw: entries('boardstudio.renderer.pointer-draw').map(entry => entry.duration),
      handles: entries('boardstudio.renderer.handle-pose').map(entry => ({ ms: entry.duration, ...entry.detail })),
      patches: entries('boardstudio.renderer.body-patch').map(entry => entry.detail),
    };
  }, pointerStart);
  expect(renderer.pointerDraw.length).toBeGreaterThan(0);
  expect(renderer.handles.length).toBeGreaterThan(0);
  expect(renderer.handles.every(entry => entry.uploadedHandles === 0)).toBe(true);
  expect(renderer.patches.length).toBeGreaterThan(0);
  if (profiler) { await stopProfile(); return; }
  const mounts: Sample[] = [];
  if (process.env.BOARDSTUDIO_LIVE_PREPARED) prepared.gaskets = await page.evaluate(() => (window as any).__livePreviewAudit.prepared);
  initial.mount = await setup(page, 'mount');
  await page.getByRole('button', { name: 'Edit mounts', exact: true }).click();
  await page.getByRole('button', { name: 'Fit', exact: true }).click();
  await page.getByRole('button', { name: 'Top', exact: true }).click();
  const mountView = await sceneProjection(page);
  const screen = (x: number) => projectScenePoint(mountView, x, 5, -9.2);
  for (let index = 0; index < 5; index += 1) {
    const from = screen(index % 2 === 0 ? 170 : 180), to = screen(index % 2 === 0 ? 180 : 170);
    const revision = Number(await page.locator('.wb-root').getAttribute('data-revision'));
    await page.mouse.move(from.x, from.y); await page.mouse.down(); await page.mouse.move(to.x, to.y, { steps: 12 });
    await expect(page.locator('.wb-root')).toHaveAttribute('data-revision', String(revision));
    const pointerBefore = await page.evaluate(() => performance.now());
    await page.mouse.up();
    const released = await eventTime(page, 'pointerup', undefined, pointerBefore);
    mounts.push(await completed(page, released, revision));
    const savedMount = await page.evaluate(async () => {
      const db = await new Promise<IDBDatabase>((resolve, reject) => {
        const request = indexedDB.open('boardstudio-v2', 1);
        request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
      });
      try {
        return await new Promise<{ x: number; y: number }>((resolve, reject) => {
          const request = db.transaction('projects').objectStore('projects').get('live-preview-mount');
          request.onsuccess = () => resolve(request.result.caseBodies[0].mounts[0].at);
          request.onerror = () => reject(request.error);
        });
      } finally { db.close(); }
    });
    // Chromium may quantize injected mouse positions to CSS pixels. Verify the
    // saved world coordinate against that input precision, independent of zoom.
    const savedScreen = projectScenePoint(mountView, savedMount.x, savedMount.y, -9.2);
    expect(Math.abs(savedScreen.x - to.x)).toBeLessThanOrEqual(1);
    expect(Math.abs(savedScreen.y - to.y)).toBeLessThanOrEqual(1);
  }
  const scenarios = Object.fromEntries(Object.entries({ numeric, undo, gaskets, mounts }).map(([name, samples]) => [name, {
    actionToWorkerReply: distribution(samples.flatMap(sample => sample.actionToWorkerReplyMs === null ? [] : [sample.actionToWorkerReplyMs])),
    releaseToPaint: distribution(samples.map(sample => sample.releaseToPaintMs)),
    previewStartToPaint: distribution(samples.map(sample => sample.previewStartToPaintMs)),
    attribution: samples.map(sample => sample.attribution),
  }]));
  const pointerWork = distribution(renderer.pointerWork), pointerDraw = distribution(renderer.pointerDraw);
  const cadWasmSha256 = createHash('sha256').update(await readFile(new URL('../../cad/wasm/pkg/boardstudio_cadrum_wasm_bg.wasm', import.meta.url))).digest('hex');
  const sourceHashes = Object.fromEntries(await Promise.all(['splitMechanicalFixture.ts', '../src/useCaseGeneration.ts', '../src/CaseClient.ts', '../../cad/wasm/src/model/construction.rs', '../../cad/wasm/src/model/construction/cache.rs'].map(async path => [path, createHash('sha256').update(await readFile(new URL(path, import.meta.url))).digest('hex')])));
  const system = await browser.newBrowserCDPSession();
  const { gpu } = await system.send('SystemInfo.getInfo');
  await system.detach();
  const result = { provenance: { chromium: browser.version(), node: process.version, cpu: cpus()[0]?.model, viewport: page.viewportSize(), cadWasmSha256, sourceHashes, gpu, cpuRate }, initial, rapid,
    fixture: 'split: 70 switches; 29 gasket bodies and an authored tray mount', scenarios, pointerWork, pointerDraw, renderer,
    proposedTargets: { mainWorkP95: { limit: 4, passed: pointerWork.p95 <= 4 }, pointerSubmissionP95: { limit: 33, passed: pointerDraw.p95 <= 33 }, exactReleaseP95: { limit: 200, passed: Object.values(scenarios).every(scenario => scenario.releaseToPaint.p95 <= 200), scenarios: Object.fromEntries(Object.entries(scenarios).map(([name, scenario]) => [name, scenario.releaseToPaint.p95 <= 200])) } },
    measurement: 'Frame-after-draw is a paint opportunity; pointerDraw ends at render submission. Action timings start at in-page blur, Undo click, or pointerup, excluding automation dispatch. Five samples per action are diagnostic distributions, not a frozen baseline.' };
  console.info('Live preview performance', JSON.stringify(result));
  await info.attach('live-preview-performance', { body: JSON.stringify(result, null, 2), contentType: 'application/json' });
  if (process.env.BOARDSTUDIO_LIVE_PERF_RESULT) await writeFile(process.env.BOARDSTUDIO_LIVE_PERF_RESULT, JSON.stringify(result, null, 2));
  if (process.env.BOARDSTUDIO_LIVE_PREPARED) await writeFile(process.env.BOARDSTUDIO_LIVE_PREPARED, JSON.stringify(prepared));
});
