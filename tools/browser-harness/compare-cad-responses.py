#!/usr/bin/env python3
"""Compare Rust C1 capture and probe on the existing packaged site; not a build receipt."""
import collections
import http.server
import importlib.util
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import browser_tools  # noqa: E402

spec = importlib.util.spec_from_file_location("serve_web", ROOT / "scripts/serve-web.py")
serve = importlib.util.module_from_spec(spec)
spec.loader.exec_module(serve)


class Handler(serve.handler_for(ROOT / "web/target/site")):
    def translate_path(self, path):
        path = path.split("?")[0]
        for prefix in ("/__perf/", "/__source/"):
            if path.startswith(prefix):
                file = (ROOT / path.removeprefix(prefix)).resolve()
                return str(file) if file.is_relative_to(ROOT) else "/dev/null"
        return super().translate_path(path)

    def log_message(self, *_args):
        pass


def main():
    chrome = browser_tools.locate_chrome()
    capture = browser_tools._build_harness("cad-capture")
    probe = browser_tools._build_harness("cad-response-probe")
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    origin = f"http://127.0.0.1:{server.server_port}"
    try:
        with tempfile.TemporaryDirectory() as output:
            capture_run = subprocess.run([str(capture), origin, "monstertruck", "checked", "C1plate:preview", output, "1", "validation"],
                                         cwd=ROOT, env=dict(os.environ, CHROME_BIN=chrome), capture_output=True, text=True, timeout=180)
            rust = subprocess.run([str(probe),
                                   origin + "/__perf/cad/scripts/performance-host.html", "C1plate:preview"],
                                  cwd=ROOT, env=dict(os.environ, CHROME_BIN=chrome), capture_output=True, text=True, timeout=180)
            print("capture exit", capture_run.returncode, "stderr", capture_run.stderr, "probe exit", rust.returncode, "stderr", rust.stderr)
            assert capture_run.returncode == 0 and rust.returncode == 0
            events = [json.loads(line) for line in pathlib.Path(output, "events.log").read_text().splitlines()]
            assert not next(event for event in events if event["name"] == "result")["failed"]
            capture_responses = next(event["responses"] for event in events if event["name"] == "resources")
            result = json.loads(rust.stdout)

            def counts(responses):
                return collections.Counter((r["url"].removeprefix(origin), r["status"], r["serviceWorker"]) for r in responses)

            old, new = counts(capture_responses), counts(result["responses"])
            required = ("/boardstudio/assets/cad-worker/entry.js", "/boardstudio/assets/cad-worker/m1_cad_worker.js",
                        "/boardstudio/assets/cad-worker/m1_cad_worker_bg.wasm", "/boardstudio/assets/cad/boardstudio_cad_wasm.js",
                        "/boardstudio/assets/cad/boardstudio_cad_wasm_bg.wasm")
            assert all(old[(url, 200, False)] for url in required), old
            assert all(new[(url, 200, False)] for url in required), new
            print("capture responses", sorted(old.items()))
            print("probe responses", sorted(new.items()))
            print("missing", list((old - new).elements()), "extra", list((new - old).elements()))
            sample = result["operation"]["sample"]
            capture_sample = next(event for event in events if event["name"] == "reply-retained")
            print("sample", {key: (capture_sample[key], sample[key]) for key in ("triangles", "bufferBytes", "stepBytes")})
            assert all(capture_sample[key] == sample[key] for key in ("triangles", "bufferBytes", "stepBytes"))
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
