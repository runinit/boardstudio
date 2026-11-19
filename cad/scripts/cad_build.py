"""Pure-Rust Monstertruck CAD WASM build and native test orchestration."""

from concurrent.futures import ThreadPoolExecutor
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import browser_tools

CAD_ROOT = Path(__file__).resolve().parents[1]
OUT_NAME = "boardstudio_cad_wasm"
WASM_BUILD_TARGET = "wasm/target"
NATIVE_TEST_TARGETS = ("monstertruck/target", "monstertruck-qualification/target")
VENDOR_CRATES = (
    "inflector",
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
    vendor_root = CAD_ROOT / "monstertruck-qualification/vendor"
    profile = "release-no-opt" if no_opt else "release"
    digest = hashlib.sha256(f"{OUT_NAME}\n{profile}\n".encode())
    digest.update(Path(__file__).read_bytes())
    roots = [
        (CAD_ROOT.parent, ("Cargo.toml", "rust-toolchain.toml")),
        (CAD_ROOT / "wasm", ("Cargo.toml", "src", "assets")),
        (CAD_ROOT / "monstertruck", ("Cargo.toml", "Cargo.lock", "src")),
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


def _test_production_cad(output):
    library_target, _ = NATIVE_TEST_TARGETS
    command = ["cargo", "test", "--manifest-path", "monstertruck/Cargo.toml",
               "-p", "boardstudio-cad", "-p", "boardstudio-cad-wasm",
               "--profile", "cad-test", "--lib", "--tests", "--examples", "--locked"]
    _run_test(command, library_target, output)


def _owner_binary_key(package, selection):
    if selection[0] == "--lib":
        return (package, "lib", package.replace("-", "_"))
    return (package, "test", selection[1])


def _test_cad_owners(output):
    _, owner_target = NATIVE_TEST_TARGETS
    env = dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / owner_target))
    command = ["cargo", "test", "--no-run", "--manifest-path", "monstertruck-qualification/Cargo.toml",
               "-p", "monstertruck-io", "-p", "monstertruck-meshing", "--lib", "--test", "input",
               "--test", "closed_trim_projection", "--profile", "cad-test", "--locked", "--message-format=json"]
    expected = {_owner_binary_key(package, selection) for package, *selection in OWNER_TESTS}
    binaries = {}
    duplicates = set()
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
            if message.get("reason") != "compiler-artifact" or not message.get("executable"):
                continue
            target = message["target"]
            kind = "lib" if target["kind"] == ["lib"] else "test" if "test" in target["kind"] else None
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
    if duplicates:
        raise RuntimeError(f"duplicate CAD owner test binaries: {sorted(duplicates)}")
    if missing := expected - binaries.keys():
        raise RuntimeError(f"missing CAD owner test binaries: {sorted(missing)}")
    # Filter legacy suites; external fixtures are read only by the tests that use them.
    for package, *selection in OWNER_TESTS:
        key = _owner_binary_key(package, selection)
        args = selection[1:] if selection[0] == "--lib" else selection[2:]
        cwd = CAD_ROOT / "monstertruck-qualification/vendor" / package
        try:
            result = run([binaries[key], *args], cwd=cwd, env=dict(env, CARGO_MANIFEST_DIR=str(cwd)),
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        except subprocess.CalledProcessError as error:
            output.write(error.stdout or "")
            raise
        output.write(result.stdout)
        if not re.search(r"^test result: ok\. [1-9][0-9]* passed;", result.stdout, re.M):
            raise RuntimeError(f"CAD owner selection has no executed tests: {package} {' '.join(selection)}")


def _run_test(command, target, output):
    env = dict(os.environ, CARGO_TARGET_DIR=str(CAD_ROOT / target))
    try:
        result = run(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    except subprocess.CalledProcessError as error:
        output.write(error.stdout or "")
        raise
    output.write(result.stdout)
    return result.stdout


def test_cad():
    lanes = (_test_production_cad, _test_cad_owners)
    try:
        serial = int(os.environ.get("BOARDSTUDIO_CHECK_JOBS", "2")) <= 1
    except ValueError:
        serial = False
    if serial:
        for lane in lanes:
            lane(sys.stdout)
        return
    outputs = [io.StringIO() for _ in lanes]
    with ThreadPoolExecutor(max_workers=len(lanes), thread_name_prefix="cad-test") as pool:
        futures = [pool.submit(lane, output) for lane, output in zip(lanes, outputs)]
    failures = []
    for lane, output, future in zip(lanes, outputs, futures):
        print(output.getvalue(), end="", flush=True)
        if error := future.exception():
            print(f"cad: {lane.__name__}: {error}", file=sys.stderr, flush=True)
            failures.append(error)
    if failures:
        raise failures[0]
