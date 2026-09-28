import { afterEach, expect, test, vi } from 'vitest';
import { createRendererCanvas } from './renderClient';

const render = vi.hoisted(() => vi.fn());
const preparedPatches = vi.hoisted(() => [] as unknown[]);
const preparedScenes = vi.hoisted(() => [] as unknown[]);
const patchAcceptance = vi.hoisted(() => ({ accepted: true }));
vi.mock('../../renderer/pkg/boardstudio_renderer_wasm.js', () => ({
  default: async () => {},
  Renderer: class {
    resize() {}
    render = render;
    dispose() {}
    free() {}
    setState() {}
    setHandles() { return 0; }
    setPreparedScene(scene: unknown) { preparedScenes.push(scene); return true; }
    setPreparedScenePatch(scene: unknown) { preparedPatches.push(scene); return patchAcceptance.accepted; }
    fit() {}
    view() {}
    zoom() {}
  },
}));

afterEach(() => { patchAcceptance.accepted = true; vi.unstubAllGlobals(); render.mockClear(); preparedScenes.length = 0; preparedPatches.length = 0; });

test('draws once for a coalesced frame and stays idle afterwards', async () => {
  const frames: FrameRequestCallback[] = [];
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => frames.push(callback));
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal('Worker', class { terminate() {} });
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('window', { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} });
  const canvas = { clientWidth: 640, clientHeight: 480, addEventListener() {}, removeEventListener() {} } as unknown as HTMLCanvasElement;
  const renderer = await createRendererCanvas(canvas);
  renderer.setState({ hidden: [], selectedLayer: '', view: 'assembled', mode: 'hybrid', theme: 'dark' });
  expect(frames).toHaveLength(1);
  frames.shift()!(0);
  expect(render).toHaveBeenCalledOnce();
  expect(frames).toHaveLength(0);
  renderer.dispose();
});

test('does not record per-frame renderer measures when profiling is disabled', async () => {
  vi.stubGlobal('location', { search: '' });
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { callback(0); return 1; });
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal('Worker', class { terminate() {} });
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('window', { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} });
  const canvas = { clientWidth: 640, clientHeight: 480, addEventListener() {}, removeEventListener() {} } as unknown as HTMLCanvasElement;
  const renderer = await createRendererCanvas(canvas);
  for (let index = 0; index < 200; index += 1) renderer.setHandles([]);
  expect(performance.getEntriesByName('boardstudio.renderer.handle-pose')).toHaveLength(0);
  renderer.dispose();
});

test('redraws handles only when their pose or appearance changes', async () => {
  const frames: FrameRequestCallback[] = [];
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => frames.push(callback));
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal('Worker', class { terminate() {} });
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('window', { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} });
  const canvas = { clientWidth: 640, clientHeight: 480, addEventListener() {}, removeEventListener() {} } as unknown as HTMLCanvasElement;
  const renderer = await createRendererCanvas(canvas);
  const handle = { id: 'mount', at: { x: 10, y: 20 }, z: 1, invalid: false };
  renderer.setHandles([handle]);
  frames.shift()!(0);
  render.mockClear();

  renderer.setHandles([{ ...handle, at: { ...handle.at } }]);
  expect(frames).toHaveLength(0);
  renderer.setHandles([{ ...handle, invalid: true }]);
  frames.shift()!(0);
  expect(render).toHaveBeenCalledOnce();
  renderer.setHandles([{ ...handle, at: { x: 11, y: 20 } }]);
  frames.shift()!(0);
  expect(render).toHaveBeenCalledTimes(2);
  renderer.setHandles([]);
  frames.shift()!(0);
  renderer.setHandles([]);
  expect(frames).toHaveLength(0);
  expect(render).toHaveBeenCalledTimes(3);
  renderer.dispose();
});

test('preserves a camera change made while a scene is being prepared', async () => {
  let worker: { onmessage?: (event: MessageEvent) => void; terminate(): void; postMessage(message: unknown): void };
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { callback(0); return 1; });
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('window', { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} });
  vi.stubGlobal('Worker', class {
    onmessage?: (event: MessageEvent) => void;
    constructor() { worker = this; }
    terminate() {}
    postMessage(message: unknown) { this.lastMessage = message; }
    lastMessage?: unknown;
  });
  const canvas = { clientWidth: 640, clientHeight: 480, addEventListener() {}, removeEventListener() {} } as unknown as HTMLCanvasElement;
  const renderer = await createRendererCanvas(canvas);
  const update = renderer.setScene({ keepCamera: false });
  renderer.view('top');
  worker!.onmessage!({ data: { id: 1, prepared: { revision: 1, keepCamera: false, objects: [] }, elapsed: 0 } } as MessageEvent);
  await expect(update).resolves.toBe(true);
  expect(preparedScenes).toEqual([{ revision: 1, keepCamera: true, objects: [] }]);
  renderer.dispose();
});

test('skips worker work when body geometry is unchanged', async () => {
  let worker: any;
  const frames: FrameRequestCallback[] = [];
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { frames.push(callback); return frames.length; });
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('window', { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} });
  vi.stubGlobal('Worker', class { onmessage?: (event: MessageEvent) => void; constructor() { worker = this; } terminate() {} postMessage = vi.fn(); });
  const canvas = { clientWidth: 640, clientHeight: 480, addEventListener() {}, removeEventListener() {} } as unknown as HTMLCanvasElement;
  const client = await createRendererCanvas(canvas);
  const body = { id: 'body', mesh: { positions: new Float32Array([1]), normals: new Float32Array([0]) } };
  const initial = client.setScene({ revision: 1, bodies: [body] });
  worker.onmessage!({ data: { id: 1, prepared: { revision: 1, objects: [] }, elapsed: 0 } } as MessageEvent);
  await initial;
  frames.shift()!(0);
  render.mockClear();
  const posts = worker.postMessage.mock.calls.length;
  await expect(client.setSceneBodies({ revision: 1, bodies: [{ ...body, mesh: { positions: new Float32Array([1]), normals: new Float32Array([0]) } }] })).resolves.toBe(true);
  expect(worker.postMessage).toHaveBeenCalledTimes(posts);
  frames.shift()!(0);
  expect(render).not.toHaveBeenCalled();
  client.zoom(1.1);
  await client.setSceneBodies({ revision: 2, bodies: [body] });
  expect(frames).toHaveLength(1);
  frames.shift()!(0);
  expect(render).toHaveBeenCalledOnce();
  client.dispose();
});


test('patches from accepted geometry after supersession or failure and removes missing bodies', async () => {
  let worker: any;
  vi.stubGlobal('requestAnimationFrame', vi.fn(() => 1));
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('window', { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} });
  vi.stubGlobal('Worker', class { onmessage?: (event: MessageEvent) => void; constructor() { worker = this; } terminate() {} postMessage = vi.fn(); });
  const canvas = { clientWidth: 640, clientHeight: 480, addEventListener() {}, removeEventListener() {} } as unknown as HTMLCanvasElement;
  const client = await createRendererCanvas(canvas);
  const body = (id: string, middle = 1) => ({ id, mesh: { positions: new Float32Array([0, middle, 2]), normals: new Float32Array([0, 0, 1]) } });
  const reply = (id: number, patch = true) => worker.onmessage({ data: { id, patch, prepared: { revision: id, objects: [] }, elapsed: 0 } });
  const initial = client.setScene({ revision: 1, bodies: [body('a'), body('b')] });
  reply(1, false); await initial;
  const superseded = client.setSceneBodies({ revision: 2, bodies: [body('a', 9), body('b')] });
  const latest = client.setSceneBodies({ revision: 3, bodies: [body('a', 9)] });
  await expect(superseded).resolves.toBe(false);
  expect(worker.postMessage.mock.lastCall[0].scene).toMatchObject({ bodies: [body('a', 9)], removed: ['b'] });
  reply(2); expect(preparedPatches).toHaveLength(0);
  reply(3); await expect(latest).resolves.toBe(true);
  const failed = client.setSceneBodies({ revision: 4, bodies: [body('a', 7)] });
  const rejected = expect(failed).rejects.toThrow('bad geometry');
  worker.onmessage({ data: { id: 4, error: 'bad geometry' } }); await rejected;
  const retry = client.setSceneBodies({ revision: 5, bodies: [body('a', 7)] });
  expect(worker.postMessage.mock.lastCall[0].scene.bodies).toEqual([body('a', 7)]);
  reply(5); await expect(retry).resolves.toBe(true);
  const pending = client.setSceneBodies({ revision: 6, bodies: [] });
  expect(worker.postMessage.mock.lastCall[0].scene.removed).toEqual(['a']);
  client.dispose(); await expect(pending).resolves.toBe(false);
});

test('acknowledges identical full scenes without worker preparation or another draw', async () => {
  let worker: any;
  const frames: FrameRequestCallback[] = [];
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { frames.push(callback); return frames.length; });
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('window', { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} });
  vi.stubGlobal('Worker', class { onmessage?: (event: MessageEvent) => void; constructor() { worker = this; } terminate() {} postMessage = vi.fn(); });
  const canvas = { clientWidth: 640, clientHeight: 480, addEventListener() {}, removeEventListener() {} } as unknown as HTMLCanvasElement;
  const client = await createRendererCanvas(canvas);
  const scene = (revision: number) => ({ revision, kind: 'assembly', theme: 'dark', keepCamera: true,
    board: { contours: [], thickness: 1.6 },
    bodies: [{ id: 'plate', mesh: { positions: new Float32Array([0, 1, 2]), normals: new Float32Array([0, 0, 1]) } }] });
  const reply = () => {
    const { id, scene } = worker.postMessage.mock.lastCall[0];
    worker.onmessage({ data: { id, prepared: { revision: scene.revision, objects: [] }, elapsed: 0 } });
  };
  const initial = client.setScene(scene(1)); reply(); await initial;
  frames.shift()!(0); render.mockClear();
  const unchanged = client.setScene(scene(2));
  expect(worker.postMessage).toHaveBeenCalledTimes(1);
  await expect(unchanged).resolves.toBe(true);
  expect(preparedPatches).toEqual([{ revision: 2, objects: [], removed: [] }]);
  frames.shift()!(0); expect(render).not.toHaveBeenCalled();

  // A no-op must not cancel a redraw already requested by a camera interaction.
  client.zoom(1.1);
  await client.setScene(scene(3));
  frames.shift()!(0); expect(render).toHaveBeenCalledOnce();

  // Metadata, geometry, and an explicit camera reset still take the full path.
  for (const change of [
    (value: ReturnType<typeof scene>) => { value.theme = 'light'; },
    (value: ReturnType<typeof scene>) => { value.board.thickness = 2; },
    (value: ReturnType<typeof scene>) => { value.bodies[0].mesh.positions[1] = 9; },
    (value: ReturnType<typeof scene>) => { value.keepCamera = false; },
  ]) {
    const next = scene(4); change(next);
    const before = worker.postMessage.mock.calls.length;
    const update = client.setScene(next);
    expect(worker.postMessage).toHaveBeenCalledTimes(before + 1);
    reply(); await update;
    const reset = client.setScene(scene(5)); reply(); await reset;
  }
  // Supersession compares with the accepted scene, never an in-flight replacement.
  const changed = scene(6); changed.board.thickness = 3;
  const obsolete = client.setScene(changed);
  const oldId = worker.postMessage.mock.lastCall[0].id;
  await expect(client.setScene(scene(7))).resolves.toBe(true);
  await expect(obsolete).resolves.toBe(false);
  const accepted = preparedScenes.length;
  worker.onmessage({ data: { id: oldId, prepared: { revision: 6, objects: [] }, elapsed: 0 } });
  expect(preparedScenes).toHaveLength(accepted);
  const patch = scene(8); patch.bodies[0].mesh.positions[1] = 4;
  const patched = client.setSceneBodies({ revision: 8, bodies: patch.bodies });
  const patchId = worker.postMessage.mock.lastCall[0].id;
  worker.onmessage({ data: { id: patchId, patch: true, prepared: { revision: 8, objects: [] }, elapsed: 0 } });
  await patched;
  const posts = worker.postMessage.mock.calls.length;
  const restored = client.setScene(scene(9));
  expect(worker.postMessage).toHaveBeenCalledTimes(posts + 1);
  reply(); await restored;
  while (frames.length) frames.shift()!(0);
  patchAcceptance.accepted = false;
  await expect(client.setScene(scene(8))).resolves.toBe(false);
  expect(frames).toHaveLength(0);
  client.dispose();
});
