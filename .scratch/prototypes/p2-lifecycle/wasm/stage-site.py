#!/usr/bin/env python3
"""Stage one locked Dioxus output with exact same-origin worker/renderer URLs."""

from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]
REQUIRED_ASSETS = {
    "p1-worker/p1_core.js", "p1-worker/p1_core_bg.wasm",
    "renderer/boardstudio_renderer_wasm.js", "renderer/boardstudio_renderer_wasm_bg.wasm",
    "worker/worker-entry.js",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    if len(sys.argv) != 4 or sys.argv[1] not in {"root", "subpath"}:
        raise SystemExit("usage: stage-site.py <root|subpath> <build-id> <dx-public-directory>")
    mode, build_id, public_arg = sys.argv[1:]
    if not build_id.replace("-", "").isalnum():
        raise SystemExit("build-id must contain only letters, numbers, and hyphens")
    public = Path(public_arg).resolve()
    if not (public / "index.html").is_file() or not (public / "assets").is_dir():
        raise SystemExit(f"not a Dioxus public output: {public}")

    provenance_file = ROOT / "asset-provenance.json"
    if not provenance_file.is_file():
        raise SystemExit("missing provider provenance; run build-release.py first")
    provenance = json.loads(provenance_file.read_text())
    if provenance.get("schema") != 1 or not provenance.get("source_files"):
        raise SystemExit("invalid provider provenance")
    expected_assets = provenance.get("assets", {})
    if not REQUIRED_ASSETS.issubset(expected_assets):
        raise SystemExit("provider provenance is missing required assets")
    for relative, expected in provenance["source_files"].items():
        source = (REPO / relative).resolve()
        if not source.is_relative_to(REPO) or not source.is_file() or sha256(source) != expected:
            raise SystemExit(f"provider source changed since build: {relative}; rebuild providers")
    for relative in REQUIRED_ASSETS:
        source = ROOT / "assets" / relative
        if not source.is_file() or sha256(source) != expected_assets[relative]:
            raise SystemExit(f"provider artifact changed since build: {relative}; rebuild providers")

    stage_root = ROOT / "target" / "renewal-worker"
    site_name = f"site-{mode}-{build_id}"
    destination = stage_root / site_name
    if mode == "subpath":
        destination /= "boardstudio"
    if destination.exists():
        raise SystemExit(f"refusing to overwrite staged evidence: {destination}")
    shutil.copytree(public, destination)

    staged: dict[str, str] = {}
    for relative in sorted(REQUIRED_ASSETS):
        expected_hash = expected_assets[relative]
        source = ROOT / "assets" / relative
        actual = sha256(source)
        if actual != expected_hash:
            raise SystemExit(f"approved generated input changed: {relative} {actual} != {expected_hash}")
        target = destination / "assets" / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        copied = sha256(target)
        if copied != expected_hash:
            raise SystemExit(f"staging copy mismatch: {relative}")
        staged[f"assets/{relative}"] = copied

    bundle = {
        "provider_provenance": provenance,
        "mode": mode,
        "build_id": build_id,
        "base_path": "/" if mode == "root" else "/boardstudio/",
        "dioxus_public": str(public),
        "index_sha256": sha256(public / "index.html"),
        "staged_site": str(destination),
        "dioxus_assets": {
            str(path.relative_to(public)): sha256(path)
            for path in sorted((public / "assets").iterdir())
            if path.is_file()
        },
        "same_origin_worker_renderer_assets": staged,
        "asset_url_paths": {
            "worker_entry": "/assets/worker/worker-entry.js" if mode == "root" else "/boardstudio/assets/worker/worker-entry.js",
            "renderer_module": "/assets/renderer/boardstudio_renderer_wasm.js" if mode == "root" else "/boardstudio/assets/renderer/boardstudio_renderer_wasm.js",
            "renderer_wasm": "/assets/renderer/boardstudio_renderer_wasm_bg.wasm" if mode == "root" else "/boardstudio/assets/renderer/boardstudio_renderer_wasm_bg.wasm",
        },
        "fixture_sha256": hashlib.sha256((ROOT / "../fixtures/reviung41.json").read_bytes()).hexdigest(),
    }
    evidence = ROOT.parent / "evidence" / "renewal-worker" / f"staging-{mode}-{build_id}.json"
    evidence.write_text(json.dumps(bundle, indent=2) + "\n")
    print(destination)
    print(evidence)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
