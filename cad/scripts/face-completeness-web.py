#!/usr/bin/env python3
"""Run the packaged WEB step-face-completeness regression with a supplied driver."""
import hashlib
import http.server
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import threading

import cad_build

CAD = Path(__file__).resolve().parents[1]
ROOT = CAD.parent
CASE = "step-face-completeness"


def preflight(cad=CAD):
    package = cad / "wasm/pkg/boardstudio_cad_wasm.js"
    wasm = package.with_name("boardstudio_cad_wasm_bg.wasm")
    stamp = cad / ".cache/cad/wasm-build.sha256"
    if not all(path.is_file() for path in (package, wasm, stamp)):
        raise RuntimeError("CAD WEB package or freshness stamp is missing")
    if stamp.read_text().strip() not in (cad_build.wasm_inputs_digest(no_opt=True), cad_build.wasm_inputs_digest()):
        raise RuntimeError("CAD WEB package is stale")
    return package, wasm


def regression_body(source):
    prefix = '''// Actual packaged provider: node this-file /absolute/path/to/package.js [--web].
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

(async () => {
const cad = await require('./load-provider.cjs')(process.argv[2]);
const repoRoot = path.resolve(__dirname, '../../..');
const fixturePath = path.join(repoRoot, 'cad/monstertruck/tests/fixtures/circular-segment-sliver-prism.step');

const bytes = fs.readFileSync(fixturePath);
const digest = crypto.createHash('sha256').update(bytes).digest('hex');

const result = cad.read_step_model(new Uint8Array(bytes), { sourceSha256: digest });

'''
    suffix = "\n\n})().catch(error => { console.error(error); process.exitCode = 1; });\n"
    if not source.startswith(prefix) or not source.endswith(suffix):
        raise ValueError("step-face-completeness source boundary missing or duplicate")
    body = source[len(prefix):-len(suffix)]
    if body.count("// Display mesh") != 1 or body.count("vs.analysisHandle.free();") != 1:
        raise ValueError("step-face-completeness source boundary missing or duplicate")
    return body


def browser_module(body):
    return f'''import init, * as cad from '/package.js';
function fail(actual, expected, operator, message) {{
  const format = value => Object.is(value, -0) ? '-0' :
    typeof value === 'number' && !Number.isFinite(value) ? String(value) : JSON.stringify(value);
  const error = new AssertionError(`${{message ?? 'assertion failed'}}\ncode: ERR_ASSERTION\nactual: ${{format(actual)}}\nexpected: ${{format(expected)}}\noperator: ${{operator}}`);
  Object.assign(error, {{name: 'AssertionError', code: 'ERR_ASSERTION', actual, expected, operator}});
  throw error;
}}
class AssertionError extends Error {{}}
function assert(value, message) {{ if (!value) fail(value, true, '==', message); }}
assert.equal = (actual, expected, message) => {{
  if (!Object.is(actual, expected)) fail(actual, expected, 'strictEqual', message);
}};
export async function run() {{
  let failures = 0;
  for (const check of [() => assert(false, 'truth negative control'),
                       () => assert.equal(0, -0, 'strict equal negative control')]) {{
    try {{ check(); }} catch (_) {{ failures++; }}
  }}
  if (failures !== 2) throw Error('assertion negative controls did not fail');
  assert.equal(NaN, NaN, 'strict equal NaN semantics');
  await init({{module_or_path: '/package_bg.wasm'}});
  const response = await fetch('/fixture.step');
  if (!response.ok) throw Error(`fixture read failed: ${{await response.text()}}`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  const digest = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))]
    .map(value => value.toString(16).padStart(2, '0')).join('');
  assert.equal(digest, response.headers.get('X-Fixture-SHA256'), 'fixture SHA-256 mismatch');
  const result = cad.read_step_model(bytes, {{sourceSha256: digest}});
  const console = {{log: (...args) => {{
    const request = new XMLHttpRequest();
    request.open('POST', '/output', false);
    request.send(args.join(' '));
    if (request.status !== 204) throw Error('cannot forward regression output');
  }}}};
{body}
  return [{{suite: '{CASE}', case: 'sliver', ok: true}}];
}}
'''


def run():
    package, wasm = preflight()
    source = (CAD / "wasm/tests/step-face-completeness.cjs").read_text()
    fixture = CAD / "monstertruck/tests/fixtures/circular-segment-sliver-prism.step"
    routes = {
        "/": ("text/html", b"<!doctype html><title>CAD face completeness</title>"),
        "/package.js": ("text/javascript", package.read_bytes()),
        "/package_bg.wasm": ("application/wasm", wasm.read_bytes()),
        "/case.js": ("text/javascript", browser_module(regression_body(source)).encode()),
    }

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == "/fixture.step":
                try:
                    data = fixture.read_bytes()
                    status, content_type = 200, "application/octet-stream"
                except OSError as error:
                    data = str(error).encode()
                    status, content_type = 500, "text/plain"
                self.send_response(status)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(data)))
                if status == 200:
                    self.send_header("X-Fixture-SHA256", hashlib.sha256(data).hexdigest())
                self.end_headers()
                self.wfile.write(data)
                return
            if self.path not in routes:
                self.send_error(404)
                return
            content_type, data = routes[self.path]
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            if self.path != "/output" or not 0 < length <= 8192:
                self.send_error(400)
                return
            print(self.rfile.read(length).decode("utf-8"), flush=True)
            self.send_response(204)
            self.end_headers()

        def log_message(self, *_args):
            pass

    binary = os.environ.get("CAD_FACE_WEB_DRIVER")
    if not binary:
        raise RuntimeError("set CAD_FACE_WEB_DRIVER to the built face-completeness-web driver")
    probe_path = ROOT / "tools/browser-harness/test-cad-target-responses.py"
    spec = importlib.util.spec_from_file_location("cad_response_runner", probe_path)
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever)
    thread.start()
    try:
        result = probe.run_probe([binary, f"http://127.0.0.1:{server.server_port}/"], timeout=600)
        if result.returncode:
            stdout = f"\nstdout: {result.stdout}" if result.stdout else ""
            raise RuntimeError(f"face completeness browser failed ({result.returncode}): {result.stderr}{stdout}")
        if result.stderr:
            sys.stderr.write(result.stderr)
        expected = [{"suite": CASE, "case": "sliver", "ok": True}]
        observed = json.loads(result.stdout)
        if observed != expected or observed[0]['ok'] is not True:
            raise RuntimeError(f"invalid face completeness result: {result.stdout!r}")
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    try:
        run()
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        sys.exit(str(error))
