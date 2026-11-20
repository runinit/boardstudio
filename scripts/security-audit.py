#!/usr/bin/env python3
"""Audit every lockfile, including advisories for the cgmath-next fork."""

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
VENDOR = ROOT / "renderer/vendor/cgmath-next-0.18.1"
PROVENANCE = ROOT / "renderer/vendor/cgmath-next-security.json"
PROVENANCE_SHA256 = "4118cd11df30c1011214ddcbd57f714ad72858b38f491a717e15f58d32b10ea7"
UPSTREAM_PACKAGE = "cgmath-next"
UPSTREAM_REPO = "https://github.com/kent-tokyo/cgmath-next"
UPSTREAM_REVISION = "ca1e234d333d184894d1838cd67451a871f33ba1"
UPSTREAM_ARCHIVE_SHA256 = "91e7fb669c7eaa07328128cc7168bb8a8ffc03f01b872cc8a50793e5b5068f30"
REGISTRY_ARCHIVE_SHA256 = "1a98d30140e3296250832bbaaff83b27dcd6fa3cc70fb6f1f3e5c9c0023b5317"
PATCHED_ADVISORIES = ("RUSTSEC-2026-0196", "RUSTSEC-2026-0197")
# Accepted pending-upstream-fix (reviewed 2026-10-28): RUSTSEC-2026-0386 flags
# tokio 1.53.1 `AsyncFd` constructors as unsound. 1.53.1 is the newest tokio
# compatible with this tree (`cargo update -p tokio` locks nothing newer), so
# no patched release exists to take, and first-party code never calls the
# affected API. Revisit on the next lockfile bump.
# Accepted pending-upstream-fix (reviewed 2026-11-19): RUSTSEC-2026-0574
# through RUSTSEC-2026-0582 flag heapless 0.8.0 APIs as unsound. 0.8.0 is the
# newest lockable heapless: it is the only 0.8.x release, rstar 0.13.1 still
# requires heapless ^0.8, and rstar 0.14 (heapless ^0.9) is excluded by
# parry3d-f64 0.31.1's rstar ^0.13 requirement (0.31.1 is the latest parry
# release), so no patched release exists to take. All nine advisories name
# affected functions outside `binary_heap` (sorted_linked_list, spsc, deque,
# index_map, history_buf, vec::IntoIter, pools; 0579's `Deque::try_from`
# range starts at 0.9.2 and 0580's `pool` range ends below 0.8.0), while
# rstar 0.13.0 -- the sole heapless consumer in cad/Cargo.lock --
# uses only `heapless::binary_heap` (`BinaryHeap::{new,push,pop,len,capacity,
# into_vec}` in `src/algorithm/nearest_neighbor.rs`). The one reachable
# 0577-adjacent path, `vec::IntoIter::drop` via `into_vec`, needs a panicking
# element `Drop` to trigger, and rstar's element type is `Copy` with no
# `Drop` (a shared reference plus a numeric scalar), so no affected API is
# reachable. Revisit when parry3d-f64 supports rstar 0.14+.
ACCEPTED_ADVISORIES = (
    "RUSTSEC-2026-0386",
    "RUSTSEC-2026-0574",
    "RUSTSEC-2026-0575",
    "RUSTSEC-2026-0576",
    "RUSTSEC-2026-0577",
    "RUSTSEC-2026-0578",
    "RUSTSEC-2026-0579",
    "RUSTSEC-2026-0580",
    "RUSTSEC-2026-0581",
    "RUSTSEC-2026-0582",
)
CGMATH_MANIFESTS = (
    "Cargo.toml",
    "cad/Cargo.toml",
    "renderer/vendor/cgmath-next-0.18.1/Cargo.toml",
)


def verify_vendor():
    provenance_bytes = PROVENANCE.read_bytes()
    if hashlib.sha256(provenance_bytes).hexdigest() != PROVENANCE_SHA256:
        raise ValueError("cgmath-next provenance record has changed")
    provenance = json.loads(provenance_bytes)
    if (provenance["upstreamRepo"] != UPSTREAM_REPO
            or provenance["upstreamVersion"] != "0.18.1"
            or provenance["upstreamRevision"] != UPSTREAM_REVISION
            or provenance["upstreamArchiveSha256"] != UPSTREAM_ARCHIVE_SHA256
            or provenance["archiveSha256"] != REGISTRY_ARCHIVE_SHA256):
        raise ValueError("Unexpected cgmath-next source revision")
    actual = {
        str(file.relative_to(VENDOR)): hashlib.sha256(file.read_bytes()).hexdigest()
        for file in VENDOR.rglob("*")
        if file.is_file() and file.relative_to(VENDOR).parts[0] != "target"
    }
    if actual != provenance["files"]:
        raise ValueError("Vendored cgmath-next differs from the reviewed source")
    manifest = tomllib.loads((VENDOR / "Cargo.toml").read_text())
    upstream_manifest = tomllib.loads((VENDOR / "Cargo.toml.orig").read_text())
    if manifest["package"]["name"] != "cgmath" or manifest["package"]["version"] != provenance["version"]:
        raise ValueError("Vendored package identity must match cgmath consumers")
    if (upstream_manifest["package"]["name"] != UPSTREAM_PACKAGE
            or upstream_manifest["package"]["version"] != provenance["upstreamVersion"]):
        raise ValueError("Vendored source is not the cgmath-next package")
    for manifest in CGMATH_MANIFESTS:
        metadata = json.loads(subprocess.check_output([
            "cargo", "metadata", "--locked", "--format-version", "1",
            "--manifest-path", manifest,
        ], cwd=ROOT))
        packages = [package for package in metadata["packages"] if package["name"] == "cgmath"]
        if (len(packages) != 1 or packages[0]["version"] != provenance["version"]
                or Path(packages[0]["manifest_path"]).resolve() != VENDOR / "Cargo.toml"
                or not any(target["name"] == "cgmath" and "lib" in target["kind"]
                           for target in packages[0]["targets"])):
            raise ValueError(f"{manifest} must resolve exclusively to the reviewed cgmath-next source")
    return provenance


def registry_lockfile(source, provenance):
    # cargo-audit skips path dependencies. Restore the cgmath identity only in
    # its audit input so future advisories still apply to this renamed fork.
    blocks = source.split("[[package]]")
    for index, block in enumerate(blocks[1:], 1):
        package = tomllib.loads(block)
        if package["name"] == "cgmath" and "source" not in package:
            version = package["version"]
            if version != provenance["version"]:
                raise ValueError("Unexpected vendored cgmath-next version")
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
    for manifest in CGMATH_MANIFESTS:
        lock = Path(manifest).with_name("Cargo.lock")
        print(f"\nAuditing {lock}", flush=True)
        source = (ROOT / lock).read_text()
        with tempfile.NamedTemporaryFile(mode="w", suffix=".lock") as audit_input:
            audit_input.write(registry_lockfile(source, provenance))
            audit_input.flush()
            command = ["cargo", "audit", "--file", audit_input.name, "--deny", "unsound"]
            print(f"Verified cgmath-next source; ignoring its renamed upstream advisories {', '.join(PATCHED_ADVISORIES)}.", flush=True)
            for advisory in (*PATCHED_ADVISORIES, *ACCEPTED_ADVISORIES):
                command.extend(["--ignore", advisory])
            failed |= subprocess.run(command, cwd=ROOT).returncode != 0
    raise SystemExit(1 if failed else 0)


if __name__ == "__main__":
    main()
