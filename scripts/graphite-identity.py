#!/usr/bin/env python3
"""Print optional local identity context for a manual Graphite comparison."""
import importlib.util
import json
import os
import subprocess
from urllib.parse import quote, urljoin, urlsplit
from urllib.request import urlopen
from pathlib import Path

def verify_served(site, url):
    parsed = urlsplit(url)
    if parsed.scheme not in ("http", "https") or parsed.query or parsed.fragment or not parsed.path.endswith("/boardstudio/"):
        raise ValueError("BOARDSTUDIO_PREVIEW_URL must end in /boardstudio/ with no query or fragment")
    if not (site / "index.html").is_file():
        raise ValueError("built site index.html is missing")
    for file in sorted(p for p in site.rglob("*") if p.is_file()):
        relative = file.relative_to(site).as_posix()
        target = urljoin(url, quote(relative))
        with urlopen(target, timeout=10) as response:
            if response.geturl() != target or response.read() != file.read_bytes():
                raise ValueError(f"served file differs from build: {relative}")


def main():
    root = Path(__file__).resolve().parents[1]
    spec = importlib.util.spec_from_file_location("drag_identity", root / "scripts/run-drag-browser.py")
    drag = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(drag)
    build_run = os.environ.get("BOARDSTUDIO_GRAPHITE_BUILD_RUN")
    if build_run:
        try:
            perf_spec = importlib.util.spec_from_file_location("measure_cad", root / "cad/scripts/measure-cad.py")
            perf = importlib.util.module_from_spec(perf_spec)
            perf_spec.loader.exec_module(perf)
            profile = os.environ.get("BOARDSTUDIO_GRAPHITE_PROFILE", "checked")
            if profile not in perf.PROFILES:
                raise ValueError("BOARDSTUDIO_GRAPHITE_PROFILE must be checked or release")
            bound, current = perf.binding(root, profile)
            if not current:
                raise ValueError("official CAD provider stamp is missing or stale")
            perf.validate_page_build_receipt(None, Path(build_run), "\n".join(bound) + "\n")
            url = os.environ.get("BOARDSTUDIO_PREVIEW_URL", "")
            verify_served(root / "web/target/site/site-subpath/boardstudio", url)
            print("Graphite comparison preflight: source, build receipt and served files match", flush=True)
        except (OSError, ValueError, KeyError, TypeError) as error:
            raise SystemExit(f"Graphite build receipt/served binding rejected: {error}; build a fresh site and supply its --build-run via BOARDSTUDIO_GRAPHITE_BUILD_RUN") from error

    head, source_hash = drag.source_identity()
    build_dir = os.environ.get("BOARDSTUDIO_GRAPHITE_BUILD_DIR")
    build = None
    if build_dir:
        path = Path(build_dir).resolve(strict=True)
        if not path.is_dir():
            raise SystemExit(f"Graphite build directory is not a directory: {path}")
        files = [item for item in path.rglob("*") if item.is_file()]
        if not files:
            raise SystemExit(f"Graphite build directory is empty: {path}")
        build = {"path": str(path), "sha256": drag.digest_files(files, path)}
    print(json.dumps({"checkout": str(root), "head": head, "source_sha256": source_hash,
                      "dirty": bool(subprocess.run(['git', 'status', '--porcelain'], cwd=root,
                                    check=True, capture_output=True, text=True).stdout),
                      "local_build": build,
                      "url": os.environ.get("BOARDSTUDIO_PREVIEW_URL", "http://127.0.0.1:8182/"),
                      "served_build": "matched at preflight" if build_run else "unverified; URL is not bound to local build or source"}), flush=True)


if __name__ == "__main__":
    main()
