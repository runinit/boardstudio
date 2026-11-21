"""Pure-Rust Monstertruck CAD WASM build and native test orchestration."""

from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import browser_tools

CAD_ROOT = Path(__file__).resolve().parents[1]
OUT_NAME = "boardstudio_cad_wasm"
WASM_BUILD_TARGET = "wasm/target"
NATIVE_TEST_TARGETS = ("target",)
VENDOR_CRATES = (
    "cadmpeg-tessellate",
    "inflector",
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
PRODUCTION_PACKAGES = ("boardstudio-cad", "boardstudio-cad-wasm")


def run(command, **kwargs):
    fd = browser_tools._inherited_build_lock()
    kwargs.setdefault("cwd", CAD_ROOT)
    return subprocess.run(command, check=True,
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
    vendor_root = CAD_ROOT / "vendor"
    profile = "release-no-opt" if no_opt else "release"
    digest = hashlib.sha256(f"{OUT_NAME}\n{profile}\n".encode())
    digest.update(Path(__file__).read_bytes())
    roots = [
        (CAD_ROOT.parent, ("Cargo.toml", "rust-toolchain.toml")),
        (CAD_ROOT, ("Cargo.toml", "Cargo.lock")),
        (CAD_ROOT / "wasm", ("Cargo.toml", "src", "assets")),
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
    env = dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / WASM_BUILD_TARGET))
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


def _artifact_kind(kinds):
    if any(kind in ("lib", "rlib", "cdylib") for kind in kinds):
        return "lib"
    if "test" in kinds:
        return "test"
    if "example" in kinds:
        return "example"
    return None


def _production_targets():
    result = run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--manifest-path", "Cargo.toml"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    packages = {pkg["name"]: pkg for pkg in json.loads(result.stdout)["packages"]}
    targets = []
    for name in PRODUCTION_PACKAGES:
        pkg = packages[name]
        pkg_dir = Path(pkg["manifest_path"]).parent
        for target in pkg["targets"]:
            kind = _artifact_kind(target["kind"])
            if kind == "example" or (kind is not None and target.get("test", False)):
                targets.append((name, kind, target["name"], pkg_dir))
    return targets


def _prebuild(command, env, output, expected, binaries, duplicates):
    fd = browser_tools._inherited_build_lock()
    with subprocess.Popen(command, cwd=CAD_ROOT, env=env, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, text=True,
                          **({"pass_fds": (fd,)} if fd is not None else {})) as process:
        for line in process.stdout:
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                output.write(line)
                continue
            if message.get("reason") == "compiler-message":
                output.write(message["message"].get("rendered") or "")
            if (message.get("reason") != "compiler-artifact"
                    or not message.get("executable")
                    or not message.get("profile", {}).get("test")):
                continue
            target = message["target"]
            kind = _artifact_kind(target["kind"])
            package_id = message["package_id"]
            fragment = package_id.rsplit("#", 1)[-1]
            package = (fragment.split("@", 1)[0] if "@" in fragment
                       else package_id.split("#", 1)[0].rstrip("/").rsplit("/", 1)[-1])
            key = (package, kind, target["name"])
            if key in expected:
                if key in binaries:
                    duplicates.add(key)
                binaries[key] = message["executable"]
        status = process.wait()
    if status:
        raise subprocess.CalledProcessError(status, command)


def _test_cad_unified(output):
    (native_target,) = NATIVE_TEST_TARGETS
    env = dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / native_target))
    production_targets = _production_targets()
    production_tests = sorted({name for _, kind, name, _ in production_targets if kind == "test"})
    example_names = sorted({name for _, kind, name, _ in production_targets if kind == "example"})
    production_expected = {(pkg, kind, name) for pkg, kind, name, _ in production_targets}
    # Production suites need monstertruck-io/-meshing optimized at runtime
    # (cad-test; see cad/Cargo.toml).
    command = [
        "cargo", "test", "--no-run", "--manifest-path", "Cargo.toml",
        *(arg for pkg in PRODUCTION_PACKAGES for arg in ("-p", pkg)),
        "--lib",
        *(arg for name in production_tests for arg in ("--test", name)),
        *(arg for name in example_names for arg in ("--example", name)),
        "--profile", "cad-test", "--locked", "--message-format=json",
    ]
    binaries = {}
    duplicates = set()
    _prebuild(command, env, output, production_expected, binaries, duplicates)
    if duplicates:
        raise RuntimeError(f"duplicate CAD test binaries: {sorted(duplicates)}")
    if missing := production_expected - binaries.keys():
        raise RuntimeError(f"missing CAD test binaries: {sorted(missing)}")
    runs = []
    for pkg, kind, name, pkg_dir in production_targets:
        key = (pkg, kind, name)
        runs.append(([binaries[key]], pkg_dir))

    def execute(item):
        cmd, cwd = item
        result = run(cmd, cwd=cwd, env=dict(env, CARGO_MANIFEST_DIR=str(cwd)),
                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        return result.stdout

    try:
        jobs = int(os.environ.get("BOARDSTUDIO_CHECK_JOBS", "2"))
    except ValueError:
        jobs = 2
    if jobs <= 1:
        for item in runs:
            try:
                output.write(execute(item))
            except subprocess.CalledProcessError as error:
                output.write(error.stdout or "")
                raise
        return
    with ThreadPoolExecutor(max_workers=min(4, jobs), thread_name_prefix="cad-run") as pool:
        futures = [pool.submit(execute, item) for item in runs]
    first_error = None
    for future in futures:
        error = future.exception()
        if error is None:
            output.write(future.result())
        else:
            if isinstance(error, subprocess.CalledProcessError):
                output.write(error.stdout or "")
            if first_error is None:
                first_error = error
    if first_error is not None:
        raise first_error


def test_cad():
    _test_cad_unified(sys.stdout)
