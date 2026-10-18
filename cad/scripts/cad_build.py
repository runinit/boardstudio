"""Pure-Rust Monstertruck CAD WASM build and native test orchestration."""

import hashlib
import os
from pathlib import Path
import subprocess

CAD_ROOT = Path(__file__).resolve().parents[1]
OUT_NAME = "boardstudio_cad_wasm"
VENDOR_CRATES = (
    "monstertruck-core",
    "monstertruck-derive",
    "monstertruck-geometry",
    "monstertruck-io",
    "monstertruck-mesh",
    "monstertruck-meshing",
    "monstertruck-modeling",
    "monstertruck-traits",
)
WASM_TEST_ONLY_SOURCES = frozenset({
    "src/model/source_validation_tests.rs",
    "src/model/generated/tests.rs",
})
LIBRARY_TEST_ONLY_SOURCES = frozenset({
    "src/tests.rs",
    "src/export_tests.rs",
    "src/step_accuracy.rs",
    "src/layered/tests.rs",
    "src/layered/pcb_tests.rs",
})


def run(command, **kwargs):
    subprocess.run(command, cwd=CAD_ROOT, check=True, **kwargs)


def _iter_source_files(base: Path, names: tuple[str, ...]):
    for name in names:
        path = base / name
        if not path.exists():
            continue
        files = sorted(path.rglob("*")) if path.is_dir() else [path]
        for file in files:
            if not file.is_file():
                continue
            rel_parts = file.relative_to(base).parts
            if any(part.startswith(".") for part in rel_parts):
                continue
            yield file


def wasm_inputs_digest(*, no_opt=False):
    """Hash everything the pure-Rust WASM provider build reads."""
    contracts = CAD_ROOT.parent / "contracts/rust"
    vendor_root = CAD_ROOT / "monstertruck-qualification/vendor"
    profile = "release-no-opt" if no_opt else "release"
    digest = hashlib.sha256(f"{OUT_NAME}\n{profile}\n".encode())
    digest.update(Path(__file__).read_bytes())
    roots = [
        (CAD_ROOT.parent, ("Cargo.toml", "rust-toolchain.toml")),
        (CAD_ROOT / "wasm", ("Cargo.toml", "Cargo.lock", "src", "assets")),
        (CAD_ROOT / "monstertruck", ("Cargo.toml", "src")),
        (contracts, ("Cargo.toml", "src")),
        (CAD_ROOT.parent / "renderer/vendor/cgmath-next-0.18.1", ("Cargo.toml", "src")),
    ]
    for crate in VENDOR_CRATES:
        roots.append((vendor_root / crate, ("Cargo.toml", "src")))
    for base, names in roots:
        for file in _iter_source_files(base, names):
            rel_to_base = file.relative_to(base).as_posix()
            if base == CAD_ROOT / "wasm" and rel_to_base in WASM_TEST_ONLY_SOURCES:
                continue
            if base == CAD_ROOT / "monstertruck" and (
                rel_to_base in LIBRARY_TEST_ONLY_SOURCES
                or rel_to_base.startswith("src/step_accuracy/")
            ):
                continue
            digest.update(file.relative_to(CAD_ROOT.parent).as_posix().encode() + b"\0")
            digest.update(file.read_bytes())
    return digest.hexdigest()


def build_wasm(*, no_opt=False):
    stamp = CAD_ROOT / ".cache/cad/wasm-build.sha256"
    inputs = wasm_inputs_digest(no_opt=no_opt)
    pkg = CAD_ROOT / "wasm/pkg"
    output = pkg / f"{OUT_NAME}_bg.wasm"
    if output.is_file() and stamp.is_file() and stamp.read_text().strip() == inputs:
        print("cad: wasm/pkg is current; skipping the WASM build", flush=True)
        return
    (pkg / "package.json").unlink(missing_ok=True)
    if pkg.is_dir():
        for stale in pkg.glob("boardstudio_cadrum_wasm*"):
            stale.unlink(missing_ok=True)
    stamp.parent.mkdir(parents=True, exist_ok=True)
    wasm_opt = ["--no-opt"] if no_opt else []
    env = dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / "wasm/target"))
    run(
        [
            "wasm-pack",
            "build",
            "wasm",
            "--target",
            "web",
            "--out-dir",
            "pkg",
            "--out-name",
            OUT_NAME,
            *wasm_opt,
            "--release",
            "--locked",
        ],
        env=env,
    )
    stamp.write_text(inputs + "\n")


def test_cad():
    library_env = dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / "monstertruck/target"))
    run(
        [
            "cargo",
            "test",
            "--manifest-path",
            "monstertruck/Cargo.toml",
            "--release",
            "--locked",
        ],
        env=library_env,
    )
    wasm_env = dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / "wasm/target"))
    run(
        [
            "cargo",
            "test",
            "--manifest-path",
            "wasm/Cargo.toml",
            "--release",
            "--locked",
        ],
        env=wasm_env,
    )
