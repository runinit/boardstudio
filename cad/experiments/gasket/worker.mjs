import init, { gasket_experiment, gasket_experiment_mesh } from '/cadrum/gasket_experiment.js';
import Module from '/manifold/manifold.js';
import { manifoldBottom } from '/experiment/manifold.mjs';
let cadrum, manifold;
self.onmessage = async ({ data }) => {
  try {
    const { input, variant, timed = false, mesh = false } = data;
    if (variant === 'manifold') {
      if (!manifold) { manifold = await Module(); manifold.setup(); }
      const result = manifoldBottom(manifold, JSON.parse(input), mesh);
      self.postMessage({ result, heapBytes: manifold.HEAPU8?.buffer.byteLength ?? null });
    } else {
      cadrum ??= await init();
      const result = JSON.parse(gasket_experiment(input, variant, timed));
      if (mesh) result.mesh = JSON.parse(gasket_experiment_mesh(input, variant));
      self.postMessage({ result, heapBytes: cadrum.memory.buffer.byteLength });
    }
  } catch (error) { self.postMessage({ error: String(error.stack ?? error) }); }
};
