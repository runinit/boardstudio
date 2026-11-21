#!/usr/bin/env python3
"""Bake allowlisted Ergogen STEP previews for the browser CAD worker."""
from hashlib import sha256
import gzip
import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory

from cad_build import CAD_ROOT, run, wasm_inputs_digest

ROOT = CAD_ROOT.parent
EXAMPLE = CAD_ROOT / "wasm/examples/bake_bundled_step.rs"
# These vendor fallback assets have matching default native and WASM emissions
# and no bounded source proof. The remaining originals use live import.
SOURCES = (
    "infused-kim/3d_models/Choc_V1_Switch.step",
    "kicad/3d_models/Panasonic_EVQPUJ_EVQPUA.step",
)
OUTPUT = ROOT / "web/target/providers/bundled-step"


def bake() -> None:
    vendor_root = ROOT / "ergogen/library/vendor"
    sources = {vendor_root / path: sha256((vendor_root / path).read_bytes()).hexdigest()
               for path in SOURCES}
    key = sha256(wasm_inputs_digest(no_opt=True).encode() + EXAMPLE.read_bytes()
                 + Path(__file__).read_bytes() + "".join(sources.values()).encode()).hexdigest()
    manifest = OUTPUT / "manifest.json"
    if manifest.is_file():
        previous = json.loads(manifest.read_text())
        if previous["inputs"] == key and all(
            (OUTPUT / f"{payload}.bstep").is_file()
            and sha256((OUTPUT / f"{payload}.bstep").read_bytes()).hexdigest() == payload
            for payload in previous["assets"].values()
        ):
            print("cad: bundled STEP previews are current", flush=True)
            return

    OUTPUT.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(dir=OUTPUT) as temporary:
        command = ["cargo", "run", "--locked", "--profile", "cad-test",
                   "-p", "boardstudio-cad-wasm", "--example", "bake_bundled_step", "--"]
        pending = {}
        for source, digest in sources.items():
            raw = Path(temporary) / digest
            pending[digest] = raw
            command.extend((str(source), digest, str(raw)))
        run(command, env=dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / "target")))
        assets = {}
        for digest, raw in pending.items():
            if not raw.is_file():
                # The importer may refuse a preview or produce a source proof.
                continue
            compressed = gzip.compress(raw.read_bytes(), mtime=0)
            identity = sha256(compressed).hexdigest()
            (OUTPUT / f"{identity}.bstep").write_bytes(compressed)
            assets[digest] = identity

        staged_manifest = Path(temporary) / "manifest.json"
        staged_manifest.write_text(json.dumps({"inputs": key, "assets": assets}) + "\n")
        staged_manifest.replace(manifest)
        wanted = {f"{digest}.bstep" for digest in assets.values()}
        for stale in OUTPUT.glob("*.bstep"):
            if stale.name not in wanted:
                stale.unlink()
    print(f"cad: baked {len(assets)} bundled STEP previews", flush=True)


if __name__ == "__main__":
    bake()
