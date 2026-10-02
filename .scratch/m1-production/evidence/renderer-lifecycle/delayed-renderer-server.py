#!/usr/bin/env python3
import http.server, json, mimetypes, pathlib, threading, time, urllib.parse

ROOT = pathlib.Path('/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/m1-release-20261001-8f509433/site-root').resolve()
PORT = 46884
state = {'requests': [], 'renderer_release_seconds': None, 'service_worker_disabled': True}
lock = threading.Lock()

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass
    def _json(self, obj):
        data = json.dumps(obj).encode()
        self.send_response(200); self.send_header('Content-Type','application/json'); self.send_header('Content-Length',str(len(data))); self.end_headers(); self.wfile.write(data)
    def do_GET(self):
        path = urllib.parse.urlsplit(self.path).path
        if path == '/__delay_state':
            with lock: self._json(dict(state))
            return
        if path == '/service-worker.js':
            self.send_error(404, 'Disabled only for this isolated race probe')
            return
        if path == '/assets/renderer/boardstudio_renderer_wasm.js':
            now = time.time()
            with lock: state['requests'].append({'path': path, 'arrived_unix': now})
            time.sleep(12)
            with lock: state['renderer_release_seconds'] = time.time()
        rel = path.lstrip('/') or 'index.html'
        target = (ROOT / rel).resolve()
        if not target.is_relative_to(ROOT) or not target.is_file():
            self.send_error(404); return
        data = target.read_bytes()
        self.send_response(200)
        self.send_header('Content-Type', mimetypes.guess_type(str(target))[0] or 'application/octet-stream')
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control','no-store')
        self.end_headers(); self.wfile.write(data)

http.server.ThreadingHTTPServer(('127.0.0.1', PORT), Handler).serve_forever()
