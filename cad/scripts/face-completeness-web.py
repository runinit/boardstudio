#!/usr/bin/env python3
"""Opt-in real packaged WEB step-face-completeness regression; no build is performed."""
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
    start = "// Display mesh"
    end = "console.log('PASS step-face-completeness:"
    release = "vs.analysisHandle.free();"
    if any(source.count(marker) != 1 for marker in (start, end, release)):
        raise ValueError("step-face-completeness source boundary missing or duplicate")
    if not source.index(start) < source.index(end) < source.index(release):
        raise ValueError("step-face-completeness source boundary out of order")
    return source[source.index(start):source.index(end)] + release + "\n"


def browser_module(body, digest):
    return f'''import init, * as cad from '/package.js';
function assert(value, message) {{ if (!value) throw Error(message || 'assertion failed'); }}
assert.equal = (actual, expected, message) => {{
  if (!Object.is(actual, expected)) throw Error(message || `strict equal failed: ${{actual}} !== ${{expected}}`);
}};
export async function run() {{
  let failures = 0;
  for (const check of [() => assert(false, 'truth negative control'),
                       () => assert.equal(0, -0, 'strict equal negative control')]) {{
    try {{ check(); }} catch (_) {{ failures++; }}
  }}
  if (failures !== 2) throw Error('assertion negative controls did not fail');
  assert.equal(NaN, NaN, 'strict equal NaN semantics');
  const response = await fetch('/fixture.step');
  if (!response.ok) throw Error(`fixture HTTP ${{response.status}}`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  const digest = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))]
    .map(value => value.toString(16).padStart(2, '0')).join('');
  assert.equal(digest, {json.dumps(digest)}, 'fixture SHA-256 mismatch');
  await init({{module_or_path: '/package_bg.wasm'}});
  const result = cad.read_step_model(bytes, {{sourceSha256: digest}});
{body}
  return [{{suite: '{CASE}', case: 'sliver', ok: true}}];
}}
'''


def run():
    package, wasm = preflight()
    source = (CAD / "wasm/tests/step-face-completeness.cjs").read_text()
    fixture = (CAD / "monstertruck/tests/fixtures/circular-segment-sliver-prism.step").read_bytes()
    routes = {
        "/": ("text/html", b"<!doctype html><title>CAD face completeness</title>"),
        "/package.js": ("text/javascript", package.read_bytes()),
        "/package_bg.wasm": ("application/wasm", wasm.read_bytes()),
        "/fixture.step": ("application/octet-stream", fixture),
        "/case.js": ("text/javascript", browser_module(regression_body(source), hashlib.sha256(fixture).hexdigest()).encode()),
    }

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path not in routes:
                self.send_error(404)
                return
            content_type, data = routes[self.path]
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

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
            raise RuntimeError(f"face completeness browser failed ({result.returncode}): {result.stderr.strip()}")
        expected = [{"suite": CASE, "case": "sliver", "ok": True}]
        if json.loads(result.stdout) != expected:
            raise RuntimeError(f"invalid face completeness result: {result.stdout!r}")
        print(f"PASS {CASE}: sliver fixture, actual packaged WEB provider")
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    try:
        run()
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        sys.exit(str(error))
