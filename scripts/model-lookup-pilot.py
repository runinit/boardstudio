#!/usr/bin/env python3
"""Node-free browser check of scripts/model-lookup.js in a restricted classic worker.

The page serves an exact route allowlist and requires the complete named case inventory.
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
from pathlib import Path
import sys
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


def validate_results(result):
    expected = {SUITE: [{'suite': SUITE, 'case': name, 'ok': True} for name in CASES]}
    if result != expected or any(type(entry['ok']) is not bool for entry in result[SUITE]):
        raise RuntimeError(f'invalid pilot inventory: {result!r}')


def run(source, recipe=ROOT / 'docs/agents/model-lookup.md'):
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

    with browser_tools.serve(Handler) as url:
        results = browser_tools.run_provider_page(url + '/', SUITE, timeout=TIMEOUT)
    validate_results(results)
    for entry in results[SUITE]:
        print(f"PASS {entry['case']}")


def run_gate():
    run(ROOT / 'scripts/model-lookup.js')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / 'scripts/model-lookup.js')
    parser.add_argument('--recipe', type=Path, default=ROOT / 'docs/agents/model-lookup.md')
    parser.add_argument('--gate', action='store_true', help='run against the checkout source and recipe')
    args = parser.parse_args()
    try:
        if args.gate:
            run_gate()
        else:
            run(args.source, args.recipe)
    except (OSError, ValueError, RuntimeError) as error:
        sys.exit(str(error))
