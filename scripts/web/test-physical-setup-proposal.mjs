// Keep the existing entrypoint while sharing the strict packaged-generator harness.
import { spawnSync } from 'node:child_process';

const result = spawnSync('python3', [
  'scripts/run-wasm-tests.py', '--files',
  'web/src/presentation/parts/catalogue.rs',
  'web/src/presentation/pcb_wiring/part_input_settings.rs',
], { cwd: new URL('../../', import.meta.url), stdio: 'inherit' });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
