import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const build = path.resolve(process.argv[2] ?? '');
const port = Number(process.argv[3] ?? '0');
if (!process.argv[2]) throw new Error('usage: node serve-release.mjs <build-output> [port] [root-update-dir]');
const updateDirectory = process.argv[4] ? path.resolve(process.argv[4]) : null;
const provenance = JSON.parse(await fs.readFile(path.join(build, 'provenance.json'), 'utf8'));
const root = path.resolve(provenance.root.site);
const subpath = path.resolve(provenance.subpath.site);
const mime = {
  '.css': 'text/css', '.html': 'text/html', '.ico': 'image/x-icon', '.js': 'text/javascript',
  '.mjs': 'text/javascript', '.json': 'application/json', '.svg': 'image/svg+xml', '.wasm': 'application/wasm', '.boardstudio': 'application/zip',
};
let updatedRootWorker = false;
const requests = [];

function send(response, status, bytes, type) {
  response.writeHead(status, { 'content-type': type, 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' });
  response.end(bytes);
}

const server = http.createServer(async (request, response) => {
  const url = new URL(request.url, 'http://localhost');
  if (url.pathname === '/__qa/worker-update' && request.method === 'POST') {
    updatedRootWorker = true;
    return send(response, 200, 'root update worker enabled', 'text/plain');
  }
  if (url.pathname === '/__qa/requests') {
    return send(response, 200, JSON.stringify(requests), 'application/json');
  }
  if (url.pathname === '/boardstudio') {
    response.writeHead(308, { location: '/boardstudio/' }).end();
    return;
  }
  const inSubpath = url.pathname.startsWith('/boardstudio/');
  const base = inSubpath ? subpath : root;
  let relative = inSubpath ? url.pathname.slice('/boardstudio/'.length) : url.pathname.slice(1);
  if (!relative || relative.endsWith('/')) relative += 'index.html';
  const normalized = path.posix.normalize(relative);
  if (normalized.startsWith('../') || normalized === '..' || normalized.startsWith('/')) {
    return send(response, 400, 'invalid path', 'text/plain');
  }
  let file = path.resolve(base, normalized);
  if (!inSubpath && updatedRootWorker && updateDirectory && normalized === 'service-worker.js') {
    file = path.join(updateDirectory, 'service-worker.js');
  } else if (!inSubpath && updatedRootWorker && updateDirectory && normalized === 'boardstudio_offline_worker.js') {
    file = path.join(updateDirectory, 'boardstudio_offline_worker.js');
  }
  if (!file.startsWith(base + path.sep) && !file.startsWith((updateDirectory ?? base) + path.sep)) {
    return send(response, 400, 'invalid path', 'text/plain');
  }
  try {
    const bytes = await fs.readFile(file);
    requests.push({ method: request.method, path: url.pathname, status: 200, bytes: bytes.byteLength,
      updateWorker: updatedRootWorker && !inSubpath && ['service-worker.js', 'boardstudio_offline_worker.js'].includes(normalized) });
    send(response, 200, bytes, mime[path.extname(file)] ?? 'application/octet-stream');
  } catch {
    requests.push({ method: request.method, path: url.pathname, status: 404, bytes: 0, updateWorker: false });
    send(response, 404, 'Not found', 'text/plain');
  }
});

server.listen(port, '127.0.0.1', () => {
  console.log(JSON.stringify({ build: path.basename(build), root, subpath,
    updateDirectory, origin: `http://127.0.0.1:${server.address().port}`, pid: process.pid }));
});
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => server.close(() => process.exit(0)));
