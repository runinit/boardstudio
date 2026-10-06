#!/usr/bin/env python3
"""Run Dioxus behind a small dev proxy that serves live, isolated app assets."""

from __future__ import annotations

import argparse
from http.client import HTTPConnection
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import mimetypes
from pathlib import Path
import select
import socket
import subprocess
import sys
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "web"
REQUIRED = (
    ROOT / "core/pkg/boardstudio_core_bg.wasm",
    ROOT / "web/assets/cad/boardstudio_cadrum_wasm_bg.wasm",
    ROOT / "web/assets/renderer/boardstudio_renderer_wasm_bg.wasm",
    ROOT / "web/assets/core-worker/entry.js",
    ROOT / "web/assets/core-worker/m1_core_worker_bg.wasm",
    ROOT / "web/assets/cad-worker/entry.js",
    ROOT / "web/assets/cad-worker/m1_cad_worker_bg.wasm",
    ROOT / "web/assets/imported-modules.json",
    ROOT / "web/assets/fixtures/sofle.boardstudio",
)


def add_isolation_headers(handler: BaseHTTPRequestHandler) -> None:
    handler.send_header("Cross-Origin-Opener-Policy", "same-origin")
    handler.send_header("Cross-Origin-Embedder-Policy", "require-corp")
    handler.send_header("Cross-Origin-Resource-Policy", "same-origin")
    handler.send_header("Cache-Control", "no-store")


def dev_handler(upstream_port: int, asset_root: Path | None = None):
    """Proxy app traffic to dx; serve /assets from the live source tree."""
    asset_root = (asset_root or ROOT / "web/assets").resolve()

    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.0"

        def end_headers(self):
            add_isolation_headers(self)
            super().end_headers()

        def log_message(self, fmt: str, *args: object) -> None:
            pass

        def _asset_path(self) -> Path | None:
            path = unquote(urlsplit(self.path).path)
            if not path.startswith("/assets/"):
                return None
            resolved = (asset_root / path[len("/assets/"):]).resolve()
            try:
                resolved.relative_to(asset_root)
            except ValueError:
                return asset_root / "__not_found__"
            return resolved

        def _static(self) -> None:
            path = self._asset_path()
            if path is None or not path.is_file():
                self.send_error(404)
                return
            body = path.read_bytes()
            content_type = mimetypes.guess_type(path.name)[0] or "application/octet-stream"
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            if self.command != "HEAD":
                self.wfile.write(body)

        def _proxy(self) -> None:
            if self.headers.get("Upgrade", "").lower() == "websocket":
                return self._websocket()
            body = None
            size = self.headers.get("Content-Length")
            if size:
                body = self.rfile.read(int(size))
            connection = HTTPConnection("127.0.0.1", upstream_port, timeout=120)
            try:
                headers = {key: value for key, value in self.headers.items()
                           if key.lower() not in {"host", "connection", "content-length"}}
                connection.request(self.command, self.path, body=body, headers=headers)
                response = connection.getresponse()
                payload = response.read()
                self.send_response(response.status, response.reason)
                content_length = False
                for key, value in response.getheaders():
                    lower = key.lower()
                    if lower in {"connection", "transfer-encoding", "keep-alive", "upgrade"}:
                        continue
                    if lower == "content-length":
                        content_length = True
                    if lower in {"cross-origin-opener-policy", "cross-origin-embedder-policy",
                                 "cross-origin-resource-policy"}:
                        continue
                    self.send_header(key, value)
                if not content_length:
                    self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                if self.command != "HEAD":
                    self.wfile.write(payload)
            except OSError as error:
                self.send_error(502, f"Dioxus dev server unavailable: {error}")
            finally:
                connection.close()

        def _websocket(self) -> None:
            try:
                upstream = socket.create_connection(("127.0.0.1", upstream_port), timeout=10)
            except OSError as error:
                self.send_error(502, f"Dioxus dev server unavailable: {error}")
                return
            request = f"{self.command} {self.path} {self.request_version}\r\n"
            request += f"Host: {self.headers.get('Host', f'127.0.0.1:{upstream_port}')}\r\n"
            request += "".join(f"{key}: {value}\r\n" for key, value in self.headers.items()
                               if key.lower() != "host") + "\r\n"
            upstream.sendall(request.encode("latin-1"))
            received = bytearray()
            while b"\r\n\r\n" not in received:
                chunk = upstream.recv(4096)
                if not chunk:
                    upstream.close()
                    return
                received.extend(chunk)
            boundary = received.index(b"\r\n\r\n") + 4
            headers = bytes(received[:boundary])
            self.connection.sendall(headers)
            if len(received) > boundary:
                self.connection.sendall(received[boundary:])
            if not headers.startswith(b"HTTP/1.1 101"):
                upstream.close()
                return
            try:
                while True:
                    readable, _, failed = select.select([self.connection, upstream], [],
                                                        [self.connection, upstream], 60)
                    if failed:
                        return
                    if not readable:
                        continue
                    for source in readable:
                        chunk = source.recv(65536)
                        if not chunk:
                            return
                        (upstream if source is self.connection else self.connection).sendall(chunk)
            except OSError:
                return
            finally:
                upstream.close()

        def do_GET(self):
            return self._static() if self._asset_path() is not None else self._proxy()

        def do_HEAD(self):
            return self._static() if self._asset_path() is not None else self._proxy()

        def do_POST(self):
            return self._proxy()

        def do_PUT(self):
            return self._proxy()

        def do_DELETE(self):
            return self._proxy()

    return Handler


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8080)
    args = parser.parse_args(argv)
    missing = [path.relative_to(ROOT).as_posix() for path in REQUIRED if not path.is_file()]
    if missing:
        print("Preparing missing runtime assets for the first development run:", flush=True)
        setup = subprocess.run([sys.executable, "scripts/build-web.py", "--providers-only"], cwd=ROOT, check=False)
        if setup.returncode:
            return setup.returncode
        missing = [path.relative_to(ROOT).as_posix() for path in REQUIRED if not path.is_file()]
        if missing:
            print("Provider build did not create required assets:\n  " + "\n  ".join(missing),
                  file=sys.stderr)
            return 2
    internal_port = args.port + 1
    try:
        server = ThreadingHTTPServer(("127.0.0.1", args.port), dev_handler(internal_port))
    except OSError as error:
        print(f"dev-web: could not start on 127.0.0.1:{args.port}: {error}", file=sys.stderr)
        return 1
    server.daemon_threads = True
    print(f"$ dx serve --web --addr 127.0.0.1 --port {internal_port} --cross-origin-policy"
          " --open false --no-default-features --features page --cargo-args=--locked", flush=True)
    process = None
    try:
        process = subprocess.Popen([
            "dx", "serve", "--web", "--addr", "127.0.0.1", "--port", str(internal_port),
            "--cross-origin-policy", "--open", "false", "--no-default-features",
            "--features", "page", "--cargo-args=--locked",
        ], cwd=WEB)
        server.timeout = 0.2
        while process.poll() is None:
            server.handle_request()
        return process.returncode
    except OSError as error:
        print(f"dev-web: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130
    finally:
        server.server_close()
        if process is not None and process.poll() is None:
            process.terminate()
            process.wait(timeout=10)


if __name__ == "__main__":
    raise SystemExit(main())
