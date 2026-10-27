"""The Node, playwright-core and Chrome the real-page browser smoke needs.

`check.py` probes before it builds the page; `run-drag-browser.py` probes again before it launches.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess


class MissingBrowserTool(Exception):
    """A prerequisite of the real-page smoke is absent; the message says how to supply it."""


def locate_chrome() -> str:
    """Returns the Chrome executable for the Rust harness, or raises `MissingBrowserTool`.

    `CHROME_BIN` takes precedence, followed by the drag-specific CI override
    and the usual executable names. The Rust driver needs no Node runtime.
    """
    chrome = os.environ.get("CHROME_BIN") or os.environ.get("BOARDSTUDIO_DRAG_CHROME_BIN") or next(
        (shutil.which(name) for name in ("google-chrome", "chromium", "chromium-browser") if shutil.which(name)),
        None,
    )
    if not chrome:
        raise MissingBrowserTool("drag browser smoke needs Chrome or Chromium; set CHROME_BIN to its executable")
    return chrome


def locate() -> tuple[str, str]:
    """Returns the `node` and Chrome executables, or raises `MissingBrowserTool`."""
    node = shutil.which("node")
    if not node:
        raise MissingBrowserTool("drag browser smoke needs Node.js")
    probe = subprocess.run([node, "-e", "require('playwright-core')"], capture_output=True, text=True)
    if probe.returncode:
        raise MissingBrowserTool("drag browser smoke needs playwright-core (the CI web-tools action installs it); "
                                 "set NODE_PATH to its node_modules directory")
    return node, locate_chrome()


def _drag_harness_binary() -> Path:
    root = Path(__file__).resolve().parents[1]
    metadata = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1",
         "--manifest-path", str(root / "tools/browser-harness/Cargo.toml")],
        cwd=root, check=True, capture_output=True, text=True,
    )
    return Path(json.loads(metadata.stdout)["target_directory"]) / "debug/drag-smoke"
