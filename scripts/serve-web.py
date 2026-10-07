#!/usr/bin/env python3
"""Serve the built Dioxus /boardstudio/ site (and the root site, if built) with web isolation headers."""

from __future__ import annotations

import argparse
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


def handler_for(build_root: Path):
    has_root = (build_root / "site-root/index.html").is_file()

    class Handler(SimpleHTTPRequestHandler):
        def send_head(self):
            # `build-web.py` packages only the Pages route unless `--routes root,subpath` is given.
            if not has_root and self.path.partition("?")[0] == "/":
                self.send_response(302)
                self.send_header("Location", "/boardstudio/")
                self.end_headers()
                return None
            return super().send_head()

        def translate_path(self, path: str) -> str:
            url_path = path.partition("?")[0].partition("#")[0]
            if url_path == "/boardstudio" or url_path.startswith("/boardstudio/"):
                site = build_root / "site-subpath" / "boardstudio"
                relative = url_path[len("/boardstudio"):].lstrip("/")
            else:
                site = build_root / "site-root"
                relative = url_path.lstrip("/")
            target = (site / relative).resolve()
            try:
                target.relative_to(site.resolve())
            except ValueError:
                return str(site / "__not_found__")
            return str(target)

        def end_headers(self) -> None:
            self.send_header("Cross-Origin-Opener-Policy", "same-origin")
            self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
            super().end_headers()

        def log_message(self, fmt: str, *args: object) -> None:
            print(fmt % args)

    return Handler


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("build", type=Path, nargs="?", default=Path("web/target/site"),
                        help="build output containing site-subpath/boardstudio/ and optionally site-root/")
    parser.add_argument("port", type=int, nargs="?", default=4173)
    parser.add_argument("--host", default="127.0.0.1")
    args = parser.parse_args()
    build_root = args.build.resolve()
    required = build_root / "site-subpath/boardstudio/index.html"
    if not required.is_file():
        parser.error(f"build output is incomplete; missing {required}")
    server = ThreadingHTTPServer((args.host, args.port), handler_for(build_root))
    print(f"Serving {build_root} at http://{args.host}:{args.port}/", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
