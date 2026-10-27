#!/usr/bin/env python3
"""Print optional local identity context for a manual Graphite comparison."""
import importlib.util
import json
import os
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("drag_identity", root / "scripts/run-drag-browser.py")
drag = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drag)
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
                  "served_build": "unverified; URL is not bound to local build or source"}), flush=True)
