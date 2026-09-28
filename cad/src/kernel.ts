import { cadStage, sampleCadMemory } from './metrics.ts';

let kernel: Promise<typeof import('../wasm/pkg/boardstudio_cadrum_wasm.js')> | undefined;
let kernelMemory: WebAssembly.Memory | undefined;

/** Loads the local Cadrum WASM only when CAD is first requested. */
export async function getKernel(): Promise<typeof import('../wasm/pkg/boardstudio_cadrum_wasm.js')> {
  if (!kernel) {
    kernel = (async () => {
      const finish = cadStage('kernelInitialization');
      try {
        const bindings = await import('../wasm/pkg/boardstudio_cadrum_wasm.js');
        const node = (globalThis as { process?: { versions?: { node?: string } } }).process?.versions?.node;
        if (node) {
          const { readFile } = await import('node:fs/promises');
          const initialized = await bindings.default({
            module_or_path: await readFile(new URL('../wasm/pkg/boardstudio_cadrum_wasm_bg.wasm', import.meta.url)),
          });
          kernelMemory = initialized.memory;
        } else {
          kernelMemory = (await bindings.default()).memory;
        }
        sampleCadMemory(kernelMemory);
        return bindings;
      } finally { finish(); }
    })().catch((cause) => {
      kernel = undefined;
      throw cause;
    });
  }

  const bindings = await kernel;
  sampleCadMemory(kernelMemory);
  return bindings;
}

