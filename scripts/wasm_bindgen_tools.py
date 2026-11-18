"""Opt-in wasm-pack test tool selection; the default remains wasm-pack's own selection."""

import os
from pathlib import Path
import re
import subprocess
import sys

DIRECTORY_ENV = "BOARDSTUDIO_WASM_BINDGEN_DIR"
TEST_RUNNER = "wasm-bindgen-test-runner"


def locked_version(root):
    lock = (Path(root) / "Cargo.lock").read_text()
    versions = re.findall(r'name = "wasm-bindgen"\nversion = "([^"]+)"', lock)
    if len(versions) != 1:
        raise ValueError("Cargo.lock must contain exactly one wasm-bindgen version")
    return versions[0]


def validate_pair(directory, version):
    for name in ("wasm-bindgen", TEST_RUNNER):
        path = directory / name
        result = subprocess.run([str(path), "--version"], capture_output=True, text=True, check=True)
        if result.stdout.strip() != f"{name} {version}":
            raise ValueError(f"{path} version {result.stdout.strip()!r}; expected {name} {version}")


def configure(env, root):
    """Prepend a validated pair so wasm-pack 0.15 selects its sibling runner."""
    if DIRECTORY_ENV not in env:
        return env
    try:
        if not env[DIRECTORY_ENV]:
            raise ValueError("directory is empty")
        directory = Path(env[DIRECTORY_ENV]).resolve()
        if os.pathsep in str(directory):
            raise ValueError(f"directory path contains a path separator: {directory}")
        # wasm-pack passes the runner path to Cargo as an unquoted runner string.
        if any(char.isspace() for char in str(directory)):
            raise ValueError(f"directory path contains whitespace: {directory}")
        version = locked_version(root)
        validate_pair(directory, version)
    except (OSError, subprocess.SubprocessError, ValueError) as error:
        raise ValueError(f"{DIRECTORY_ENV}: invalid wasm-bindgen pair: {error}") from error
    env = env.copy()
    env[DIRECTORY_ENV] = str(directory)
    env["PATH"] = str(directory) + os.pathsep + env.get("PATH", "")
    print(f"wasm-bindgen pair: {directory} (version {version})", file=sys.stderr)
    return env
