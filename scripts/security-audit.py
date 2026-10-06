#!/usr/bin/env python3
"""Audit every lockfile, including the registry identity of patched cgmath."""

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
VENDOR = ROOT / "renderer/vendor/cgmath-0.18.0"
PATCHED_ADVISORY = "RUSTSEC-2026-0197"


def verify_vendor():
    provenance = json.loads((VENDOR.parent / "cgmath-security.json").read_text())
    if provenance["patchedAdvisory"] != PATCHED_ADVISORY:
        raise ValueError("Unexpected cgmath advisory exception")
    actual = {
        str(file.relative_to(VENDOR)): hashlib.sha256(file.read_bytes()).hexdigest()
        for file in VENDOR.rglob("*")
        if file.is_file() and file.relative_to(VENDOR).parts[0] != "target"
    }
    if actual != provenance["files"]:
        raise ValueError("Vendored cgmath differs from the reviewed security patch")
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--locked", "--format-version", "1",
        "--manifest-path", "renderer/Cargo.toml",
    ], cwd=ROOT))
    packages = [package for package in metadata["packages"] if package["name"] == "cgmath"]
    if len(packages) != 1 or Path(packages[0]["manifest_path"]).resolve() != VENDOR / "Cargo.toml":
        raise ValueError("Renderer must resolve exclusively to the reviewed cgmath patch")
    return provenance


def registry_lockfile(source, provenance):
    # cargo-audit skips path dependencies. Restore the original registry identity
    # only in its audit input so future cgmath advisories are still detected.
    blocks = source.split("[[package]]")
    for index, block in enumerate(blocks[1:], 1):
        package = tomllib.loads(block)
        if package["name"] == "cgmath" and "source" not in package:
            version = package["version"]
            if version != provenance["version"]:
                raise ValueError("Unexpected vendored cgmath version")
            blocks[index] = block.replace(
                f'version = "{version}"',
                f'version = "{version}"\n'
                'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
                f'checksum = "{provenance["archiveSha256"]}"',
                1,
            )
    return "[[package]]".join(blocks)


def main():
    provenance = verify_vendor()
    failed = False
    locks = ["core/Cargo.lock", "renderer/Cargo.lock", "cad/wasm/Cargo.lock",
             "renderer/vendor/cgmath-0.18.0/Cargo.lock"]
    for lock in locks:
        print(f"\nAuditing {lock}", flush=True)
        source = (ROOT / lock).read_text()
        patched = lock.startswith("renderer/")
        with tempfile.NamedTemporaryFile(mode="w", suffix=".lock") as audit_input:
            audit_input.write(registry_lockfile(source, provenance) if patched else source)
            audit_input.flush()
            command = ["cargo", "audit", "--file", audit_input.name, "--deny", "unsound"]
            if patched:
                print(f"Verified local fix for {PATCHED_ADVISORY}; other advisories remain enforced.", flush=True)
                command.extend(["--ignore", PATCHED_ADVISORY])
            failed |= subprocess.run(command, cwd=ROOT).returncode != 0
    raise SystemExit(1 if failed else 0)


if __name__ == "__main__":
    main()
