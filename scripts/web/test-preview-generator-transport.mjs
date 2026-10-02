// Exercise Rust serialization and correlation through a real browser module worker.
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const assets = await mkdtemp(join(tmpdir(), 'boardstudio-preview-transport-'));
const run = (command, args, env = process.env) => new Promise((resolve, reject) => {
  const child = spawn(command, args, { cwd: root, env, stdio: 'inherit' });
  child.on('error', reject);
  child.on('exit', (code) => code === 0 ? resolve() : reject(new Error(`${command} exited ${code}`)));
});
const server = createServer(async (request, response) => {
  const path = resolve(assets, `.${new URL(request.url, 'http://localhost').pathname}`);
  if (!path.startsWith(assets + sep)) {
    response.writeHead(403).end();
    return;
  }
  try {
    const source = await readFile(path);
    response.writeHead(200, { 'Content-Type': 'text/javascript', 'Access-Control-Allow-Origin': '*' });
    response.end(source);
  } catch {
    response.writeHead(404).end();
  }
});
try {
  await run(process.execPath, ['scripts/web/build-preview-generator.mjs', assets]);
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const url = `http://127.0.0.1:${server.address().port}/preview-generator/worker.mjs`;
  await run('wasm-pack', [
    'test', '--headless', '--chrome', 'web', '--test', 'preview_generator_transport',
  ], { ...process.env, BOARDSTUDIO_TEST_PREVIEW_WORKER_URL: url });
} finally {
  server.closeAllConnections();
  await new Promise((resolve) => server.close(resolve));
  await rm(assets, { recursive: true, force: true });
}
