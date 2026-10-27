#!/usr/bin/env python3
"""Exercise actual worker responses and fatal page/worker failures in Chrome."""
import http.server
import json
import os
import subprocess
import threading


class Handler(http.server.BaseHTTPRequestHandler):
    mode = "responses"

    def do_GET(self):
        routes = {
            "/host": (200, "text/html", "<script>throw Error('controlled page exception')</script>" if self.mode == "page-error" else ""),
            "/boardstudio/assets/cad-worker/entry.js": (200, "text/javascript", "await fetch('/redirect'); await fetch('/missing'); postMessage({kind:'ready'});"),
            "/created": (201, "text/plain", "created"),
            "/redirect": (302, "text/plain", "redirect"),
        }
        status, content_type, body = routes.get(self.path, (404, "text/plain", "missing"))
        if self.mode == "missing-worker" and self.path.endswith("entry.js"):
            status, content_type, body = 404, "text/plain", "missing"
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        if self.path == "/redirect":
            self.send_header("Location", "/created")
        self.end_headers()
        self.wfile.write(body.encode())

    def log_message(self, *_args):
        pass


def main():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    origin = f"http://127.0.0.1:{server.server_port}"
    binary = os.environ.get("CAD_RESPONSE_PROBE", "tools/browser-harness/target/debug/cad-response-probe")
    try:
        for mode in ("responses", "page-error", "missing-worker"):
            Handler.mode = mode
            result = subprocess.run([binary, origin + "/host"], capture_output=True, text=True, timeout=180)
            if mode == "responses":
                assert result.returncode == 0, result.stderr
                output = json.loads(result.stdout)
                responses = {(r["url"].removeprefix(origin), r["status"], r["serviceWorker"]) for r in output["responses"]}
                assert {("/host", 200, False), ("/boardstudio/assets/cad-worker/entry.js", 200, False), ("/redirect", 302, False), ("/created", 201, False), ("/missing", 404, False)} <= responses, responses
                print("real worker response matrix: passed")
            else:
                assert result.returncode != 0 and not result.stdout, (mode, result.returncode, result.stdout, result.stderr)
                assert ("controlled page exception" if mode == "page-error" else "worker") in result.stderr, result.stderr
                print(f"{mode} fails closed: passed")
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
