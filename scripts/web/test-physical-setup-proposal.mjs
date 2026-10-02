import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = new URL('../../', import.meta.url);
const temporary = await mkdtemp(join(tmpdir(), 'boardstudio-physical-setup-'));
try {
  const assets = join(temporary, 'assets');
  const build = spawnSync(process.execPath, ['scripts/web/build-layout-generators.mjs', assets], {
    cwd: root,
    stdio: 'inherit',
  });
  if (build.status !== 0) process.exitCode = build.status ?? 1;
  else {
    const moduleUrl = pathToFileURL(join(assets, 'layout-generators/src/index.js')).href;
    const tests = spawnSync('wasm-pack', [
      'test', '--node', 'web', '--',
      'reversible_proposal_uses_the_packaged_gateron_normalizer',
    ], {
      cwd: root,
      stdio: 'inherit',
      env: { ...process.env, BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL: moduleUrl },
    });
    if (tests.status !== 0) process.exitCode = tests.status ?? 1;
  }
} finally {
  await rm(temporary, { recursive: true, force: true });
}
