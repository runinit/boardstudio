#!/usr/bin/env python3
"""Serve a built M1 candidate for publish-candidate and browser checks.

Root at `/` (site-root), subpath at `/boardstudio/` (site-subpath), both with the
COOP/COEP headers that publish-candidate requires. Run detached, for example:
    setsid nohup python3 scripts/serve-candidate.py <build-id> <port> >/dev/null 2>&1 < /dev/null &
"""
from pathlib import Path
import http.server
import os
import sys

BUILDS = Path(__file__).resolve().parents[1] / "web" / "target" / "builds"


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit("usage: serve-candidate.py BUILD_ID PORT")
    build = BUILDS / sys.argv[1]
    if not (build / "site-root").is_dir() or not (build / "site-subpath").is_dir():
        sys.exit(f"no completed build at {build}")

    class Handler(http.server.SimpleHTTPRequestHandler):
        def translate_path(self, path: str) -> str:
            path = path.split("?")[0].split("#")[0]
            site = "site-subpath" if path.startswith("/boardstudio/") else "site-root"
            return os.path.join(build, site, path.lstrip("/"))

        def end_headers(self) -> None:
            self.send_header("Cross-Origin-Opener-Policy", "same-origin")
            self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
            super().end_headers()

        def log_message(self, *args) -> None:
            pass

    http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[2])), Handler).serve_forever()


if __name__ == "__main__":
    main()
