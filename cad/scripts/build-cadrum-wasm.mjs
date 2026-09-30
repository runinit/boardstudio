import { fileURLToPath } from 'node:url';
import { relative } from 'node:path';
import { spawnSync } from 'node:child_process';
import { rmSync } from 'node:fs';

const cadRoot = fileURLToPath(new URL('..', import.meta.url));
const image = 'boardstudio-cadrum-wasm:0.8.20-rust-1.98.0-wasi-sdk-33-bindgen-0.2.129';
const prepare = spawnSync(process.execPath, [fileURLToPath(new URL('./prepare-cadrum-occt.mjs', import.meta.url)), 'wasm'], {
  cwd: cadRoot,
  encoding: 'utf8',
});
if (prepare.status !== 0) throw new Error(prepare.stderr || 'Could not prepare verified OCCT WASM libraries');
const occtRoot = prepare.stdout.trim();
const occtRelative = relative(cadRoot, occtRoot).split('\\').join('/');
const runtime = process.env.CADRUM_CONTAINER_RUNTIME || (available('podman') ? 'podman' : available('docker') ? 'docker' : undefined);
if (!runtime) throw new Error('Building the Cadrum WASM module requires Podman or Docker');

run(runtime, ['build', '--file', 'wasm/Containerfile', '--tag', image, '.'], cadRoot);
const contractsRoot = fileURLToPath(new URL('../../contracts/rust', import.meta.url));
const contractsMount = `${contractsRoot}:/workspace/contracts/rust:ro`;
const cadMount = `${cadRoot}:/workspace/cad${runtime === 'podman' ? ':Z' : ''}`;
// wasm-pack 0.15 mistakes its previous package manifest for wasm-bindgen's
// dependency map. Remove only the generated manifest before rebuilding it.
rmSync(new URL('../wasm/pkg/package.json', import.meta.url), { force: true });
run(runtime, [
  'run', '--rm', '--volume', cadMount, '--volume', contractsMount, '--workdir', '/workspace/cad',
  '--env', `OCCT_ROOT=/workspace/cad/${occtRelative}`,
  '--env', 'CARGO_TARGET_DIR=/workspace/cad/wasm/target',
  image, 'wasm-pack', 'build', 'wasm', '--target', 'web', '--out-dir', 'pkg',
  '--out-name', 'boardstudio_cadrum_wasm', '--release', '--locked',
], cadRoot);

function available(command) {
  return spawnSync(command, ['--version'], { stdio: 'ignore' }).status === 0;
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed with exit code ${result.status}`);
}
