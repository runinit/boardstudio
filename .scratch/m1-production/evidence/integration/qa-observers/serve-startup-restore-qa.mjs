import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';

const [buildDirectory, portText = '0', ...flags] = process.argv.slice(2);
if (!buildDirectory) {
  throw new Error('usage: node serve-startup-restore-qa.mjs <maintained-build-dir> [port] [--hold-core] [--hold-cad] [--no-sw]');
}
const flagsSet = new Set(flags);
const holdCore = flagsSet.has('--hold-core');
const holdCad = flagsSet.has('--hold-cad');
const noServiceWorker = flagsSet.has('--no-sw');
const build = path.resolve(buildDirectory);
const provenance = JSON.parse(await fs.readFile(path.join(build, 'provenance.json'), 'utf8'));
const roots = {
  root: path.resolve(provenance.root.site),
  subpath: path.resolve(provenance.subpath.site),
};
const mime = {
  '.css': 'text/css', '.html': 'text/html', '.ico': 'image/x-icon', '.js': 'text/javascript',
  '.json': 'application/json', '.svg': 'image/svg+xml', '.wasm': 'application/wasm', '.boardstudio': 'application/zip',
};
const requests = [];
const gates = {
  core: { enabled: holdCore, released: false, pending: 0, waiters: [] },
  cad: { enabled: holdCad, released: false, pending: 0, waiters: [] },
};

function release(name) {
  const gate = gates[name];
  gate.released = true;
  for (const resolve of gate.waiters.splice(0)) resolve();
}

async function gated(name, bytes) {
  const gate = gates[name];
  if (!gate.enabled || gate.released) return;
  gate.pending += 1;
  await new Promise((resolve) => gate.waiters.push(resolve));
  gate.pending -= 1;
}

function send(response, status, body, type = 'application/json') {
  response.writeHead(status, {
    'content-type': type,
    'cache-control': 'no-store',
    'x-content-type-options': 'nosniff',
  });
  response.end(body);
}

const server = http.createServer(async (request, response) => {
  const url = new URL(request.url, 'http://localhost');
  if (url.pathname === '/__qa/status') {
    return send(response, 200, JSON.stringify({
      build: path.basename(build),
      gates: Object.fromEntries(Object.entries(gates).map(([name, gate]) => [name, {
        enabled: gate.enabled, released: gate.released, pending: gate.pending,
      }])),
      requests,
    }));
  }
  if (request.method === 'POST' && url.pathname === '/__qa/arm-core') {
    gates.core.enabled = true;
    gates.core.released = false;
    return send(response, 200, JSON.stringify({ armed: 'core' }));
  }
  if (request.method === 'POST' && url.pathname === '/__qa/arm-cad') {
    gates.cad.enabled = true;
    gates.cad.released = false;
    return send(response, 200, JSON.stringify({ armed: 'cad' }));
  }
  if (request.method === 'POST' && url.pathname === '/__qa/release-core') {
    release('core');
    return send(response, 200, JSON.stringify({ released: 'core' }));
  }
  if (request.method === 'POST' && url.pathname === '/__qa/release-cad') {
    release('cad');
    return send(response, 200, JSON.stringify({ released: 'cad' }));
  }
  if (url.pathname === '/boardstudio') {
    response.writeHead(308, { location: '/boardstudio/' }).end();
    return;
  }

  const subpath = url.pathname.startsWith('/boardstudio/');
  const root = subpath ? roots.subpath : roots.root;
  let relative = subpath ? url.pathname.slice('/boardstudio/'.length) : url.pathname.slice(1);
  if (!relative || relative.endsWith('/')) relative += 'index.html';
  relative = path.posix.normalize(relative);
  if (relative.startsWith('../') || relative === '..' || relative.startsWith('/')) {
    return send(response, 400, 'invalid path', 'text/plain');
  }
  const file = path.resolve(root, relative);
  if (!file.startsWith(`${root}${path.sep}`)) return send(response, 400, 'invalid path', 'text/plain');
  if (noServiceWorker && ['service-worker.js', 'boardstudio_offline_worker.js'].includes(relative)) {
    requests.push({ method: request.method, path: url.pathname, status: 404, qa: 'service worker disabled for cold-worker tests' });
    return send(response, 404, 'service worker disabled for cold-worker tests', 'text/plain');
  }

  try {
    const bytes = await fs.readFile(file);
    requests.push({ method: request.method, path: url.pathname, status: 200, bytes: bytes.byteLength });
    if (relative === 'assets/core-worker/m1_core_worker_bg.wasm') await gated('core', bytes);
    if (relative === 'assets/cad-worker/m1_cad_worker_bg.wasm') await gated('cad', bytes);
    return send(response, 200, bytes, mime[path.extname(file)] ?? 'application/octet-stream');
  } catch {
    requests.push({ method: request.method, path: url.pathname, status: 404, bytes: 0 });
    return send(response, 404, 'Not found', 'text/plain');
  }
});

server.listen(Number(portText), '127.0.0.1', () => {
  console.log(JSON.stringify({
    build: path.basename(build), roots,
    holdCore, holdCad, noServiceWorker,
    origin: `http://127.0.0.1:${server.address().port}`,
    pid: process.pid,
  }));
});
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => server.close(() => process.exit(0)));
