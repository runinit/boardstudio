import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const stage = resolve(process.argv[2]);
const image = 'boardstudio-cadrum-wasm:0.8.20-rust-1.98.0-wasi-sdk-33-bindgen-0.2.129';
const runtime = process.env.CADRUM_CONTAINER_RUNTIME ?? 'podman';
const occt = execFileSync(process.execPath, [resolve(root, 'scripts/prepare-cadrum-occt.mjs'), 'wasm'], { encoding: 'utf8' }).trim();
execFileSync(runtime, ['run', '--rm', '--pull=never',
  '--volume', `${stage}:/experiment:Z`, '--volume', `${occt}:/occt:ro`,
  '--workdir', '/experiment/cad', '--env', 'OCCT_ROOT=/occt',
  '--env', 'CARGO_TARGET_DIR=/experiment/target-wasm', image,
  'wasm-pack', 'build', 'wasm', '--target', 'web', '--out-dir', 'pkg',
  '--out-name', 'gasket_experiment', '--release', '--locked'], { stdio: 'inherit' });
