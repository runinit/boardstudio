import { changedSceneBodies, type SceneBody } from './sceneBodies';
import { recordCadMeasure, scheduleCadPaint } from './cadPerformance';

type RendererWasm = {
  default: () => Promise<unknown>;
  Renderer: new (canvas: HTMLCanvasElement) => RendererInstance;
  decodeStl(bytes: Uint8Array): ModelMesh;
  decodeWrl(bytes: Uint8Array): ModelMesh;
};

type RendererInstance = {
  resize(width: number, height: number): void;
  setScene(scene: unknown): boolean;
  setPreparedScene(scene: unknown): boolean;
  setPreparedScenePatch(patch: unknown): boolean;
  setState(state: unknown): void;
  setHandles(handles: unknown): number;
  pointOnPlane(x:number,y:number,z:number): Float32Array;
  render(): void;
  fit(): void;
  view(preset: string): void;
  orbit(deltaX: number, deltaY: number): void;
  zoom(factor: number): void;
  pick(x: number, y: number): string | undefined;
  dispose(): void;
  free(): void;
};

type ModelMesh = {
  positions: Float32Array;
  normals: Float32Array;
  colors?: Float32Array;
};

type RendererState = { hidden: string[]; selectedLayer: string; view: string; mode: 'shaded' | 'wireframe' | 'hybrid'; theme: string; explodeAmount?: number; sectionPlane?: string; sectionPosition?: number; showSectionPlane?: boolean; showHidden?: boolean; colors?: Record<string, string> };
type ObjectDrag = { start(id: string): number | undefined; move(point: { x: number; y: number }): void; end(cancelled: boolean): void };
export type RendererCanvas = {
  setScene(scene: unknown): Promise<boolean>;
  setSceneBodies(patch: { revision: number; bodies: SceneBody[]; removed?: string[] }): Promise<boolean>;
  setState(state: RendererState): void;
  setDrag(drag?: ObjectDrag): void;
  setHandles(handles: unknown): void;
  fit(): void;
  view(preset: 'top' | 'bottom' | 'isometric' | 'fit'): void;
  zoom(factor: number): void;
  dispose(): void;
};

// Scene inputs are immutable snapshots. Bound comparison work and fall back to
// preparation for unfamiliar values or large metadata; mesh buffers are handled
// by changedSceneBodies, which already defines exact body equivalence.
function sameSceneBase(previous: unknown, next: unknown): boolean {
  let remaining = 10_000;
  const equal = (a: unknown, b: unknown, depth: number): boolean => {
    if (--remaining < 0 || depth > 32) return false;
    if (a === b) return true;
    if (!a || !b || typeof a !== 'object' || typeof b !== 'object') return false;
    if (Array.isArray(a) || Array.isArray(b)) {
      return Array.isArray(a) && Array.isArray(b) && a.length === b.length
        && a.every((value, index) => equal(value, b[index], depth + 1));
    }
    if (Object.getPrototypeOf(a) !== Object.prototype || Object.getPrototypeOf(b) !== Object.prototype) return false;
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length && keys.every(key =>
      Object.hasOwn(b, key) && equal((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key], depth + 1));
  };
  const metadata = (value: unknown) => Object.fromEntries(Object.entries(value as object)
    .filter(([key]) => key !== 'revision' && key !== 'keepCamera' && key !== 'bodies'));
  return Boolean(previous && next && typeof previous === 'object' && typeof next === 'object')
    && equal(metadata(previous), metadata(next), 0);
}

let rendererModule: Promise<RendererWasm> | undefined;

async function loadRenderer(): Promise<RendererWasm> {
  rendererModule ??= (async () => {
    const started = performance.now();
    const module = await import('../../renderer/pkg/boardstudio_renderer_wasm.js');
    await module.default();
    recordCadMeasure('boardstudio.renderer.wasm.cold-start', { start: started, end: performance.now() });
    return module as RendererWasm;
  })().catch(cause => { rendererModule = undefined; throw cause; });
  return rendererModule;
}

export async function readMeshModel(bytes: Uint8Array, filename: string): Promise<ModelMesh> {
  if (!/\.(?:stl|wrl)$/i.test(filename))
    throw new Error('Choose a STEP, STL, or WRL model');
  if (!bytes.length || bytes.length > 32 * 1024 * 1024)
    throw new Error('Model must be between 1 byte and 32 MiB');
  const wasm = await loadRenderer();
  if (/\.stl$/i.test(filename)) return wasm.decodeStl(bytes);
  return wasm.decodeWrl(bytes);
}

export async function createRendererCanvas(
  canvas: HTMLCanvasElement,
  onPick?: (id: string) => void,
  onInteract?: () => void,
): Promise<RendererCanvas> {
  const started = performance.now();
  const wasm = await loadRenderer();
  const renderer = new wasm.Renderer(canvas);
  let disposed = false;
  const sceneWorker = new Worker(new URL('./scene.worker.ts', import.meta.url), { type: 'module' });
  let sceneId = 0;
  // A prepared scene may arrive after the user has changed the camera. Keep
  // that interaction authoritative over the keepCamera flag captured when
  // the worker request was posted.
  let cameraGeneration = 0;
  const pending = new Map<number, {
    resolve: (accepted: boolean) => void;
    reject: (error: Error) => void;
    cameraGeneration: number;
    scene: unknown;
    bodies: SceneBody[];
    started: number;
    transferredBytes: number;
  }>();
  let frame = 0;
  let needsDraw = false;
  let handlesKey: string | undefined;
  let drag: ObjectDrag | undefined;
  let appliedBodies: SceneBody[] = [];
  let paintedScene: unknown;
  let baseScene: { theme?: string; mechanicalStack?: unknown[] } = {};
  let hidden: string[] = [];
  let cancelPaint = () => {};
  let dragPlane: number | undefined;
  let pointerSample: { x: number; y: number; at: number } | undefined;
  const flushPointer = () => {
    const sample = pointerSample; pointerSample = undefined;
    if (!sample || dragPlane === undefined) return undefined;
    const bounds = canvas.getBoundingClientRect();
    const ratio = canvas.width / Math.max(bounds.width, 1);
    const point = renderer.pointOnPlane((sample.x - bounds.left) * ratio, (sample.y - bounds.top) * ratio, dragPlane);
    const start = performance.now();
    if (point.length) drag?.move({ x: point[0], y: point[1] });
    recordCadMeasure('boardstudio.renderer.pointer-work', { start, end: performance.now() });
    return sample.at;
  };
  const render = (redraw = true) => {
    needsDraw ||= redraw;
    if (!frame && !disposed) frame = requestAnimationFrame(() => { if (!disposed) {
    const pointerAt = flushPointer();
    if (needsDraw) { needsDraw = false; renderer.render(); }
    if (pointerAt !== undefined) recordCadMeasure('boardstudio.renderer.pointer-draw', { start: pointerAt, end: performance.now(), detail: { method: 'render-submission' } });
    cancelPaint();
    const snapshot = paintedScene;
    cancelPaint = scheduleCadPaint(snapshot, hidden, () => !disposed && paintedScene === snapshot);
  } frame = 0; }); };
  sceneWorker.onmessage = event => {
    const { id, prepared, patch, error, elapsed } = event.data;
    const request = pending.get(id);
    pending.delete(id);
    if (!request) return;
    if (id !== sceneId || disposed) { request.resolve(false); return; }
    if (error) { request.reject(new Error(error)); return; }
    try {
      recordCadMeasure('boardstudio.renderer.prepare', { start: performance.now() - elapsed, end: performance.now() });
      const start = performance.now();
      const value = cameraGeneration !== request.cameraGeneration ? { ...prepared, keepCamera: true } : prepared;
      const accepted = patch
        ? renderer.setPreparedScenePatch(value)
        : renderer.setPreparedScene(value);
      recordCadMeasure('boardstudio.renderer.upload', { start, end: performance.now(), detail: { patch: Boolean(patch), objects: prepared.objects.length } });
      if (accepted) {
        if (!patch) baseScene = request.scene as typeof baseScene;
        appliedBodies = request.bodies;
        paintedScene = request.scene;
        if (patch) recordCadMeasure('boardstudio.renderer.body-patch', {
          start: request.started, end: performance.now(), detail: {
            preparedObjects: prepared.objects.length, transferredBytes: request.transferredBytes,
            unchangedBoardModels: true,
          },
        });
        render();
      }
      if (accepted && !measuredFirstScene) {
        measuredFirstScene = true;
        recordCadMeasure('boardstudio.renderer.canvas.cold-start', { start: started, end: performance.now() });
      }
      request.resolve(accepted);
    } catch (cause) { request.reject(new Error(String(cause))); }
  };
  sceneWorker.onerror = event => { event.preventDefault(); for (const request of pending.values()) request.reject(new Error('Scene preparation failed; reopen the preview to retry')); pending.clear(); };
  let measuredFirstScene = false;
  let dragging = false;
  let moved = false;
  let previous = { x: 0, y: 0 };
  let down = { x: 0, y: 0 };

  const resize = () => {
    if (disposed) return;
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    const width = Math.max(1, Math.round(canvas.clientWidth * ratio));
    const height = Math.max(1, Math.round(canvas.clientHeight * ratio));
    if (canvas.width !== width) canvas.width = width;
    if (canvas.height !== height) canvas.height = height;
    renderer.resize(width, height);
    render();
  };
  const observer = new ResizeObserver(resize);
  observer.observe(canvas);
  resize();

  const pointerDown = (event: PointerEvent) => {
    dragging = true;
    moved = false;
    cameraGeneration++;
    onInteract?.();
    previous = down = { x: event.clientX, y: event.clientY };
    canvas.setPointerCapture(event.pointerId);
    if (drag) {
      const bounds = canvas.getBoundingClientRect();
      const ratio = canvas.width / Math.max(bounds.width, 1);
      const id = renderer.pick((event.clientX - bounds.left) * ratio, (event.clientY - bounds.top) * ratio);
      if (id) dragPlane = drag.start(id);
    }
  };
  const pointerMove = (event: PointerEvent) => {
    if (!dragging) return;
    const deltaX = event.clientX - previous.x;
    const deltaY = event.clientY - previous.y;
    previous = { x: event.clientX, y: event.clientY };
    if (Math.hypot(event.clientX - down.x, event.clientY - down.y) > 3) moved = true;
    if (dragPlane !== undefined) {
      pointerSample = { x: event.clientX, y: event.clientY, at: performance.now() };
    } else renderer.orbit(deltaX, deltaY);
    render();
  };
  const pointerUp = (event: PointerEvent) => {
    if (!dragging) return;
    dragging = false;
    if (dragPlane !== undefined) {
      if (event.type !== 'pointercancel') flushPointer(); else pointerSample = undefined;
      drag?.end(event.type === 'pointercancel'); dragPlane = undefined; return;
    }
    if (!moved) {
      const bounds = canvas.getBoundingClientRect();
      const ratio = canvas.width / Math.max(bounds.width, 1);
      const id = renderer.pick((event.clientX - bounds.left) * ratio, (event.clientY - bounds.top) * ratio);
      if (id) onPick?.(id);
    }
  };
  const wheel = (event: WheelEvent) => {
    event.preventDefault();
    cameraGeneration++;
    onInteract?.();
    renderer.zoom(Math.exp(Math.sign(event.deltaY) * Math.min(Math.abs(event.deltaY) * 0.001, 1)));
    render();
  };
  const keyDown = (event: KeyboardEvent) => { if (event.key === 'Escape' && dragPlane !== undefined) { pointerSample = undefined; drag?.end(true); dragPlane = undefined; dragging = false; } };
  window.addEventListener('keydown', keyDown);
  canvas.addEventListener('pointerdown', pointerDown);
  canvas.addEventListener('pointermove', pointerMove);
  canvas.addEventListener('pointerup', pointerUp);
  canvas.addEventListener('pointercancel', pointerUp);
  canvas.addEventListener('wheel', wheel, { passive: false });

  return {
    setScene(scene) {
      if (disposed) return Promise.resolve(false);
      for (const request of pending.values()) request.resolve(false);
      pending.clear();
      const id = ++sceneId;
      const input = scene as { revision?: number; keepCamera?: boolean; bodies?: SceneBody[] };
      const bodies = input.bodies ?? [];
      if (paintedScene && input.keepCamera === true && sameSceneBase(baseScene, scene)
        && bodies.length === appliedBodies.length && bodies.every((body, index) => body.id === appliedBodies[index].id)
        && !changedSceneBodies(appliedBodies, bodies).bodies.length) {
        // Advance the renderer's own revision guard without rebuilding identical
        // objects. Explicit camera resets and changed scene metadata never enter.
        const accepted = renderer.setPreparedScenePatch({ revision: input.revision ?? 0, objects: [], removed: [] });
        if (accepted) {
          baseScene = scene as typeof baseScene;
          appliedBodies = bodies;
          paintedScene = scene;
          render(false);
        }
        return Promise.resolve(accepted);
      }
      return new Promise<boolean>((resolve, reject) => {
        pending.set(id, { resolve, reject, cameraGeneration, scene,
          bodies, started: performance.now(), transferredBytes: 0 });
        sceneWorker.postMessage({ id, scene });
      });
    },
    setSceneBodies(patch) {
      if (disposed) return Promise.resolve(false);
      for (const request of pending.values()) request.resolve(false);
      pending.clear();
      const id = ++sceneId;
      const delta = changedSceneBodies(appliedBodies, patch.bodies);
      const removed = [...new Set([...delta.removed, ...(patch.removed ?? [])])];
      const nextScene = { bodies: patch.bodies };
      if (!delta.bodies.length && !removed.length) {
        paintedScene = nextScene;
        appliedBodies = patch.bodies;
        // Identical geometry is already visible; only advance its paint attribution.
        render(false);
        return Promise.resolve(true);
      }
      const transferredBytes = delta.bodies.reduce((sum, body) => sum + body.mesh.positions.byteLength
        + body.mesh.normals.byteLength + (body.mesh.colors?.byteLength ?? 0), 0);
      const scene = { kind: 'bodyPatch', revision: patch.revision, bodies: delta.bodies, removed, theme: baseScene.theme, mechanicalStack: baseScene.mechanicalStack };
      return new Promise<boolean>((resolve, reject) => {
        pending.set(id, { resolve, reject, cameraGeneration, scene: nextScene, bodies: patch.bodies,
          started: performance.now(), transferredBytes });
        sceneWorker.postMessage({ id, scene, patch: true });
      });
    },
    setState(state) { if (!disposed) { hidden = state.hidden; renderer.setState(state); render(); } },
    setDrag(value) { drag = value; },
    setHandles(value) { if (!disposed) {
      const key = JSON.stringify(value);
      if (key === handlesKey) return;
      const start = performance.now();
      const uploadedHandles = renderer.setHandles(value);
      handlesKey = key;
      recordCadMeasure('boardstudio.renderer.handle-pose', { start, end: performance.now(), detail: { uploadedHandles } });
      render();
    } },
    fit() { if (!disposed) { cameraGeneration++; onInteract?.(); renderer.fit(); render(); } },
    view(preset) { if (!disposed) { cameraGeneration++; onInteract?.(); renderer.view(preset); render(); } },
    zoom(factor) { if (!disposed) { cameraGeneration++; onInteract?.(); renderer.zoom(factor); render(); } },
    dispose() {
      if (disposed) return;
      disposed = true;
      observer.disconnect();
      cancelAnimationFrame(frame);
      cancelPaint();
      appliedBodies = [];
      paintedScene = undefined;
      sceneWorker.terminate();
      for (const request of pending.values()) request.resolve(false);
      pending.clear();
      window.removeEventListener('keydown', keyDown);
      canvas.removeEventListener('pointerdown', pointerDown);
      canvas.removeEventListener('pointermove', pointerMove);
      canvas.removeEventListener('pointerup', pointerUp);
      canvas.removeEventListener('pointercancel', pointerUp);
      canvas.removeEventListener('wheel', wheel);
      renderer.dispose();
      renderer.free();
    },
  };
}
