#!/usr/bin/env python3
"""Bump the decoupled Chrome-for-Testing pin to a new CfT Stable version.

Usage: python3 scripts/bump-chrome-for-testing.py VERSION

Downloads both linux64 zips for VERSION from the CfT public bucket, computes
sha256, rewrites scripts/chrome-for-testing.json, then runs
scripts/install-chrome-for-testing.sh and opens a page with the installed pair.
A failing smoke exits nonzero with the updated pin still on disk.
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

import browser_tools

REPO = Path(__file__).resolve().parent.parent
PIN_JSON = REPO / "scripts" / "chrome-for-testing.json"
INSTALLER = REPO / "scripts" / "install-chrome-for-testing.sh"

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


def smoke(base):
    with browser_tools.WebDriverSession(chrome=base / "chrome-linux64/chrome",
                                        driver=base / "chromedriver-linux64/chromedriver",
                                        timeout=30) as browser:
        browser.get("data:text/html,<title>BoardStudio</title>")
        if browser.execute_sync("return document.title;") != "BoardStudio":
            raise RuntimeError("WebDriver did not execute JavaScript on the smoke page")
        browser.check_page_errors()


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
    base = browser_tools.pinned_cft_dir()
    try:
        smoke(base)
    except (OSError, RuntimeError) as error:
        print(f"FAIL: WebDriver smoke on {version}: {error}. The updated pin remains in place.",
              file=sys.stderr)
        return 1
    print(f"PASS: CfT bump to {version} installed and WebDriver smoke passed.", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
