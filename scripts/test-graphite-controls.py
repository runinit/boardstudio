#!/usr/bin/env python3
"""Exercise Graphite assertion and page-error failures against a built site (diagnostic only)."""
import argparse
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import threading
from http.server import ThreadingHTTPServer

import browser_tools

ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "web/target/site"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="new output directory for failure logs")
    args = parser.parse_args()
    index = SITE / "site-subpath/boardstudio/index.html"
    if not index.is_file():
        parser.error("build the site first with `python3 scripts/check.py build`")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    spec = importlib.util.spec_from_file_location("serve", ROOT / "scripts/serve-web.py")
    serve = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(serve)

    class Handler(serve.handler_for(SITE)):
        def log_message(self, *_args):
            pass

        def do_GET(self):
            path, _, query = self.path.partition("?")
            if path == "/boardstudio/" and query in ("assertion", "page-error"):
                injection = ("const e=document.createElement('div'); e.className='m1-canvas-measurements'; document.body.append(e);"
                             if query == "assertion" else "setTimeout(() => { throw Error('graphite control page error'); }, 0);")
                script = f"<script>document.addEventListener('DOMContentLoaded', () => {{ {injection} }});</script>".encode()
                original = index.read_bytes()
                if original.count(b"</head>") != 1:
                    raise RuntimeError("site index does not have one closing head")
                data = original.replace(b"</head>", script + b"</head>")
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            else:
                super().do_GET()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        for case, launcher, expected in (
                ("assertion", "run-graphite-feedback.py", "FAIL canvas numeric card removed"),
                ("page-error", "run-graphite-usability.py", "graphite control page error")):
            env = os.environ.copy()
            for key in ("NODE_PATH", "BOARDSTUDIO_GRAPHITE_BUILD_RUN", "BOARDSTUDIO_GRAPHITE_IDENTITY", "BOARDSTUDIO_SCREENSHOTS"):
                env.pop(key, None)
            env["BOARDSTUDIO_PREVIEW_URL"] = f"http://127.0.0.1:{server.server_port}/boardstudio/?{case}"
            lock = browser_tools._inherited_build_lock()
            result = subprocess.run([sys.executable, str(ROOT / "scripts" / launcher)], cwd=ROOT, env=env,
                                    capture_output=True, text=True, timeout=300,
                                    **({"pass_fds": (lock,)} if lock is not None else {}))
            log = result.stdout + result.stderr
            (output / f"{case}.log").write_text(f"exit={result.returncode}\n{log}")
            if result.returncode == 0 or expected not in log:
                raise SystemExit(f"{case} negative control failed; see {output / (case + '.log')}")
            print(f"PASS {case}: {launcher} rejected injected failure", flush=True)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
