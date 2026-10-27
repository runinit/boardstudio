"""The Node, playwright-core and Chrome the real-page browser smoke needs.

`check.py` probes before it builds the page; `run-drag-browser.py` probes again before it launches.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import re
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


def build_drag_harness() -> Path:
    """Build under the shared lock and return Cargo's executable for this build."""
    root = Path(__file__).resolve().parents[1]
    lock = Path.home() / ".cache/tmp/boardstudio-build.lock"
    lock.parent.mkdir(parents=True, exist_ok=True)
    if not shutil.which("flock"):
        raise MissingBrowserTool("drag browser smoke needs flock for the shared build lock; install flock (util-linux)")
    result = subprocess.run(
        ["flock", str(lock), "cargo", "build", "--locked", "--message-format=json",
         "--manifest-path", str(root / "tools/browser-harness/Cargo.toml"),
         "--package", "boardstudio-browser-harness", "--bin", "drag-smoke"],
        cwd=root, capture_output=True, text=True,
    )
    if result.returncode:
        diagnostic = result.stderr
        for line in result.stdout.splitlines():
            try:
                message = json.loads(line)
                if message.get("reason") == "compiler-message":
                    diagnostic += message.get("message", {}).get("rendered") or ""
            except (ValueError, AttributeError):
                pass
        for name, value in os.environ.items():
            if value and re.search(r"TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL|API_KEY", name, re.I):
                diagnostic = diagnostic.replace(value, "<REDACTED>")
        # URLs can contain credentials in userinfo, queries or fragments; do not guess their end.
        diagnostic = re.sub(r"(?i)\b[a-z][a-z0-9+.-]*://[^\r\n]*", "<URL diagnostic withheld>", diagnostic)
        # A sensitive field can contain nested or multiline data; do not guess its end.
        sensitive = re.search(r"(?i)(?:token|secret|password|passwd|credential|api[_-]?key|authorization)"
                              r"[\w-]*[\"']?\s*[=:]|\bBearer\s+", diagnostic)
        if sensitive:
            diagnostic = diagnostic[:sensitive.start()] + "<sensitive diagnostic withheld>\n"
        raise SystemExit(f"drag browser smoke cannot build the Rust harness:\n{diagnostic}")
    try:
        artifacts = [message["executable"] for line in result.stdout.splitlines()
                     if (message := json.loads(line)).get("reason") == "compiler-artifact"
                     and message.get("package_id", "").startswith(
                         f"path+{(root / 'tools/browser-harness').as_uri()}#")
                     and message.get("target", {}).get("name") == "drag-smoke"
                     and "bin" in message["target"].get("kind", []) and message.get("executable")]
    except (ValueError, KeyError, TypeError) as error:
        raise SystemExit(f"drag browser smoke cannot read Cargo build output: {error}") from error
    if len(artifacts) != 1:
        raise SystemExit("drag browser smoke cannot identify its executable in Cargo build output")
    return Path(artifacts[0])
