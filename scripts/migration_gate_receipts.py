"""Exact-input success receipts for the migration delivery gates.

Receipts are local build cache entries, never acceptance records. Their identity
includes the maintained source/config inventory and the exact gate invocation.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import re
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import time
from typing import Any, Callable

SCHEMA = 1
CACHE_RELATIVE = Path("web/target/migration-gates")
ENVIRONMENT_KEYS = (
    "HOME", "CARGO_HOME", "RUSTUP_HOME", "CARGO_TARGET_DIR", "CARGO_ENCODED_RUSTFLAGS", "RUSTFLAGS",
    "RUSTDOCFLAGS", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
    "PATH", "TMPDIR", "CC", "CXX", "CFLAGS", "CXXFLAGS", "AR", "PKG_CONFIG",
    "PKG_CONFIG_PATH", "LIBCLANG_PATH", "BINDGEN_EXTRA_CLANG_ARGS", "CHROME_BIN",
    "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH",
    "SSL_CERT_FILE", "CARGO_HTTP_PROXY", "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY",
    "RUSTUP_TOOLCHAIN", "BOARDSTUDIO_WASM_TEST_COMMAND",
    "BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL", "WASM_BINDGEN_TEST_WEBDRIVER_JSON",
    "CHROMEDRIVER",
)
GENERATED_PARTS = {".git", "node_modules", "target", "dist", ".generated", ".cache"}


def _digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _regular_file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _file_digest(path: Path, root: Path | None = None) -> str:
    """Hash bytes consumed through a path; reject dangling or escaping source links."""
    path = Path(path)
    if not path.is_symlink():
        if not path.is_file():
            raise ValueError(f"maintained input is not a regular file: {path}")
        return _regular_file_digest(path)
    try:
        resolved = path.resolve(strict=True)
    except OSError as error:
        raise ValueError(f"unresolved maintained input symlink: {path}: {error}") from error
    if root is not None:
        try:
            resolved.relative_to(Path(root).resolve())
        except ValueError as error:
            raise ValueError(f"maintained input symlink escapes the repository: {path}") from error
    if not resolved.is_file():
        raise ValueError(f"maintained input symlink does not target a regular file: {path}")
    material = (b"symlink\0" + os.readlink(path).encode() + b"\0"
                + str(resolved).encode() + b"\0" + bytes.fromhex(_regular_file_digest(resolved)))
    return _digest(material)


def _inventory(root: Path) -> dict[str, str]:
    """Hash maintained files, including nonignored local build inputs.

    `.scratch` contains mutable evidence and planning records rather than compiler
    inputs. Generated caches and package output are excluded; all other tracked or
    nonignored untracked files participate conservatively.
    """
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root, text=True, capture_output=True, check=True,
    )
    paths = set(result.stdout.split("\0"))
    deleted_result = subprocess.run(
        ["git", "diff", "--cached", "--diff-filter=D", "--name-only", "-z"],
        cwd=root, text=True, capture_output=True, check=True,
    )
    paths.update(deleted_result.stdout.split("\0"))
    inventory: dict[str, str] = {}
    for name in sorted(path for path in paths if path):
        rel = Path(name)
        if rel.parts[0] == ".scratch" or any(part in GENERATED_PARTS for part in rel.parts):
            continue
        path = root / rel
        if path.is_file() or path.is_symlink():
            inventory[rel.as_posix()] = _file_digest(path, root)
        else:
            inventory[rel.as_posix()] = "missing"
    stage_result = subprocess.run(["git", "ls-files", "--stage", "-z"], cwd=root,
                                  text=True, capture_output=True, check=True)
    indexed: dict[str, list[str]] = {}
    for record in stage_result.stdout.split("\0"):
        if not record:
            continue
        metadata, name = record.split("\t", 1)
        mode, blob, stage = metadata.split()
        if name in inventory:
            indexed.setdefault(name, []).append(f"{mode} {blob} {stage}")
    for name in deleted_result.stdout.split("\0"):
        if name and name in inventory:
            indexed[name] = []
    inventory.update({f"@index:{name}": ";".join(entries)
                      for name, entries in sorted(indexed.items())})
    return inventory


BUILD_ROOT_INPUTS = frozenset({
    "Cargo.toml", ".cargo/config", ".cargo/config.toml", ".node-version", ".npmrc",
    "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", "rust-toolchain.toml",
})
BUILD_PREFIXES = (
    "web/", "application/", "core/", "contracts/", "renderer/", "cad/",
    "kicad/src/", "app/src/", "app/public/", "scripts/", "ergogen/",
)
GENERATED_INPUT_PREFIXES = (
    "web/target/", "core/pkg/", "cad/wasm/pkg/", "cad/.cache/", "renderer/pkg/",
    "web/assets/ergogen-models/", "web/assets/layout-generators/",
    "web/assets/preview-generator/", "web/assets/layout-generators.js",
)


def _build_source_names(root: Path) -> set[str]:
    """Use build-m1's maintained-input selector, including its test/build controls."""
    helper_path = root / "scripts/build-m1.py"
    if helper_path.is_file():
        import importlib.util
        spec = importlib.util.spec_from_file_location("migration_receipt_build_inputs", helper_path)
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        helper.REPO = root
        tracked = subprocess.run(
            ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
            cwd=root, capture_output=True, check=True,
        ).stdout.decode().split("\0")
        ignored = subprocess.run(
            ["git", "ls-files", "--others", "--ignored", "--exclude-standard", "-z"],
            cwd=root, capture_output=True, check=True,
        ).stdout.decode().split("\0")
        names = {name for name in (*tracked, *ignored) if name and helper.is_build_source_path(name)}
        vendor = root / "ergogen/library/vendor"
        if vendor.is_dir():
            names.update(
                path.relative_to(root).as_posix()
                for path in vendor.glob("*/3d_models/**/*")
                if path.is_file() and path.suffix.lower() in {".step", ".stp", ".stl", ".wrl"}
            )
        return names

    # Small isolated receipt fixtures do not carry the repository build helper.
    listed = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root, capture_output=True, check=True,
    ).stdout.decode().split("\0")
    selected = set()
    for name in listed:
        if not name or name.startswith((".scratch/", "docs/", "web/target/")):
            continue
        if name in BUILD_ROOT_INPUTS or name.startswith(BUILD_PREFIXES):
            selected.add(name)
        elif "/" not in name and Path(name).suffix.lower() in {
            ".rs", ".toml", ".json", ".js", ".mjs", ".ts", ".tsx", ".py",
        }:
            selected.add(name)
    return selected


def is_maintained_build_input(root: Path, name: str) -> bool:
    """Whether a path is in the maintained input closure used for receipts."""
    root = Path(root).resolve()
    helper_path = root / "scripts/build-m1.py"
    if helper_path.is_file():
        import importlib.util
        spec = importlib.util.spec_from_file_location("migration_receipt_build_filter", helper_path)
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        helper.REPO = root
        return bool(helper.is_build_source_path(name))
    return (name in BUILD_ROOT_INPUTS or name.startswith(BUILD_PREFIXES)
            or ("/" not in name and Path(name).suffix.lower() in {
                ".rs", ".toml", ".json", ".js", ".mjs", ".ts", ".tsx", ".py",
            }))


def _executable_inventory(root: Path, extra_inputs=()) -> dict[str, str]:
    root = Path(root).resolve()
    names = _build_source_names(root) | set(extra_inputs)
    inventory = {}
    for name in sorted(names):
        path = root / name
        if path.is_symlink():
            inventory[name] = _file_digest(path, root)
        elif path.is_file():
            inventory[name] = _file_digest(path, root)
        else:
            inventory[name] = "missing"
    # Staged versions of maintained files are what commit will publish. Include
    # only their index identities; unrelated staged docs/index churn is omitted.
    selected = set(names)
    stage_result = subprocess.run(["git", "ls-files", "--stage", "-z"], cwd=root,
                                  capture_output=True, check=True)
    for record in stage_result.stdout.decode().split("\0"):
        if not record:
            continue
        metadata, name = record.split("\t", 1)
        if name in selected:
            inventory[f"@index:{name}"] = metadata
    deleted = subprocess.run(["git", "diff", "--cached", "--diff-filter=D", "--name-only", "-z"],
                             cwd=root, capture_output=True, check=True).stdout.decode().split("\0")
    for name in deleted:
        if name in selected:
            inventory[f"@index:{name}"] = "missing"
    return inventory


def _executable(name: str, env: dict[str, str], root: Path) -> str | None:
    candidate = Path(name).expanduser()
    if candidate.is_absolute():
        return str(candidate.resolve()) if candidate.exists() else None
    if candidate.parent != Path("."):
        candidate = root / candidate
        return str(candidate.resolve()) if candidate.exists() else None
    return shutil.which(name, path=env.get("PATH"))


def _tool_identity(command: list[str], env: dict[str, str], root: Path, extra_tools=(), cache=None) -> list[dict[str, str]]:
    cache = cache if cache is not None else {}
    names = []
    runner_override = env.get("BOARDSTUDIO_WASM_TEST_COMMAND")
    if runner_override:
        names.extend(shlex.split(runner_override)[:1])
    for key in ("CHROMEDRIVER", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CC", "CXX"):
        if env.get(key):
            names.extend(shlex.split(env[key])[:1])
    if command:
        names.append(command[0])
    names.extend(extra_tools)
    if Path(command[0]).name == "cargo" or "cargo" in extra_tools:
        names.extend(("rustc", "rustup", "wasm-ld", "clang", "cc", "ld"))
    tools = []
    seen = set()
    for name in names:
        resolved = _executable(name, env, root)
        key = (name, resolved)
        if key in seen:
            continue
        seen.add(key)
        if key not in cache:
            version = ""
            binary_digest = ""
            if resolved:
                path = Path(resolved)
                try:
                    binary_digest = _file_digest(path)
                except (OSError, ValueError):
                    binary_digest = ""
                try:
                    with path.open("rb") as executable_file:
                        script_file = executable_file.read(2) == b"#!"
                except OSError:
                    script_file = False
                for flag in (() if script_file else ("--version", "-V", "version")):
                    try:
                        probe = subprocess.run([name, flag], cwd=root, env=env, text=True,
                                               capture_output=True, timeout=5)
                    except (OSError, subprocess.SubprocessError):
                        continue
                    text = (probe.stdout + "\n" + probe.stderr).strip()
                    if text:
                        version = text[:2000]
                        break
            cache[key] = {"command": name, "path": resolved or "", "sha256": binary_digest,
                          "version": version}
        tools.append(cache[key])

    if Path(command[0]).name == "cargo" or "cargo" in extra_tools:
        tools.append(_selected_rustc_identity(root, env, cache))
    return tools


def _selected_rustc_identity(root: Path, env: dict[str, str], cache) -> dict[str, str]:
    command = shlex.split(env.get("RUSTC", "rustc"))[:1] or ["rustc"]
    name = command[0]
    resolved = _executable(name, env, root)
    key = ("selected-rustc", resolved)
    if key in cache:
        return cache[key]
    record = {"command": "selected-rustc", "path": "", "sysroot": "", "sha256": "", "version": ""}
    if resolved:
        try:
            probe = subprocess.run([name, "--print", "sysroot"], cwd=root, env=env, text=True,
                                   capture_output=True, timeout=5)
            sysroot = Path(probe.stdout.strip()).resolve() if probe.returncode == 0 else None
            binary = sysroot / "bin/rustc" if sysroot else None
            if binary and binary.is_file():
                record["path"] = str(binary.resolve())
                record["sysroot"] = str(sysroot)
                record["sha256"] = _file_digest(binary)
                version = subprocess.run([name, "--version"], cwd=root, env=env, text=True,
                                         capture_output=True, timeout=5)
                record["version"] = (version.stdout + "\n" + version.stderr).strip()[:2000]
        except (OSError, ValueError, subprocess.SubprocessError):
            pass
    cache[key] = record
    return record


def _logical_command(command: list[str]) -> list[str]:
    """Remove the ephemeral result JSON location while preserving runner semantics."""
    output = []
    skip = False
    for token in command:
        if skip:
            output.append("<result-json>")
            skip = False
        elif token == "--result-json":
            output.append(token)
            skip = True
        else:
            output.append(token)
    return output


def make_identity(root: Path, gate: str, command: list[str], *, selected_files=(),
                  env: dict[str, str] | None = None, extra_tools=(), extra_inputs=(),
                  tool_cache=None) -> dict[str, Any]:
    root = Path(root).resolve()
    env = dict(os.environ if env is None else env)
    # Gate receipts and commit drift checks cover the same maintained build/test
    # inputs, including their staged identities; unrelated records are excluded.
    inputs = _executable_inventory(root, extra_inputs)
    external_inputs = {}
    home = Path(env.get("HOME", str(Path.home()))).expanduser()
    if not home.is_absolute():
        home = root / home
    cargo_home = Path(env.get("CARGO_HOME") or (home / ".cargo")).expanduser()
    if not cargo_home.is_absolute():
        cargo_home = root / cargo_home
    for path in (cargo_home / "config", cargo_home / "config.toml"):
        if path.is_file():
            external_inputs[str(path.resolve())] = _file_digest(path)
    webdriver_config = env.get("WASM_BINDGEN_TEST_WEBDRIVER_JSON")
    chromedriver = env.get("CHROMEDRIVER")
    for value in (webdriver_config, chromedriver):
        if value:
            path = Path(value).expanduser()
            if not path.is_absolute():
                path = root / path
            if path.is_file():
                external_inputs[str(path.resolve())] = _file_digest(path)
    relevant_env = sorted(key for key in env if key in ENVIRONMENT_KEYS or key.startswith(
        ("CARGO_", "RUST_", "WASM_BINDGEN_", "BOARDSTUDIO_")))
    env_hashes = {key: _digest(env[key].encode()) for key in relevant_env}
    for key in ENVIRONMENT_KEYS:
        env_hashes.setdefault(key, None)
    tools = list(extra_tools)
    if command and Path(command[0]).name == "cargo":
        tools.extend(("rustc", "rustup", "wasm-ld", "clang", "cc", "ld"))
    if gate == "wasm-headless-tests" and not env.get("BOARDSTUDIO_WASM_TEST_COMMAND"):
        runner_script = root / "scripts/run-wasm-tests.py"
        if runner_script.is_file():
            import importlib.util
            spec = importlib.util.spec_from_file_location("migration_receipt_wasm_runner", runner_script)
            runner = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(runner)
            try:
                bindgen_dir = Path(runner.wasm_bindgen_dir(root))
                tools.extend((str(bindgen_dir / "wasm-bindgen"),
                              str(bindgen_dir / "wasm-bindgen-test-runner")))
            except runner.RunnerError:
                pass
    if gate == "wasm-headless-tests":
        # The Python test runner delegates compilation/execution to cargo and
        # rustc, so those tools must participate even though command[0] is Python.
        tools.append("cargo")
    identity = {
        "schema": SCHEMA,
        "gate": gate,
        "root": str(root),
        "command": _logical_command(list(command)),
        "selected_files": sorted(set(selected_files)),
        "inputs": inputs,
        "environment_sha256": env_hashes,
        "external_inputs_sha256": external_inputs,
        "tools": _tool_identity(list(command), env, root, tools, cache=tool_cache),
    }
    canonical = json.dumps(identity, sort_keys=True, separators=(",", ":")).encode()
    identity["key"] = _digest(canonical)
    return identity


def cache_directory(root: Path) -> Path:
    return Path(root).resolve() / CACHE_RELATIVE


def source_fingerprint(root: Path) -> str:
    """Detect maintained source/index/config drift without blocking record edits."""
    return _digest(json.dumps(_executable_inventory(Path(root).resolve()), sort_keys=True,
                              separators=(",", ":")).encode())


def _receipt_path(root: Path, identity: dict[str, Any]) -> Path:
    return cache_directory(root) / identity["gate"] / f"{identity['key']}.json"


def lookup(root: Path, identity: dict[str, Any], *, require_positive=False) -> dict[str, Any] | None:
    path = _receipt_path(root, identity)
    try:
        receipt = json.loads(path.read_text())
    except (FileNotFoundError, OSError, json.JSONDecodeError):
        return None
    if receipt.get("schema") != SCHEMA or receipt.get("identity") != identity:
        return None
    if receipt.get("status") != "success" or receipt.get("complete") is not True:
        return None
    timestamps = (receipt.get("started_unix"), receipt.get("ended_unix"), receipt.get("duration_seconds"))
    if not all(isinstance(value, (int, float)) and math.isfinite(value) for value in timestamps):
        return None
    if timestamps[1] < timestamps[0] or timestamps[2] < 0:
        return None
    if not isinstance(receipt.get("log_sha256"), str) or not re.fullmatch(r"[0-9a-f]{64}", receipt["log_sha256"]):
        return None
    counts = receipt.get("counts")
    if not isinstance(counts, dict) or any(type(value) is not int or value < 0 for value in counts.values()):
        return None
    if require_positive and counts.get("passed", 0) <= 0:
        return None
    if counts.get("failed", 0) != 0 or counts.get("excluded_failures", 0) != 0:
        return None
    if counts.get("incomplete", 0) != 0 or ("passed" in counts and counts.get("passed", 0) <= 0):
        return None
    return receipt


def record_success(root: Path, identity: dict[str, Any], *, started: float, ended: float,
                   counts: dict[str, int] | None = None, log_digest: str = "",
                   details: dict[str, Any] | None = None, complete=True) -> dict[str, Any]:
    counts = counts or {}
    if not isinstance(log_digest, str) or not re.fullmatch(r"[0-9a-f]{64}", log_digest):
        raise ValueError("successful receipts require a SHA-256 log digest")
    if (not isinstance(started, (int, float)) or not isinstance(ended, (int, float))
            or not math.isfinite(started) or not math.isfinite(ended) or ended < started):
        raise ValueError("receipt timestamps must be finite and ordered")
    if any(type(value) is not int or value < 0 for value in counts.values()):
        raise ValueError("receipt counts must be nonnegative integers")
    if (not complete or counts.get("failed", 0) or counts.get("excluded_failures", 0)
            or counts.get("incomplete", 0) or ("passed" in counts and counts.get("passed", 0) <= 0)):
        raise ValueError("only complete results without failures can be stored as reusable receipts")
    receipt = {
        "schema": SCHEMA,
        "identity": identity,
        "status": "success",
        "complete": True,
        "started_unix": started,
        "ended_unix": ended,
        "duration_seconds": max(0.0, ended - started),
        "counts": counts,
        "log_sha256": log_digest,
        "details": details or {},
    }
    path = _receipt_path(root, identity)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent, text=True)
    try:
        with os.fdopen(fd, "w") as target:
            json.dump(receipt, target, sort_keys=True, indent=2)
            target.write("\n")
            target.flush()
            os.fsync(target.fileno())
        os.replace(temporary, path)
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
    return receipt


def run_cached(root: Path, gate: str, command: list[str], *, env=None, selected_files=(),
               extra_tools=(), extra_inputs=(), result_parser: Callable[[str], dict[str, Any]] | None = None,
               require_positive=False, tool_cache=None) -> tuple[dict[str, Any], bool, subprocess.CompletedProcess]:
    """Reuse or execute one gate. Failed, empty, or incomplete results are never cached."""
    root = Path(root).resolve()
    child_env = dict(os.environ if env is None else env)
    identity = make_identity(root, gate, command, selected_files=selected_files,
                             env=child_env, extra_tools=extra_tools, extra_inputs=extra_inputs,
                             tool_cache=tool_cache)
    prior = lookup(root, identity, require_positive=require_positive)
    if prior:
        return prior, True, subprocess.CompletedProcess(command, 0, "", "")
    started = time.time()
    started_mono = time.monotonic()
    process = subprocess.run(command, cwd=root, env=child_env, text=True, capture_output=True)
    output = process.stdout + "\n" + process.stderr
    if process.returncode != 0:
        return {"started_unix": started, "ended_unix": time.time(),
                "duration_seconds": time.monotonic() - started_mono,
                "counts": {}, "complete": False}, False, process
    parsed = result_parser(output) if result_parser else {"counts": {}, "complete": True}
    if not parsed.get("complete"):
        return {"started_unix": started, "ended_unix": time.time(),
                "duration_seconds": time.monotonic() - started_mono,
                **parsed}, False, process
    counts = parsed.get("counts", {})
    if counts.get("failed", 0) or counts.get("excluded_failures", 0) or counts.get("incomplete", 0):
        return {"started_unix": started, "ended_unix": time.time(),
                "duration_seconds": time.monotonic() - started_mono,
                **parsed}, False, process
    if require_positive and counts.get("passed", 0) <= 0:
        return {"started_unix": started, "ended_unix": time.time(),
                "duration_seconds": time.monotonic() - started_mono,
                **parsed}, False, process
    ended = time.time()
    after = make_identity(root, gate, command, selected_files=selected_files,
                          env=child_env, extra_tools=extra_tools, extra_inputs=extra_inputs,
                          tool_cache=tool_cache)
    if after != identity:
        return {"started_unix": started, "ended_unix": ended,
                "duration_seconds": ended - started, "complete": False,
                "counts": counts, "source_drift": True}, False, process
    digest = _digest(output.encode(errors="replace"))
    receipt = record_success(root, identity, started=started, ended=ended,
                             counts=counts, log_digest=digest, details=parsed,
                             complete=bool(parsed.get("complete")))
    return receipt, False, process
