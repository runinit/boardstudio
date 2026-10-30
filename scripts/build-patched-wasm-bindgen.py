#!/usr/bin/env python3
"""Build an experimental wasm-bindgen pair without changing wasm-pack's cache.

Example (use a spacious job-local directory, not a shared Cargo target):
  python3 scripts/build-patched-wasm-bindgen.py --output ~/.cache/my-job/wasm-bindgen
  BOARDSTUDIO_WASM_BINDGEN_DIR=~/.cache/my-job/wasm-bindgen python3 scripts/check.py browser

Use --offline to require cached crates and dependencies. --crate-dir can point at
previously downloaded .crate archives; otherwise Cargo's registry cache is searched.
The temporary source and target live alongside the output and are removed on failure.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import urllib.request

VERSION = "0.2.129"
CRATES = {
    "wasm-bindgen-cli-0.2.129": "5fd044ed178958a277eeaf21f3d384287b0f4428d38c019141df01da9c03168c",
    "tiny_http-0.12.0": "389915df6413a2e74fb181895f933386023c71110878cd0825588928e64cdc82",
}
BINS = ("wasm-bindgen", "wasm-bindgen-test-runner")
ROOT = Path(__file__).resolve().parent.parent
OLD = "if self.sharing.waiting_tasks.load(Ordering::Acquire) == 0 {"
NEW = "if self.sharing.waiting_tasks.load(Ordering::Acquire) <= queue.len() {"
TINY_LOCK = '''name = "tiny_http"
version = "0.12.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "389915df6413a2e74fb181895f933386023c71110878cd0825588928e64cdc82"'''


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(output, crate_dir, offline, workspace_lock):
    output = Path(output).absolute()
    if output.exists() or output.is_symlink():
        raise ValueError(f"output already exists: {output}")
    if any(c.isspace() or c == os.pathsep for c in str(output.resolve())):
        raise ValueError("output path must not contain whitespace or PATH separators")
    versions = re.findall(r'name = "wasm-bindgen"\nversion = "([^"]+)"', Path(workspace_lock).read_text())
    if versions != [VERSION]:
        raise ValueError(f"workspace wasm-bindgen version must be exactly {VERSION}: {versions}")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".wasm-bindgen-build-", dir=output.parent) as tmp:
        work = Path(tmp)
        for name, expected in CRATES.items():
            candidates = ([Path(crate_dir) / (name + ".crate")] if crate_dir else
                          sorted((Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "registry/cache").glob("*/" + name + ".crate")))
            archive = next((p for p in candidates if p.is_file()), None)
            if archive is None:
                if offline:
                    raise ValueError(f"offline crate archive not found: {name}")
                archive = work / (name + ".crate")
                urllib.request.urlretrieve(f"https://static.crates.io/crates/{name.rsplit('-', 1)[0]}/{name}.crate", archive)
            if digest(archive) != expected:
                raise ValueError(f"crate checksum mismatch: {archive}")
            with tarfile.open(archive, "r:gz") as tar:
                tar.extractall(work, filter="data")
        source = work / "wasm-bindgen-cli-0.2.129"
        tiny = work / "tiny_http-0.12.0"
        task_pool = tiny / "src/util/task_pool.rs"
        text = task_pool.read_text()
        if text.count(OLD) != 1:
            raise ValueError("tiny_http patch did not match exactly once")
        task_pool.write_text(text.replace(OLD, NEW))
        lock = source / "Cargo.lock"
        text = lock.read_text()
        if text.count(TINY_LOCK) != 1:
            raise ValueError("upstream lockfile tiny_http entry did not match")
        # A path-patched package has no registry source/checksum; all other pins stay intact.
        lock.write_text(text.replace(TINY_LOCK, 'name = "tiny_http"\nversion = "0.12.0"'))
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(work / "target")
        env["RUSTUP_TOOLCHAIN"] = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        command = ["cargo", "--config", str(ROOT / ".cargo/config.toml"), "build", "--release", "--locked"]
        if offline:
            command.append("--offline")
        command += ["--config", f"patch.crates-io.tiny_http.path={json.dumps(str(tiny))}"]
        for name in BINS:
            command += ["--bin", name]
        subprocess.run(command, cwd=source, env=env, check=True)
        pair = work / "pair"
        pair.mkdir()
        for name in BINS:
            binary = work / "target/release" / name
            result = subprocess.run([str(binary), "--version"], capture_output=True, text=True, check=True)
            if result.stdout.strip() != f"{name} {VERSION}":
                raise ValueError(f"unexpected version from {binary}: {result.stdout.strip()!r}")
            shutil.copy2(binary, pair / name)
        pair.rename(output)
    print(f"built patched pair: {output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="new directory for the two binaries")
    parser.add_argument("--crate-dir", type=Path, help="directory containing both .crate archives")
    parser.add_argument("--offline", action="store_true", help="do not download crates or resolve dependencies online")
    args = parser.parse_args()
    try:
        build(args.output, args.crate_dir, args.offline, Path(__file__).resolve().parent.parent / "Cargo.lock")
    except (OSError, ValueError, subprocess.SubprocessError, tarfile.TarError) as error:
        print(f"build-patched-wasm-bindgen: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
