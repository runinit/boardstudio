#!/usr/bin/env python3
"""Exercise Rust CAD capture sequencing against a controlled browser worker (not a benchmark)."""
import argparse
import csv
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import browser_tools  # noqa: E402

WORKER = r'''
const id = crypto.randomUUID();
const record = data => fetch('/record', {method:'POST', body:JSON.stringify({id, ...data})});
await record({event:'ready'});
self.onmessage = async ({data}) => {
  const frame = JSON.parse(data.frame);
  await record({event:'request', requestId:frame.requestId, operation:frame.operation});
  const mesh = id => ({id, positions:new Float32Array([0,0,0,1,0,0,0,1,0]),
    normals:new Float32Array([0,0,1,0,0,1,0,0,1])});
  const result = frame.operation === 'export-step' ? {revision:frame.identity.revision, step:new Uint8Array([1,2,3])} :
    {revision:frame.identity.revision, bodies:frame.prepared.bodies.map(body => mesh(body.body.id))};
  if (BAD && result.bodies) result.bodies[0].positions[0] = NaN;
  self.postMessage({kind:'reply', frame:JSON.stringify({...frame, outcome:'completed'}), result});
};
self.postMessage({kind:'ready'});
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="new output directory for capture logs and controlled observations")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    chrome = browser_tools.locate_chrome()
    binary = browser_tools._build_harness("cad-capture")
    records = []
    bad = False

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            if self.path != "/record":
                self.send_error(404)
                return
            records.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            self.send_response(204)
            self.end_headers()

        def do_GET(self):
            if self.path == "/__perf/cad/scripts/performance-host.html":
                data, content_type = b"<!doctype html><title>Controlled CAD capture</title>", "text/html"
            elif self.path == "/__perf/cad/test/fixtures/performance/C1plate.json":
                data = (ROOT / "cad/test/fixtures/performance/C1plate.json").read_bytes()
                content_type = "application/json"
            elif self.path == "/boardstudio/assets/cad-worker/entry.js":
                data, content_type = WORKER.replace("BAD", "true" if bad else "false").encode(), "text/javascript"
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(data)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        for name, operation, sequence, bad in (
                ("preview-validation", "preview", "validation", False),
                ("preview-latency", "preview", "latency", False),
                ("step-latency", "step", "latency", False),
                ("invalid-mesh", "preview", "validation", True)):
            records.clear()
            directory = output / name
            directory.mkdir()
            command = [str(binary), f"http://127.0.0.1:{server.server_port}", "monstertruck", "checked",
                       f"C1plate:{operation}", str(directory), "1", sequence]
            result = subprocess.run(command, cwd=ROOT, env=dict(os.environ, CHROME_BIN=chrome),
                                    capture_output=True, text=True, timeout=240)
            (directory / "process.log").write_text(f"exit={result.returncode}\n{result.stdout}\n{result.stderr}")
            (directory / "worker-observations.json").write_text(json.dumps(records, indent=2))
            events = [json.loads(line) for line in (directory / "events.log").read_text().splitlines()]
            final = next(event for event in events if event["name"] == "result")
            if bad:
                assert result.returncode != 0 and final["failed"], name
                assert any(event["name"] == "failure" and "Error: invalid mesh" in event["error"] for event in events), name
                assert not any(event["name"] == "reply-retained" for event in events), name
                print(f"PASS {name}: invalid worker result fails capture", flush=True)
                continue
            assert result.returncode == 0 and not final["failed"], (name, result.stderr)
            count = 1 if sequence == "validation" else 6
            requests = [record for record in records if record["event"] == "request"]
            ready = [record for record in records if record["event"] == "ready"]
            assert len(requests) == count and len({r["requestId"] for r in requests}) == count, (name, records)
            assert all(r["operation"] == ("export-step" if operation == "step" else "preview") for r in requests), records
            assert len(ready) == (count if operation == "step" else 1), (name, records)
            assert len({r["id"] for r in requests}) == (count if operation == "step" else 1), (name, records)
            assert [r["id"] for r in requests] == ([r["id"] for r in ready] if operation == "step"
                                                    else [ready[0]["id"]] * count), (name, records)
            assert [e["ordinal"] for e in events if e["name"] == "reply-retained"] == list(range(count)), name
            assert [e["ordinal"] for e in events if e["name"] == "fresh-export-worker-ready"] == (
                list(range(1, count)) if operation == "step" else []), name
            with (directory / ("diagnostic.csv" if sequence == "validation" else "runtime.csv")).open() as source:
                metrics = [row for row in csv.DictReader(source) if row["metric"] == "dispatch-to-validated-reply"]
            assert len(metrics) == count, (name, metrics)
            assert [row["state"] for row in metrics] == ["cold"] + ["warm"] * (count - 1), metrics
            assert all(row["status"] == ("diagnostic-not-benchmark" if sequence == "validation" else "observed")
                       and math.isfinite(float(row["value"])) and float(row["value"]) >= 0 for row in metrics), metrics
            print(f"PASS {name}: {count} operations and {len(ready)} worker(s)", flush=True)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
