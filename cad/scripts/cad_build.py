"""Pure-Rust Monstertruck CAD WASM build and native test orchestration."""

import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import browser_tools

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
    "src/model/keycaps/print_legend_tests.rs",
})
LIBRARY_TEST_ONLY_SOURCES = frozenset({
    "src/tests.rs",
    "src/export_tests.rs",
    "src/step_accuracy.rs",
    "src/layered/tests.rs",
    "src/layered/pcb_tests.rs",
})
OWNER_TESTS = (
    ("monstertruck-io", "--test", "input", "repair_movement"),
    ("monstertruck-io", "--lib", "repair_movement_sink"),
    ("monstertruck-io", "--test", "input", "source_pcurve_active_view"),
    ("monstertruck-io", "--test", "input", "preview_tolerance"),
    ("monstertruck-meshing", "--lib", "repair_"),
    ("monstertruck-meshing", "--test", "closed_trim_projection"),
    ("monstertruck-meshing", "--lib", "seam_branch_tests"),
)
PROVIDER_TESTS = (
    "step-repair-accounting.cjs",
    "step-zero-repair.cjs",
    "step-projected-line-repair.cjs",
    "step-planar-seam-repair.cjs",
    "step-face-completeness.cjs",
    "step-complete-surface-preview.cjs",
    "collision-analysis.cjs",
    "qualified-analysis.cjs",
)


def run(command, **kwargs):
    fd = browser_tools._inherited_build_lock()
    return subprocess.run(command, cwd=CAD_ROOT, check=True,
                          **({"pass_fds": (fd,)} if fd is not None else {}), **kwargs)


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
        # Kept deliberately: drops output left in wasm/pkg by checkouts from the Cadrum era.
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
    owner_env = dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / "monstertruck-qualification/target"))
    # Filter legacy suites; external fixtures are read only by the tests that use them.
    for package, *selection in OWNER_TESTS:
        try:
            result = run(["cargo", "test", "--manifest-path", "monstertruck-qualification/Cargo.toml",
                          "-p", package, *selection, "--release", "--locked"], env=owner_env,
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        except subprocess.CalledProcessError as error:
            print(error.stdout or "", end="", flush=True)
            raise
        print(result.stdout, end="", flush=True)
        if not re.search(r"^test result: ok\. [1-9][0-9]* passed;", result.stdout, re.M):
            raise RuntimeError(f"CAD owner selection has no executed tests: {package} {' '.join(selection)}")


def test_provider():
    node = shutil.which("node")
    if not node:
        raise RuntimeError("CAD packaged-provider regressions require Node.js")
    package = CAD_ROOT / "wasm/pkg" / f"{OUT_NAME}.js"
    wasm = package.with_name(f"{OUT_NAME}_bg.wasm")
    stamp = CAD_ROOT / ".cache/cad/wasm-build.sha256"
    if not all(path.is_file() for path in (package, wasm, stamp)):
        raise RuntimeError("CAD WEB package is missing; run python3 cad/scripts/build-cad-wasm.py --no-opt")
    if stamp.read_text().strip() not in (wasm_inputs_digest(no_opt=True), wasm_inputs_digest()):
        raise RuntimeError("CAD WEB package is stale; run python3 cad/scripts/build-cad-wasm.py --no-opt")
    for script in PROVIDER_TESTS:
        run([node, str(CAD_ROOT / "wasm/tests" / script), str(package), "--web"])
