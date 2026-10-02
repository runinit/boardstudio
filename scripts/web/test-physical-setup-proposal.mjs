import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { fileURLToPath } from 'node:url';

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
    const assetRoot = resolve(assets);
    const server = createServer(async (request, response) => {
      const path = new URL(request.url ?? '/', 'http://localhost').pathname;
      const allowed = new Set([
        '/layout-generators/src/index.js',
        '/layout-generators/generated/catalogue.mjs',
      ]);
      if (!allowed.has(path)) {
        response.writeHead(404).end();
        return;
      }
      try {
        const bytes = await readFile(join(assetRoot, path.slice(1)));
        response.writeHead(200, {
          'Access-Control-Allow-Origin': '*',
          'Content-Type': 'text/javascript',
        }).end(bytes);
      } catch {
        response.writeHead(500).end();
      }
    });
    await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));
    const address = server.address();
    if (!address || typeof address === 'string') throw new Error('Generator asset server did not start');
    const moduleUrl = `http://127.0.0.1:${address.port}/layout-generators/src/index.js`;
    const tests = spawn('wasm-pack', [
      'test', '--headless', '--chrome', '--chromedriver', '/usr/bin/chromedriver', 'web', '--bin', 'boardstudio-web', '--',
      'pcb_part_',
    ], {
      cwd: fileURLToPath(root),
      stdio: 'inherit',
      env: { ...process.env, BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL: moduleUrl },
    });
    let status;
    try {
      status = await new Promise((resolveExit, reject) => {
        tests.once('error', reject);
        tests.once('exit', (code) => resolveExit(code ?? 1));
      });
    } finally {
      await new Promise((resolveClose) => server.close(resolveClose));
    }
    if (status !== 0) process.exitCode = status;
  }
} finally {
  await rm(temporary, { recursive: true, force: true });
}
