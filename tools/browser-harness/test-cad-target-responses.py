#!/usr/bin/env python3
"""Exercise actual worker responses and fatal page/worker failures in Chrome."""
import contextlib
import http.server
import json
import os
import selectors
import signal
import subprocess
import threading
import time


class Handler(http.server.BaseHTTPRequestHandler):
    mode = "responses"

    def do_GET(self):
        routes = {
            "/host": (200, "text/html", "<script>throw Error('controlled page exception')</script>" if self.mode == "page-error" else "<link rel=icon href=/favicon.ico>"),
            "/boardstudio/assets/cad-worker/entry.js": (302, "text/javascript", "redirect"),
            "/worker-final.js": (200, "text/javascript", "await fetch('/redirect'); await fetch('/missing'); postMessage({kind:'ready'});"),
            "/created": (201, "text/plain", "created"),
            "/redirect": (302, "text/plain", "redirect"),
            "/favicon.ico": (302, "text/plain", "redirect"),
        }
        status, content_type, body = routes.get(self.path, (404, "text/plain", "missing"))
        if self.mode == "missing-worker" and self.path.endswith("entry.js"):
            status, content_type, body = 404, "text/plain", "missing"
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        if self.path in ("/redirect", "/favicon.ico", "/boardstudio/assets/cad-worker/entry.js") and status == 302:
            self.send_header("Location", "/worker-final.js" if self.path.endswith("entry.js") else "/created")
        self.end_headers()
        self.wfile.write(body.encode())

    def log_message(self, *_args):
        pass


def run_probe(command, timeout):
    def terminate(signum, _frame):
        raise SystemExit(128 + signum)

    previous = signal.signal(signal.SIGTERM, terminate)
    # Keep the exited leader unreaped until its group is killed, preventing PID reuse.
    try:
        with contextlib.ExitStack() as stack:
            selector = stack.enter_context(selectors.DefaultSelector())
            process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
            stack.enter_context(process.stdout)
            stack.enter_context(process.stderr)
            output = {process.stdout: bytearray(), process.stderr: bytearray()}
            for stream in output:
                selector.register(stream, selectors.EVENT_READ)
            deadline = time.monotonic() + timeout
            try:
                while not os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT):
                    if time.monotonic() >= deadline:
                        raise subprocess.TimeoutExpired(command, timeout)
                    for key, _ in selector.select(min(0.05, max(0, deadline - time.monotonic()))):
                        chunk = os.read(key.fd, 65536)
                        if not chunk:
                            selector.unregister(key.fileobj)
                        else:
                            output[key.fileobj].extend(chunk)
                            if len(output[key.fileobj]) > 4 * 1024 * 1024:
                                raise ValueError('driver output limit exceeded')
            finally:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
            for stream in output:
                os.set_blocking(stream.fileno(), False)
                while True:
                    try:
                        chunk = os.read(stream.fileno(), 65536)
                    except BlockingIOError:
                        break
                    if not chunk:
                        break
                    output[stream].extend(chunk)
                    if len(output[stream]) > 4 * 1024 * 1024:
                        raise ValueError('driver output limit exceeded')
            return subprocess.CompletedProcess(command, process.returncode,
                                               output[process.stdout].decode(), output[process.stderr].decode())
    finally:
        signal.signal(signal.SIGTERM, previous)


def main():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    origin = f"http://127.0.0.1:{server.server_port}"
    binary = os.environ.get("CAD_RESPONSE_PROBE", "tools/browser-harness/target/debug/cad-response-probe")
    try:
        for mode in ("responses", "page-error", "missing-worker"):
            Handler.mode = mode
            result = run_probe([binary, origin + "/host"], timeout=180)
            if mode == "responses":
                assert result.returncode == 0, result.stderr
                output = json.loads(result.stdout)
                responses = {(r["url"].removeprefix(origin), r["status"], r["serviceWorker"]) for r in output["responses"]}
                assert {("/host", 200, False), ("/boardstudio/assets/cad-worker/entry.js", 200, False), ("/redirect", 302, False), ("/created", 201, False), ("/missing", 404, False)} == responses, responses
                assert not any(url == "/worker-final.js" or (url.endswith("/entry.js") and status == 302) for url, status, _ in responses), responses
                assert ("/favicon.ico", 302, False) not in responses, responses
                assert sum(r["url"] == origin + "/created" for r in output["responses"]) == 1, responses
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
