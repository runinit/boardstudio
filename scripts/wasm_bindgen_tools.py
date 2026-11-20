"""Select and validate the wasm-bindgen tools used by browser tests."""

from contextlib import contextmanager
import fcntl
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys

import browser_tools

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


@contextmanager
def _pair_build_lock():
    if (fd := browser_tools._inherited_build_lock()) is not None:
        yield fd
        return
    path = Path.home() / ".cache/tmp/boardstudio-build.lock"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield lock.fileno()


def _cached_pair(directory, version):
    if not directory.exists() and not directory.is_symlink():
        return False
    try:
        validate_pair(directory, version)
    except (OSError, subprocess.SubprocessError, ValueError) as error:
        raise ValueError(f"invalid cached wasm-bindgen pair at {directory}: {error}") from error
    return True


def local_patched_pair(cache, root, version):
    """Return a validated local pair, or None if its build inputs cannot be fetched."""
    root = Path(root).resolve()
    builder = [sys.executable, "scripts/build-patched-wasm-bindgen.py"]
    env = os.environ.copy()
    # A cold-run logger wraps workspace compiles, not this independently cached
    # host tool. Its per-worktree path must not produce a different cache key.
    if env.get("RUSTC_WRAPPER") == str(root / "scripts/build-time/rustc-log.py"):
        env.pop("RUSTC_WRAPPER")
    result = subprocess.run([*builder, "--cache-key"], cwd=root, env=env,
                            capture_output=True, text=True)
    key = result.stdout.strip()
    if result.returncode or not re.fullmatch(r"patched-wasm-bindgen-v1-[0-9a-f]{64}", key):
        raise ValueError(f"cannot compute patched wasm-bindgen cache key: {result.stderr.strip()}")
    directory = (cache / "boardstudio/wasm-bindgen-patched" / key).resolve()
    if _cached_pair(directory, version):
        return directory
    with _pair_build_lock() as fd:
        if _cached_pair(directory, version):
            return directory
        command = [*builder, "--output", str(directory)]
        modes = [True] if env.get("CARGO_NET_OFFLINE", "").lower() == "true" else [True, False]
        for offline in modes:
            result = subprocess.run([*command, *(["--offline"] if offline else [])],
                                    cwd=root, env=env, pass_fds=(fd,), capture_output=True, text=True)
            # The builder installs by renaming a validated pair from a sibling
            # temporary directory. A concurrent install may win that rename.
            if _cached_pair(directory, version):
                return directory
            unavailable = (
                ("offline crate archive not found:", "no matching package named", "attempting to make an HTTP request")
                if offline else
                ("<urlopen error", "HTTP Error ", "failed to download", "download of config.json failed",
                 "error sending request")
            )
            if not result.returncode or not any(marker in result.stderr for marker in unavailable):
                raise ValueError(f"patched wasm-bindgen build failed: {result.stderr.strip()}")
    print("warning: patched wasm-bindgen inputs are unavailable. Using the stock pair, "
          "which can stall for about 270 seconds. Build the patched pair under the build lock "
          f"when online with `{shlex.join(command)}`.", file=sys.stderr)
    return None


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
