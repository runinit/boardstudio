#!/usr/bin/env python3
"""Stage one locked Dioxus output with exact same-origin worker/renderer URLs."""

from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
EXPECTED = {
    "p1-worker/p1_core.js": "22159af2bcdea073243e47408c716900021ba52055161ad2de5547a0a1e9870d",
    "p1-worker/p1_core_bg.wasm": "913041a1fc01e5337dbfb3c0c53a217761c31209965ae3268d4780d9113a7f26",
    "renderer/boardstudio_renderer_wasm.js": "2131b4d8e23929e24e5f533f1cb86264e39ef7532d0dc4fee494bce65e0cc6fc",
    "renderer/boardstudio_renderer_wasm_bg.wasm": "ed5e115042045358680ad4355ddceb5d457aa3b906f23646fc31494caf979864",
    "worker/worker-entry.js": "652acf853416adcf6af0cd83183a048df67d6c53cad3608af313117d3647c948",
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

    stage_root = ROOT / "target" / "renewal-worker"
    site_name = f"site-{mode}-{build_id}"
    destination = stage_root / site_name
    if mode == "subpath":
        destination /= "boardstudio"
    if destination.exists():
        raise SystemExit(f"refusing to overwrite staged evidence: {destination}")
    shutil.copytree(public, destination)

    staged: dict[str, str] = {}
    for relative, expected_hash in EXPECTED.items():
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
        "mode": mode,
        "build_id": build_id,
        "base_path": "/" if mode == "root" else "/boardstudio/",
        "dioxus_public": str(public),
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
