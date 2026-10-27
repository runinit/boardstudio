#!/usr/bin/env python3
"""Compare real startup capture readiness on controlled landing pages (diagnostic only)."""
import csv
import ctypes
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time
from http.server import ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(sys.argv[1]).resolve()
OUT.mkdir(parents=True, exist_ok=False)
ctypes.CDLL(None).prctl(36, 1, 0, 0, 0)  # Reap browser descendants after the capture exits.
spec = importlib.util.spec_from_file_location('serve', ROOT / 'scripts/serve-web.py')
serve = importlib.util.module_from_spec(spec)
spec.loader.exec_module(serve)
scenario = ''
transitions = []

class Handler(serve.handler_for(ROOT / 'web/target/site')):
    def log_message(self, *args):
        pass

    def do_GET(self):
        path = self.path.partition('?')[0]
        if path == '/mark':
            transitions.append({'label': self.path.partition('?')[2], 'unix_ms': time.time_ns() / 1e6})
            self.send_response(204)
            self.end_headers()
            return
        if path.startswith('/__perf/'):
            file = (ROOT / path.removeprefix('/__perf/')).resolve()
            if not file.is_relative_to(ROOT) or not file.is_file():
                self.send_error(404)
                return
            data = file.read_bytes()
            content_type = 'text/html' if file.suffix == '.html' else 'application/json'
        elif path == '/boardstudio/':
            # Wrong name and hidden exact name precede the real visible state.
            schedule = "setTimeout(() => show('transient'), 150); setTimeout(() => hide(), 250);" if scenario == 'transient' else ''
            data = f'''<!doctype html><button>Create new keyboard extra</button>
<button id="target" style="display:none">Create new keyboard</button><script>
const target = document.querySelector('#target');
function mark(label) {{ fetch('/mark?' + label, {{keepalive:true}}); }}
function show(label) {{ target.style.display=''; mark(label); }}
function hide() {{ target.style.display='none'; mark('hidden'); }}
mark('loaded'); {schedule} setTimeout(() => show('stable'), 1200);
</script>'''.encode()
            content_type = 'text/html'
        else:
            return super().do_GET()
        self.send_response(200)
        self.send_header('Content-Type', content_type)
        self.send_header('Cache-Control', 'no-store')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
thread = threading.Thread(target=server.serve_forever)
thread.start()
seen = {}

def children():
    entries = {}
    for path in Path('/proc').iterdir():
        if not path.name.isdigit():
            continue
        try:
            stat = (path / 'stat').read_text().rsplit(')', 1)[1].split()
            entries[int(path.name)] = (int(stat[1]), stat[19])
        except (OSError, ValueError, IndexError):
            pass
    owned = {os.getpid()}
    while True:
        added = {pid for pid, (parent, _) in entries.items() if parent in owned} - owned
        if not added:
            break
        owned.update(added)
        seen.update({pid: entries[pid][1] for pid in added})
    return owned - {os.getpid()}

def reap():
    while True:
        try:
            if not os.waitpid(-1, os.WNOHANG)[0]:
                return
        except ChildProcessError:
            return

results = {}
try:
    for scenario in ('delayed', 'transient'):
        for driver in ('node', 'rust'):
            transitions.clear()
            directory = OUT / f'{scenario}-{driver}'
            directory.mkdir()
            command = (['node', 'cad/scripts/measure-cad-runtime.cjs'] if driver == 'node' else
                       ['tools/browser-harness/target/debug/cad-capture'])
            command += [f'http://127.0.0.1:{server.server_port}', 'monstertruck', 'fixture',
                        'startup:cold', str(directory), '1', 'validation']
            with (directory / 'process.log').open('w') as log:
                process = subprocess.Popen(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
                deadline = time.monotonic() + 90
                while process.poll() is None and time.monotonic() < deadline:
                    children()
                    time.sleep(.1)
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=5)
            events = [json.loads(line) for line in (directory / 'events.log').read_text().splitlines()]
            with (directory / 'diagnostic.csv').open() as metrics:
                landing = next(row for row in csv.DictReader(metrics) if row['metric'] == 'navigation-to-usable-landing')
            ready = next(event['unix_ms'] for event in events if event['name'] == 'landing-ready')
            assert process.returncode == 0, (scenario, driver, process.returncode)
            assert any(event['name'] == 'reply-retained' for event in events)
            assert transitions and transitions[0]['label'] == 'loaded'
            if scenario == 'delayed':
                assert any(mark['label'] == 'stable' for mark in transitions), 'hidden or wrong-name button was accepted'
            results[f'{scenario}-{driver}'] = {'metric_ms': float(landing['value']), 'landing_ready_unix_ms': ready,
                                               'transitions': list(transitions)}
            print(scenario, driver, json.dumps(results[f'{scenario}-{driver}']), flush=True)
finally:
    server.shutdown()
    server.server_close()
    thread.join()
    for sig in (signal.SIGTERM, signal.SIGKILL):
        reap()
        for pid in children():
            try:
                os.kill(pid, sig)
            except ProcessLookupError:
                pass
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            reap()
            if not children():
                break
            time.sleep(.1)
    reap()
    remaining = sorted(children())
    (OUT / 'receipt.json').write_text(json.dumps({'results': results, 'observed': seen, 'remaining': remaining}, indent=2))
    print('remaining children:', remaining, flush=True)
    if remaining:
        raise RuntimeError('owned processes remain')
