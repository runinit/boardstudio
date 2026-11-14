"""Chrome and build tools the real-browser harness checks need."""

from __future__ import annotations

import json
import os
from pathlib import Path
import re
import shutil
import subprocess


CHROME_BINARIES = ("google-chrome-stable", "google-chrome", "chromium", "chromium-browser", "chrome")
CFT_DIR_ENV = "BOARDSTUDIO_CFT_DIR"
CFT_PIN_JSON = Path(__file__).with_name("chrome-for-testing.json")


class MissingBrowserTool(Exception):
    """A prerequisite of a real-browser check is absent; the message says how to supply it."""


def pinned_cft_dir(env=None):
    """Pinned Chrome-for-Testing install dir, or None when the pin cannot be read.

    Explicit BOARDSTUDIO_CFT_DIR wins; otherwise the default cache path derived
    from the version in scripts/chrome-for-testing.json. Never raises: an
    unreadable pin simply means no pinned install.
    """
    src = os.environ if env is None else env
    override = src.get(CFT_DIR_ENV) or os.environ.get(CFT_DIR_ENV)
    if override:
        return Path(override)
    try:
        version = json.loads(CFT_PIN_JSON.read_text())["version"]
    except (OSError, ValueError, KeyError, TypeError):
        return None
    if not version:
        return None
    return Path(os.path.expanduser("~/.cache/boardstudio/chrome-for-testing")) / version


def pinned_cft_binary(kind, env=None):
    """Path to the pinned CfT `kind` binary ('chrome' or 'chromedriver'), or None."""
    base = pinned_cft_dir(env)
    if base is None:
        return None
    candidate = base / f"{kind}-linux64" / kind
    return str(candidate) if candidate.is_file() else None


def locate_chrome(env=None) -> str:
    """Return Chrome: explicit CHROME_BIN > pinned CfT install > PATH discovery.

    No Node runtime is required. Raise `MissingBrowserTool` when none is found.
    """
    src = os.environ if env is None else env
    chrome = src.get("CHROME_BIN") or pinned_cft_binary("chrome", src)
    if chrome:
        return chrome
    for name in CHROME_BINARIES:
        chrome = shutil.which(name, path=src.get("PATH"))
        if chrome:
            return chrome
    raise MissingBrowserTool("browser harness needs Chrome or Chromium; set CHROME_BIN to its executable")


def _inherited_build_lock() -> int | None:
    """Validate and hold the explicitly inherited shared build lock, if supplied."""
    value = os.environ.get("BOARDSTUDIO_BUILD_LOCK_FD")
    if value is None:
        return None
    import fcntl

    lock = Path.home() / ".cache/tmp/boardstudio-build.lock"
    try:
        fd = int(value)
        stat = os.fstat(fd)
        path_stat = lock.stat()
        if fd < 3 or (stat.st_dev, stat.st_ino) != (path_stat.st_dev, path_stat.st_ino):
            raise ValueError("descriptor does not refer to the shared lock")
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except (ValueError, OSError) as error:
        raise SystemExit(f"invalid BOARDSTUDIO_BUILD_LOCK_FD: {error}") from error
    return fd


def _build_harness(binary: str) -> Path:
    """Build the requested harness under the shared lock."""
    root = Path(__file__).resolve().parents[1]
    lock = Path.home() / ".cache/tmp/boardstudio-build.lock"
    lock.parent.mkdir(parents=True, exist_ok=True)
    inherited = _inherited_build_lock()
    if inherited is None and not shutil.which("flock"):
        raise MissingBrowserTool("browser harness needs flock for the shared build lock; install flock (util-linux)")
    if not shutil.which("cargo"):
        raise MissingBrowserTool("browser harness needs Cargo; install Rust and Cargo and add cargo to PATH")
    try:
        result = subprocess.run(
            [*(["flock", str(lock)] if inherited is None else []), "cargo", "build", "--locked", "--message-format=json",
             "--manifest-path", str(root / "tools/browser-harness/Cargo.toml"),
             "--package", "boardstudio-browser-harness", "--bin", binary],
            cwd=root, capture_output=True, text=True, **({"pass_fds": (inherited,)} if inherited is not None else {}),
        )
    except FileNotFoundError as error:
        if error.filename == ("flock" if inherited is None else "cargo"):
            raise MissingBrowserTool(f"browser harness cannot find {error.filename}; install it and add it to PATH") from error
        raise
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
        raise SystemExit(f"{binary} cannot build the Rust harness:\n{diagnostic}")
    try:
        artifacts = [message["executable"] for line in result.stdout.splitlines()
                     if (message := json.loads(line)).get("reason") == "compiler-artifact"
                     and message.get("package_id", "").startswith(
                         f"path+{(root / 'tools/browser-harness').as_uri()}#")
                     and message.get("target", {}).get("name") == binary
                     and "bin" in message["target"].get("kind", []) and message.get("executable")]
    except (ValueError, KeyError, TypeError) as error:
        raise SystemExit(f"{binary} cannot read Cargo build output: {error}") from error
    if len(artifacts) != 1:
        raise SystemExit(f"{binary} cannot identify its executable in Cargo build output")
    return Path(artifacts[0])
