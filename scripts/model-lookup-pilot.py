#!/usr/bin/env python3
"""Node-free browser check of scripts/model-lookup.js, a candidate to replace test-model-lookup.cjs.

The Node test stays the gate until the parent reviews and registers this one.

Driver contract: CAD_PROVIDER_PAGES_DRIVER is an absolute path to a cad-provider-pages binary
the operator built from this checkout (provenance is not verified; the success line prints its
SHA-256 for the caller to compare) and CHROME_BIN names Chrome. The driver disables Chrome's
sandbox: run inside an OS sandbox with only loopback networking and Node masked. The driver
requests /run.js, calls run(suite) and fails on any page error, failed case or invalid record;
this pilot serves an exact route allowlist and requires the complete named case inventory.
--source and --recipe let tests mutate temporary copies only.

Accepted adaptations from the original bare Node vm test: the helper runs as a classic Chrome
worker with all other globals deleted. Chrome retains the immutable constants TEMPORARY=0 and
PERSISTENT=1 on the worker prototype chain, so a literal reference to them is linted, but a
computed one is not detected. CSP blocks string code generation (stricter than the vm test).
A fetch (or other deleted global) in a branch the cases never execute is not detected, as in
the original. This is neither a true isolate nor a security sandbox.

Completion: the worker publishes results only after several timer and message turns, so a
promise rejection the helper leaves unhandled is reported before the results. Async work the
helper schedules beyond those turns is not observed.
"""
import argparse
import http.server
import importlib.util
import json
import hashlib
import os
from pathlib import Path
import sys
import threading
from urllib.parse import urlsplit

import browser_tools

ROOT = Path(__file__).resolve().parents[1]
SUITE = 'model-lookup-pilot'
TIMEOUT = 300  # the driver bounds navigation and the suite itself; this covers a hung process
CASES = (
    'helper source uses no Node globals',
    'helper loads as one function declaration in a restricted classic worker',
    'the file text is exactly the function source',
    'exact pair selects from the large catalog',
    'selection carries the exact provider/model IDs and label',
    'selection carries the target options only',
    'rendered selection omits unrelated sentinel data',
    'rendered selection is compact',
    'a model ID shared by another provider resolves inside the requested provider',
    'a matching label under another provider cannot change the selection',
    'a repeated exact pair reports duplicate-match',
    'an unknown provider reports provider-not-found',
    'an unknown model reports model-not-found',
    'the three diagnostics are distinct',
    'diagnostics carry only ok/code/message',
    'validated JSON text is a fallback when structured content is absent',
    'a response carrying both forms is handled once',
    'a non-text block before the JSON block is skipped',
    'malformed fallback JSON reports malformed-text-json',
    'unexpected response shapes report unexpected-shape',
    'malformed present structured content is rejected rather than rescued by fallback',
    'malformed selected options are not reported as a valid empty option',
    'malformed selected provider models are not reported as a missing model',
    'a valid empty options list stays a success',
    'absent model options stay a success',
    'empty options are distinguishable from an error',
    'missing provider/model IDs report missing-input',
    'missing-input stays distinct from unexpected-shape',
    'no representation or missing-input diagnostic leaks sentinel data',
    'the recipe probe literal matches the tested helper',
    'the documented behavior probe stays in sync with the tested literal',
    'boundary-controls',
)


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


probe = load_module('cad_response_probe_pilot', ROOT / 'tools/browser-harness/test-cad-target-responses.py')


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'duplicate result key: {key}')
        result[key] = value
    return result


def trusted_driver(driver):
    if not driver:
        raise RuntimeError('set CAD_PROVIDER_PAGES_DRIVER to the built cad-provider-pages driver')
    path = Path(driver)
    if not path.is_absolute() or not path.is_file() or not os.access(path, os.X_OK):
        raise RuntimeError(f'driver must be an absolute path to an executable file: {driver}')
    return path


def validate_results(text):
    result = json.loads(text, object_pairs_hook=unique_object)
    expected = {SUITE: [{'suite': SUITE, 'case': name, 'ok': True} for name in CASES]}
    if result != expected or any(type(entry['ok']) is not bool for entry in result[SUITE]):
        raise RuntimeError(f'invalid pilot inventory: {result!r}')


def run(source, driver, recipe=ROOT / 'docs/agents/model-lookup.md'):
    driver = trusted_driver(driver)
    routes = {
        '/': ('text/html', b'<!doctype html><title>Model lookup pilot</title>'),
        '/run.js': ('text/javascript', b"export { run } from '/pilot.mjs';"),
        '/pilot.mjs': ('text/javascript', (ROOT / 'scripts/model-lookup-pilot.mjs').read_bytes()),
        '/model-lookup-pilot-worker.js': ('text/javascript', (ROOT / 'scripts/model-lookup-pilot-worker.js').read_bytes()),
        '/model-lookup.js': ('text/javascript', Path(source).read_bytes()),
        '/model-lookup.md': ('text/plain', Path(recipe).read_bytes()),
    }

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            route = routes.get(urlsplit(self.path).path)
            if route is None:
                self.send_error(404)
                return
            content_type, data = route
            self.send_response(200)
            self.send_header('Content-Type', content_type)
            self.send_header('Content-Length', str(len(data)))
            self.send_header('Cache-Control', 'no-store')
            if urlsplit(self.path).path == '/model-lookup-pilot-worker.js':
                self.send_header('Content-Security-Policy', "default-src 'none'; script-src 'self'")
            self.end_headers()
            self.wfile.write(data)

        def log_message(self, *_args):
            pass

    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever)
    thread.start()
    try:
        result = probe.run_probe([str(driver), f'http://127.0.0.1:{server.server_port}/', SUITE], timeout=TIMEOUT)
        if result.stderr:
            sys.stderr.write(result.stderr)
        if result.returncode:
            raise RuntimeError(f'pilot driver failed ({result.returncode}): {result.stderr}\nstdout: {result.stdout}')
        validate_results(result.stdout)
        for entry in json.loads(result.stdout)[SUITE]:
            print(f"PASS {entry['case']}")
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def run_gate():
    driver = browser_tools._build_harness('cad-provider-pages')
    run(ROOT / 'scripts/model-lookup.js', driver)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / 'scripts/model-lookup.js')
    parser.add_argument('--recipe', type=Path, default=ROOT / 'docs/agents/model-lookup.md')
    parser.add_argument('--gate', action='store_true', help='build the checkout driver under the shared lock')
    args = parser.parse_args()
    try:
        if args.gate:
            run_gate()
        else:
            driver = os.environ.get('CAD_PROVIDER_PAGES_DRIVER')
            run(args.source, driver, args.recipe)
            print(f'model lookup pilot passed (restricted worker; not an isolate or security sandbox); driver sha256 {hashlib.sha256(Path(driver).read_bytes()).hexdigest()}')
    except (OSError, ValueError, RuntimeError, browser_tools.MissingBrowserTool) as error:
        sys.exit(str(error))
