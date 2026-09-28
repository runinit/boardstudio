let modulePromise: Promise<typeof import('../../renderer/pkg/boardstudio_renderer_wasm.js')> | undefined;
let latest = 0;
self.onmessage = async (event: MessageEvent<{ id: number; scene: unknown; patch?: boolean }>) => {
  const { id, scene, patch } = event.data;
  latest = id;
  try {
    modulePromise ??= import('../../renderer/pkg/boardstudio_renderer_wasm.js').then(async module => { await module.default(); return module; });
    const module = await modulePromise;
    await new Promise(resolve => setTimeout(resolve, 0));
    if (id !== latest) return;
    const started = performance.now();
    // A body patch is intentionally prepared by the scene worker with no board,
    // PCB models, or unchanged case bodies in its input. The renderer merges
    // it into its retained GPU scene by stable id.
    const rawPrepared = module.prepareScene(scene);
    const prepared = patch && scene && typeof scene === 'object'
      ? { ...rawPrepared, removed: (scene as { removed?: string[] }).removed ?? [] }
      : rawPrepared;
    const buffers = new Set<ArrayBuffer>();
    const collect = (value: unknown) => {
      if (ArrayBuffer.isView(value) && value.buffer instanceof ArrayBuffer) buffers.add(value.buffer);
      else if (value && typeof value === 'object') Object.values(value).forEach(collect);
    };
    collect(prepared);
    self.postMessage({ id, prepared, patch: Boolean(patch), elapsed: performance.now() - started }, [...buffers]);
  } catch (cause) {
    modulePromise = undefined;
    self.postMessage({ id, error: String(cause) });
  }
};
