import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn, spawnSync } from 'node:child_process';

const root = new URL('../../', import.meta.url);
const temporary = await mkdtemp(join(tmpdir(), 'boardstudio-portable-models-'));
try {
  const assets = join(temporary, 'assets');
  const build = spawnSync(process.execPath, ['scripts/web/build-layout-generators.mjs', assets], {
    cwd: root,
    stdio: 'inherit',
  });
  if (build.status !== 0) process.exitCode = build.status ?? 1;
  else {
    const service = await import(pathToFileURL(join(assets, 'layout-generators.js')).href);
    const paths = [
      '${KIPRJMOD}/models/boardstudio/kiswitch/SW_Cherry_MX_PCB.stp',
      '${KIPRJMOD}/models/boardstudio/thqwgd001/THQWGD001-rotation.stp',
      '${EG_INFUSED_KIM_3D_MODELS}/diode/example.wrl',
      'C:/untrusted/model.step',
    ];
    const actual = service.modelAssetIdsForPaths?.(paths);
    const expected = [
      'ergogen:model:kiswitch/SW_Cherry_MX_PCB.stp',
      'ergogen:model:thqwgd001/THQWGD001-rotation.stp',
      'ergogen:model:infused-kim/diode/example.wrl',
      null,
    ];
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      throw new Error(`Packaged Ergogen path identity mismatch: ${JSON.stringify(actual)}`);
    }
    const assetRoot = resolve(assets);
    const server = createServer(async (request, response) => {
      const path = resolve(assetRoot, `.${new URL(request.url ?? '/', 'http://127.0.0.1').pathname}`);
      if (!path.startsWith(`${assetRoot}/`)) {
        response.writeHead(404).end();
        return;
      }
      try {
        const body = await readFile(path);
        response.setHeader('Access-Control-Allow-Origin', '*');
        response.setHeader('Content-Type', 'text/javascript');
        response.writeHead(200).end(body);
      } catch {
        response.writeHead(404).end();
      }
    });
    await new Promise((resolveListen, rejectListen) => {
      server.once('error', rejectListen);
      server.listen(0, '127.0.0.1', resolveListen);
    });
    const address = server.address();
    if (!address || typeof address === 'string') throw new Error('Test asset server did not bind a TCP port');
    const moduleUrl = `http://127.0.0.1:${address.port}/layout-generators/src/index.js`;
    try {
      const tests = await new Promise((resolveExit, rejectExit) => {
        const child = spawn('wasm-pack', [
          'test',
          '--headless',
          '--chrome',
          'web',
          '--',
          'native_preview_paths_use_the_packaged_ergogen_asset_identity_helper',
        ], {
          cwd: root,
          stdio: 'inherit',
          env: { ...process.env, BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL: moduleUrl },
        });
        child.once('error', rejectExit);
        child.once('close', (code) => resolveExit(code ?? 1));
      });
      if (tests !== 0) process.exitCode = tests;
    } finally {
      await new Promise((resolveClose) => server.close(resolveClose));
    }
  }
} finally {
  await rm(temporary, { recursive: true, force: true });
}
