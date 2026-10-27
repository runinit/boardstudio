#!/usr/bin/env python3
"""Exercise Rust CAD capture sequencing against a controlled browser worker (not a benchmark)."""
import argparse
import csv
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import time
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
  let result;
  if (frame.operation === 'export-step') result = {revision:frame.identity.revision, step:new Uint8Array([1,2,3])};
  else if (frame.operation === 'read-step') {
    const count = FAULT === 'vendor' ? 154 : 155;
    result = {mesh:mesh('vendor'), bounds:{}, vendorStep:{sourceSha256:frame.sourceSha256,
      sourceFaces:Array.from({length:count}, (_, i) => ({faceId:i})),
      faceOccurrences:Array.from({length:count}, (_, i) => ({sourceFaceIndex:i})),
      triangleFaceOccurrenceIndices:Uint32Array.from({length:count}, (_, i) => i),
      surfacePositionsMmF64:new Float64Array(count*9), lineSegmentsMmF64:new Float64Array(),
      segmentLineOccurrenceIndices:new Uint32Array(), lineOccurrences:[]}};
  } else result = {revision:frame.identity.revision, bodies:frame.prepared.bodies.map(body => mesh(body.body.id))};
  if (FAULT === true && result.bodies) result.bodies[0].positions[0] = NaN;
  self.postMessage({kind:'reply', frame:JSON.stringify({...frame, outcome:'completed'}), result});
};
self.postMessage({kind:'ready'});
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?", help="new output directory for capture logs and controlled observations")
    args = parser.parse_args()
    if args.output:
        output = args.output.resolve()
        output.mkdir(parents=True, exist_ok=False)
    else:
        parent = ROOT / "web/target/test-results/cad-capture-controls"
        parent.mkdir(parents=True, exist_ok=True)
        output = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    print(f"CAD capture control output: {output}", flush=True)
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
            if self.path == "/boardstudio/":
                data = (b"<!doctype html><script>fetch('/boardstudio/assets/core-worker/missing.wasm')"
                        b".then(() => document.body.innerHTML='<button>Create new keyboard</button>')</script><body></body>")
                content_type = "text/html"
            elif self.path == "/__perf/cad/scripts/performance-host.html":
                data, content_type = b"<!doctype html><title>Controlled CAD capture</title>", "text/html"
            elif self.path == "/__perf/cad/test/fixtures/performance/C1plate.json":
                data = (ROOT / "cad/test/fixtures/performance/C1plate.json").read_bytes()
                content_type = "application/json"
            elif self.path == "/__perf/cad/test/fixtures/followup-manifest.json":
                data = (ROOT / "cad/test/fixtures/followup-manifest.json").read_bytes()
                content_type = "application/json"
            elif self.path == "/__source/ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step":
                data = (ROOT / self.path.removeprefix("/__source/")).read_bytes()
                content_type = "application/octet-stream"
            elif self.path == "/boardstudio/assets/cad-worker/entry.js":
                data, content_type = WORKER.replace("FAULT", json.dumps(bad)).encode(), "text/javascript"
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
        for name, fixture, operation, sequence, bad in (
                ("preview-validation", "C1plate", "preview", "validation", False),
                ("preview-latency", "C1plate", "preview", "latency", False),
                ("step-latency", "C1plate", "step", "latency", False),
                ("vendor-latency", "V1switch", "import", "latency", False),
                ("invalid-vendor", "V1switch", "import", "validation", "vendor"),
                ("invalid-mesh", "C1plate", "preview", "validation", True)):
            records.clear()
            directory = output / name
            directory.mkdir()
            command = [str(binary), f"http://127.0.0.1:{server.server_port}", "monstertruck", "checked",
                       f"{fixture}:{operation}", str(directory), "1", sequence]
            result = subprocess.run(command, cwd=ROOT, env=dict(os.environ, CHROME_BIN=chrome),
                                    capture_output=True, text=True, timeout=240)
            (directory / "process.log").write_text(f"exit={result.returncode}\n{result.stdout}\n{result.stderr}")
            (directory / "worker-observations.json").write_text(json.dumps(records, indent=2))
            events = [json.loads(line) for line in (directory / "events.log").read_text().splitlines()]
            final = next(event for event in events if event["name"] == "result")
            if bad:
                assert result.returncode != 0 and final["failed"], name
                expected = "Error: incomplete vendor source/occurrence coverage" if bad == "vendor" else "Error: invalid mesh"
                assert any(event["name"] == "failure" and expected in event["error"] for event in events), name
                assert not any(event["name"] == "reply-retained" for event in events), name
                assert not any(message in result.stderr for message in
                               ("browser shutdown failed", "browser termination unconfirmed", "cannot remove browser profile")), result.stderr
                print(f"PASS {name}: invalid worker result fails capture", flush=True)
                continue
            assert result.returncode == 0 and not final["failed"], (name, result.stderr)
            count = 1 if sequence == "validation" else 6
            requests = [record for record in records if record["event"] == "request"]
            ready = [record for record in records if record["event"] == "ready"]
            assert len(requests) == count and len({r["requestId"] for r in requests}) == count, (name, records)
            expected_op = "export-step" if operation == "step" else "read-step" if operation == "import" else "preview"
            assert all(r["operation"] == expected_op for r in requests), records
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

        bad = False
        directory = output / "observed-core-failure"
        directory.mkdir()
        command = [str(binary), f"http://127.0.0.1:{server.server_port}", "monstertruck", "checked",
                   "startup:cold", str(directory), "1", "validation"]
        result = subprocess.run(command, cwd=ROOT, env=dict(os.environ, CHROME_BIN=chrome),
                                capture_output=True, text=True, timeout=240)
        (directory / "process.log").write_text(f"exit={result.returncode}\n{result.stdout}\n{result.stderr}")
        events = [json.loads(line) for line in (directory / "events.log").read_text().splitlines()]
        resources = next(event["responses"] for event in events if event["name"] == "resources")
        assert any(r["url"].endswith("/core-worker/missing.wasm") and r["status"] == 404 for r in resources), resources
        assert result.returncode != 0 and next(event for event in events if event["name"] == "result")["failed"], events
        assert any(event["name"] == "failure" and "observed core worker response failed" in event["error"] for event in events), events
        print("PASS observed-core-failure: recorded 404 fails capture", flush=True)

        # Interrupt only this capture's verified Chrome after a successful reply, before shutdown.
        bad = False
        records.clear()
        directory = output / "browser-close-failure"
        directory.mkdir()
        profiles = Path(tempfile.mkdtemp(prefix="cad-close-", dir=os.environ.get("TMPDIR")))
        command = [str(binary), f"http://127.0.0.1:{server.server_port}", "monstertruck", "checked",
                   "C1plate:preview", str(directory), "1", "validation"]
        with (directory / "process.log").open("w") as log:
            process = subprocess.Popen(command, cwd=ROOT, env=dict(os.environ, CHROME_BIN=chrome, TMPDIR=str(profiles)),
                                       stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                deadline = time.monotonic() + 60
                events_file = directory / "events.log"
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        raise RuntimeError("capture exited before the browser close control")
                    if events_file.exists() and '"name":"reply-retained"' in events_file.read_text():
                        break
                    time.sleep(.01)
                else:
                    raise RuntimeError("capture did not retain a reply before the browser close control")
                runs = list(profiles.glob("bs-drag-*"))
                assert len(runs) == 1, runs
                profile = f"--user-data-dir={runs[0] / 'profile'}".encode()
                tasks = Path(f"/proc/{process.pid}/task")
                children = {pid for task in tasks.iterdir() for pid in (task / "children").read_text().split()}
                matches = [int(pid) for pid in children
                           if profile in Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ").split()]
                assert len(matches) == 1, matches
                fd = os.pidfd_open(matches[0])
                try:
                    assert profile in Path(f"/proc/{matches[0]}/cmdline").read_bytes().replace(b"\0", b" ").split()
                    signal.pidfd_send_signal(fd, signal.SIGKILL)
                finally:
                    os.close(fd)
                process.wait(timeout=90)
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
        events = [json.loads(line) for line in events_file.read_text().splitlines()]
        assert process.returncode != 0, "browser close failure must fail the capture"
        assert any(event["name"] == "reply-retained" for event in events), events
        assert next(event for event in events if event["name"] == "result")["failed"], events
        assert any(event["name"] == "failure" and "browser shutdown failed" in event["error"] for event in events), events
        if not any(profiles.iterdir()):
            profiles.rmdir()
        else:
            print(f"browser profile retained at {profiles}", flush=True)
        print("PASS browser-close-failure: retained reply cannot hide browser shutdown failure", flush=True)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
