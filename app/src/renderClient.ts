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
  setState(state: unknown): void;
  setHandles(handles: unknown): void;
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

type RendererState = { hidden: string[]; selectedLayer: string; view: string; mode: 'shaded' | 'wireframe' | 'hybrid'; theme: string };
type ObjectDrag = { start(id: string): number | undefined; move(point: { x: number; y: number }): void; end(cancelled: boolean): void };
export type RendererCanvas = {
  setScene(scene: unknown): Promise<boolean>;
  setState(state: RendererState): void;
  setDrag(drag?: ObjectDrag): void;
  setHandles(handles: unknown): void;
  fit(): void;
  view(preset: 'top' | 'bottom' | 'isometric' | 'fit'): void;
  zoom(factor: number): void;
  dispose(): void;
};

let rendererModule: Promise<RendererWasm> | undefined;

async function loadRenderer(): Promise<RendererWasm> {
  rendererModule ??= (async () => {
    const started = performance.now();
    const module = await import('../../renderer/pkg/boardstudio_renderer_wasm.js');
    await module.default();
    performance.measure('boardstudio.renderer.wasm.cold-start', { start: started, end: performance.now() });
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
  }>();
  let frame = 0;
  let drag: ObjectDrag | undefined;
  let dragPlane: number | undefined;
  const render = () => { if (!frame && !disposed) frame = requestAnimationFrame(() => { frame = 0; if (!disposed) renderer.render(); }); };
  sceneWorker.onmessage = event => {
    const { id, prepared, error, elapsed } = event.data;
    const request = pending.get(id);
    pending.delete(id);
    if (!request) return;
    if (id !== sceneId || disposed) { request.resolve(false); return; }
    if (error) { request.reject(new Error(error)); return; }
    try {
      performance.measure('boardstudio.renderer.prepare', { start: performance.now() - elapsed, end: performance.now() });
      const start = performance.now();
      const accepted = renderer.setPreparedScene(
        cameraGeneration !== request.cameraGeneration
          ? { ...prepared, keepCamera: true }
          : prepared,
      );
      performance.measure('boardstudio.renderer.upload', { start, end: performance.now() });
      render();
      if (accepted && !measuredFirstScene) {
        measuredFirstScene = true;
        performance.measure('boardstudio.renderer.canvas.cold-start', { start: started, end: performance.now() });
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
      const bounds = canvas.getBoundingClientRect();
      const ratio = canvas.width / Math.max(bounds.width, 1);
      const point = renderer.pointOnPlane((event.clientX - bounds.left) * ratio, (event.clientY - bounds.top) * ratio, dragPlane);
      if (point.length) drag?.move({ x: point[0], y: point[1] });
    } else renderer.orbit(deltaX, deltaY);
    render();
  };
  const pointerUp = (event: PointerEvent) => {
    if (!dragging) return;
    dragging = false;
    if (dragPlane !== undefined) { drag?.end(event.type === 'pointercancel'); dragPlane = undefined; return; }
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
  const keyDown = (event: KeyboardEvent) => { if (event.key === 'Escape' && dragPlane !== undefined) { drag?.end(true); dragPlane = undefined; dragging = false; } };
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
      return new Promise<boolean>((resolve, reject) => {
        pending.set(id, { resolve, reject, cameraGeneration });
        sceneWorker.postMessage({ id, scene });
      });
    },
    setState(state) { if (!disposed) { renderer.setState(state); render(); } },
    setDrag(value) { drag = value; },
    setHandles(value) { if (!disposed) { renderer.setHandles(value); render(); } },
    fit() { if (!disposed) { cameraGeneration++; onInteract?.(); renderer.fit(); render(); } },
    view(preset) { if (!disposed) { cameraGeneration++; onInteract?.(); renderer.view(preset); render(); } },
    zoom(factor) { if (!disposed) { cameraGeneration++; onInteract?.(); renderer.zoom(factor); render(); } },
    dispose() {
      if (disposed) return;
      disposed = true;
      observer.disconnect();
      cancelAnimationFrame(frame);
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
