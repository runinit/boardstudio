#!/usr/bin/env python3
"""Browser host for the pinned Ergogen parser and cases.cjs; generate.py drives its payload modes.

A path driver is the caller's to trust (generate.py builds one from this checkout). Use a trusted temporary
parent and stable checkout/lock. Chrome runs without its own sandbox: use an
external OS sandbox. No publisher authentication or secure JS isolation is claimed.
"""
import argparse
from contextlib import nullcontext
import http.server
import importlib.util
import json
import math
from pathlib import Path
import sys
import threading
from urllib.parse import quote, unquote

import verified_runtime

ROOT = verified_runtime.ROOT
SUITE = 'ergogen-host-probe'
MODES = ('normal', 'missing', 'builtin', 'traversal', 'invalid', 'retry', 'cases', 'invalid-case', 'generate', 'source')
CASES = Path(__file__).with_name('cases.cjs')


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'duplicate result key: {key}')
        result[key] = value
    return result


def read_json(stdout):
    def reject_constant(value):
        raise ValueError(f'non-finite JSON constant: {value}')
    return json.loads(stdout, object_pairs_hook=unique, parse_constant=reject_constant)


def valid_points(points):
    if not isinstance(points, dict) or not points:
        raise ValueError('empty or invalid points')
    for name, point in points.items():
        if (not isinstance(name, str) or not name or not isinstance(point, dict)
                or set(point) != {'x', 'y', 'r'}
                or any(type(value) not in (int, float) or not math.isfinite(value) for value in point.values())):
            raise ValueError(f'invalid point: {name}')


def validate_reference(reference):
    if not isinstance(reference, list) or len(reference) != 42:
        raise ValueError('reference case count must be 42')
    ids = set()
    for entry in reference:
        if (not isinstance(entry, dict) or set(entry) != {'case', 'config', 'result'}
                or not isinstance(entry['case'], str) or not entry['case'] or entry['case'] in ids
                or not isinstance(entry['config'], dict)):
            raise ValueError('invalid reference case')
        ids.add(entry['case'])
        valid_points(entry['result'])
    if sum(len(entry['result']) for entry in reference) != 360:
        raise ValueError('reference point count must be 360')


def validate_output(stdout, reference=None):
    output = read_json(stdout)
    entries = output.get(SUITE) if isinstance(output, dict) and set(output) == {SUITE} else None
    if reference is not None:
        validate_reference(reference)
        if not isinstance(entries, list) or len(entries) != len(reference):
            raise ValueError('browser case count mismatch')
        for entry, expected in zip(entries, reference):
            if (not isinstance(entry, dict) or set(entry) != {'suite', 'case', 'ok', 'result'}
                    or entry['suite'] != SUITE or entry['case'] != expected['case'] or entry['ok'] is not True):
                raise ValueError('invalid browser case')
            valid_points(entry['result'])
            if entry['result'] != expected['result']:
                raise ValueError(f"browser points differ: {expected['case']}")
        return [entry['result'] for entry in entries]
    if (not isinstance(entries, list) or len(entries) != 1 or not isinstance(entries[0], dict)
            or set(entries[0]) != {'suite', 'case', 'ok', 'result'}
            or entries[0]['suite'] != SUITE or entries[0]['case'] != 'pinned-parser'
            or entries[0]['ok'] is not True or not isinstance(entries[0]['result'], dict)):
        raise ValueError('invalid browser result')
    result = entries[0]['result']
    if set(result) != {'m_a_r0'} or not isinstance(result['m_a_r0'], dict):
        raise ValueError('invalid parser points')
    point = result['m_a_r0']
    if (set(point) != {'x', 'y', 'r'} or any(type(value) not in (int, float) or not math.isfinite(value) for value in point.values())
            or point != {'x': 10, 'y': 20, 'r': 30}):
        raise ValueError('invalid parser point')
    return result


def validate_payload(stdout, mode):
    """Evaluated cases.cjs (and upstream points for generate) from one JSON.stringify string."""
    output = read_json(stdout)
    entries = output.get(SUITE) if isinstance(output, dict) and set(output) == {SUITE} else None
    if (not isinstance(entries, list) or len(entries) != 1 or not isinstance(entries[0], dict)
            or set(entries[0]) != {'suite', 'case', 'ok', 'json'} or entries[0]['suite'] != SUITE
            or entries[0]['case'] != mode or entries[0]['ok'] is not True or not isinstance(entries[0]['json'], str)):
        raise ValueError('invalid browser result')
    payload = read_json(entries[0]['json'])
    keys = {'cases', 'boardStudioOnly', *(('actual',) if mode == 'generate' else ())}
    if (not isinstance(payload, dict) or set(payload) != keys or not isinstance(payload['cases'], list)
            or not all(isinstance(item, dict) for item in payload['cases'])
            or not isinstance(payload['boardStudioOnly'], list)):
        raise ValueError('invalid browser payload')
    return payload


def serve(prepared, mode, reference=None):
    lock = json.loads(verified_runtime.LOCK.read_text())
    packages = [name.removeprefix('node_modules/') for name in lock['packages'] if name] if prepared else []
    roots = [prepared / 'upstream', *(prepared / 'node_modules' / name for name in packages)] if prepared else []
    files = {}
    for root in roots:
        for file in root.rglob('*'):
            if file.is_file() and file.suffix in ('.js', '.json'):
                path = file.relative_to(prepared).as_posix()
                files['/files/' + quote(path, safe='')] = file
    if mode in ('generate', 'source'):
        files['/files/' + quote('boardstudio/cases.cjs', safe='')] = CASES
    manifest = json.dumps({'files': [unquote(path.removeprefix('/files/')) for path in files],
                           'packages': packages, 'mode': mode,
                           'cases': [{'case': item['case'], 'config': item['config']} for item in reference or []]},
                          allow_nan=False).encode()
    routes = {'/': ('text/html', b'<!doctype html><title>Ergogen host proof</title>'),
              '/run.js': ('text/javascript', Path(__file__).with_suffix('.mjs').read_bytes()),
              '/manifest': ('application/json', manifest)}

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            route = self.path.split('?', 1)[0] if self.path.startswith('/?suite=') else self.path
            if route in files:
                try:
                    content_type, data = 'text/plain', files[route].read_bytes()
                except OSError:
                    self.send_error(500)
                    return
            elif route in routes:
                content_type, data = routes[route]
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header('Content-Type', content_type)
            self.send_header('Content-Length', str(len(data)))
            self.send_header('Cache-Control', 'no-store')
            self.end_headers()
            self.wfile.write(data)

        def log_message(self, *_args):
            pass

    return http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)


def run(checkout, npm_cache, driver, mode='normal', reference=None):
    if mode not in MODES:
        raise ValueError(f'unknown mode: {mode}')
    if mode in ('cases', 'invalid-case'):
        validate_reference(reference)
    elif reference is not None:
        raise ValueError('reference only applies to case modes')
    spec = importlib.util.spec_from_file_location('probe', ROOT / 'tools/browser-harness/test-cad-target-responses.py')
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    # Source mode evaluates only repository-authored cases.cjs; upstream modes need a verified tree.
    with (nullcontext() if mode == 'source' else verified_runtime.verified_runtime(checkout, npm_cache)) as prepared:
        # A callable driver is built only after the checkout and archives verified.
        binary = driver() if callable(driver) else driver
        server = serve(prepared, mode, reference) if reference is not None else serve(prepared, mode)
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        try:
            result = probe.run_probe([str(binary), f'http://127.0.0.1:{server.server_port}/', SUITE], timeout=360)
            if result.stderr:
                sys.stderr.write(result.stderr)
            if result.returncode:
                raise RuntimeError(f'browser failed ({result.returncode}): {result.stderr}')
            if mode in ('generate', 'source'):
                return validate_payload(result.stdout, mode)
            return validate_output(result.stdout, reference)
        finally:
            server.shutdown()
            server.server_close()
            thread.join()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('checkout', type=Path)
    parser.add_argument('--npm-cache', type=Path, default=Path.home() / '.npm/_cacache/content-v2/sha512')
    parser.add_argument('--driver', type=Path, required=True)
    parser.add_argument('--mode', choices=MODES, default='normal')
    parser.add_argument('--reference', type=Path, help='trusted separately generated Node reference JSON')
    args = parser.parse_args()
    try:
        reference = read_json(args.reference.read_text()) if args.reference else None
        print(json.dumps(run(args.checkout, args.npm_cache, args.driver, args.mode, reference), allow_nan=False))
    except (OSError, ValueError, RuntimeError) as error:
        sys.exit(str(error))
