import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const releaseRoot = path.resolve(process.argv[2] ?? '');
const port = Number(process.argv[3] ?? '0');
const fixturePath = path.resolve(here, '../../../../../cad/bench/fixtures/gasketed-pair.json');
if (!process.argv[2]) throw new Error('usage: node server.mjs <release-site-root> [port]');
const server = http.createServer(async (request, response) => {
  const url = new URL(request.url, 'http://localhost');
  let file;
  if (url.pathname === '/__diag') file = path.join(here, 'page.html');
  else if (url.pathname === '/__diag/fixture') file = fixturePath;
  else file = path.resolve(releaseRoot, `.${url.pathname}`);
  if (!file.startsWith(here + path.sep) && !file.startsWith(releaseRoot + path.sep) && file !== fixturePath) return response.writeHead(403).end();
  const ext = path.extname(file);
  const mime = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.css': 'text/css' };
  try { const bytes = await fs.readFile(file); response.writeHead(200, { 'content-type': mime[ext] ?? 'application/octet-stream', 'cache-control': 'no-store' }); response.end(bytes); }
  catch { response.writeHead(404).end('not found'); }
});
server.listen(port, '127.0.0.1', () => console.log(JSON.stringify({ pid: process.pid, origin: `http://127.0.0.1:${server.address().port}`, releaseRoot })));
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => server.close(() => process.exit(0)));
