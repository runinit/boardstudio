#!/usr/bin/env python3
"""Bump the decoupled Chrome-for-Testing pin to a new CfT Stable version.

Usage: python3 scripts/bump-chrome-for-testing.py VERSION

Downloads both linux64 zips for VERSION from the CfT public bucket, computes
sha256, rewrites scripts/chrome-for-testing.json, then runs
scripts/install-chrome-for-testing.sh and the skew probe
(tools/browser-harness --bin skew-probe) against the newly installed chrome
(CHROME_BIN=<installed chrome>), printing a PASS/FAIL summary.

Blocked-bump path: if the probe finds required skew, this script exits
nonzero with a message that the pin file was updated but the bump is BLOCKED.
It does NOT auto-revert the pin file; say so in the output.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PIN_JSON = REPO / "scripts" / "chrome-for-testing.json"
INSTALLER = REPO / "scripts" / "install-chrome-for-testing.sh"
HARNESS_DIR = REPO / "tools" / "browser-harness"

VERSION_RE = re.compile(r"^\d+\.\d+\.\d+\.\d+$")
BUCKET = "https://storage.googleapis.com/chrome-for-testing-public"


def rewrite_pin(pin_path: Path, version: str, chrome_sha256: str, driver_sha256: str) -> dict:
    """Pure pin-rewrite step: importable/testable without network."""
    if not VERSION_RE.match(version):
        raise ValueError(f"refusing to write invalid version: {version!r}")
    for name, digest in (("chrome_sha256", chrome_sha256), ("chromedriver_sha256", driver_sha256)):
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValueError(f"refusing to write invalid {name}: {digest!r}")
    pin = {"version": version, "chrome_sha256": chrome_sha256,
           "chromedriver_sha256": driver_sha256}
    pin_path.write_text(json.dumps(pin, indent=2) + "\n")
    return pin


def fetch_sha256(version: str, name: str) -> str:
    url = f"{BUCKET}/{version}/linux64/{name}-linux64.zip"
    digest = hashlib.sha256()
    with tempfile.NamedTemporaryFile(prefix=f"cft-{name}-", suffix=".zip", delete=True) as staged:
        with urllib.request.urlopen(url) as response, open(staged.name, "wb") as out:
            shutil_copy(response, out, digest)
        print(f"downloaded {url}\n  sha256={digest.hexdigest()}", flush=True)
    return digest.hexdigest()


def shutil_copy(response, out, digest) -> None:
    while True:
        chunk = response.read(1024 * 1024)
        if not chunk:
            return
        digest.update(chunk)
        out.write(chunk)


def run(argv: list[str], env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
    print("$ " + " ".join(argv), flush=True)
    return subprocess.run(argv, cwd=REPO, env=env)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version", help="CfT version to pin, e.g. 157.0.1234.56")
    args = parser.parse_args()
    version = args.version.strip()
    if not VERSION_RE.match(version):
        print(f"FAIL: refusing to bump to invalid version {version!r}", file=sys.stderr)
        return 2

    print(f"Bumping Chrome-for-Testing pin to {version} ...", flush=True)
    chrome_sha = fetch_sha256(version, "chrome")
    driver_sha = fetch_sha256(version, "chromedriver")
    rewrite_pin(PIN_JSON, version, chrome_sha, driver_sha)
    print(f"Pin file updated: {PIN_JSON} -> {version}", flush=True)

    installed = run(["bash", str(INSTALLER)], env={**os.environ})
    if installed.returncode != 0:
        print("FAIL: install-chrome-for-testing.sh failed after the pin update; "
              "the pin file was updated but the bump is BLOCKED (not reverted).",
              file=sys.stderr)
        return 1
    # The installer echoes the chrome/chromedriver paths; the probe needs only
    # CHROME_BIN, re-derived here from the install dir (no log parsing).
    chrome_path = (Path(os.environ.get("BOARDSTUDIO_CFT_DIR",
                                       Path.home() / ".cache/boardstudio/chrome-for-testing" / version))
                   / "chrome-linux64" / "chrome")
    probe_env = {**os.environ, "CHROME_BIN": str(chrome_path)}
    probe = run(["cargo", "run", "--locked", "--manifest-path",
                 str(HARNESS_DIR / "Cargo.toml"), "--bin", "skew-probe"],
                env=probe_env)
    if probe.returncode != 0:
        print(f"FAIL: skew probe found required skew on {version}; "
              f"the pin file was updated but the bump is BLOCKED (not auto-reverted). "
              f"Re-run the probe with CHROME_BIN={chrome_path} after triage.",
              file=sys.stderr)
        return 1
    print(f"PASS: CfT bump to {version} installed and skew-probe clean.", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
