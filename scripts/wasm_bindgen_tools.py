"""Select and validate the wasm-bindgen tools used by browser tests."""

from contextlib import contextmanager
import fcntl
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tomllib

import browser_tools

DIRECTORY_ENV = "BOARDSTUDIO_WASM_BINDGEN_DIR"
TEST_RUNNER = "wasm-bindgen-test-runner"


def test_build_env(environment, root):
    env = dict(environment, CARGO_TARGET_DIR=str(Path(root) / "target/wasm-tests"))
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    return env


def locked_version(root, *, lock_path=None):
    with Path(lock_path or Path(root) / "Cargo.lock").open("rb") as lock:
        packages = tomllib.load(lock).get("package", [])
    if not isinstance(packages, list) or any(not isinstance(package, dict) for package in packages):
        raise ValueError("workspace lock must contain Cargo package entries")
    versions = [package.get("version") for package in packages if package.get("name") == "wasm-bindgen"]
    if len(versions) != 1 or not isinstance(versions[0], str):
        raise ValueError("Cargo.lock must contain exactly one wasm-bindgen version")
    return versions[0]


def validate_binary(path, version, name="wasm-bindgen"):
    result = subprocess.run([str(path), "--version"], capture_output=True, text=True, check=True)
    if result.stdout.strip() != f"{name} {version}":
        raise ValueError(f"{path} version {result.stdout.strip()!r}; expected {name} {version}")


def find_bindgen(root, extra=()):
    version = locked_version(root)
    candidates = []
    if directory := os.environ.get(DIRECTORY_ENV):
        candidates.append(Path(directory) / "wasm-bindgen")
    else:
        cache = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
        try:
            # Planning callers (`overlap_span`, `page_tools`) must fall back instead of raising.
            if patched := local_patched_pair(cache, root, version):
                candidates.append(patched / "wasm-bindgen")
        except (OSError, subprocess.SubprocessError, ValueError):
            pass
    if found := shutil.which("wasm-bindgen"):
        candidates.append(Path(found))
    candidates.extend(extra)
    cache = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    candidates.extend(sorted(cache.glob(".wasm-pack/wasm-bindgen-*/wasm-bindgen")))
    for path in candidates:
        try:
            validate_binary(path, version)
            return path
        except (OSError, subprocess.SubprocessError, ValueError):
            continue
    return None


def validate_pair(directory, version):
    for name in ("wasm-bindgen", TEST_RUNNER):
        validate_binary(directory / name, version, name)


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


def find_pair(root):
    try:
        if DIRECTORY_ENV in os.environ:
            env = configure(os.environ, root)
            return Path(env[DIRECTORY_ENV])
        version = locked_version(root)
        cache = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
        if patched := local_patched_pair(cache, root, version):
            return patched
    except (OSError, ValueError) as error:
        raise ValueError(str(error)) from error
    cache /= ".wasm-pack"
    candidates = [path.parent for path in sorted(cache.glob(
        f"wasm-bindgen-*/{TEST_RUNNER}"))]
    if bindgen := shutil.which("wasm-bindgen"):
        candidates.insert(0, Path(bindgen).absolute().parent)
    for directory in candidates:
        try:
            validate_pair(directory, version)
        except (OSError, subprocess.SubprocessError, ValueError):
            continue
        return directory
    raise ValueError(f"no wasm-bindgen pair at locked version {version} on PATH or in {cache}; "
                      f"run `cargo install wasm-bindgen-cli --locked --version {version}` "
                      "or set BOARDSTUDIO_WASM_BINDGEN_DIR to a matching pair")
