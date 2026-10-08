"""Pinned OCCT preparation and Cadrum build orchestration (standard library only)."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import urllib.request

CAD_ROOT = Path(__file__).resolve().parents[1]
VERSION = "8_0_1_rev2"
BUILDS = {
    "wasm": (f"occt-{VERSION}-wasm32_unknown_unknown.tar.gz",
             "8149e781acdbd21507cfb29937e48e5fdbe628ee5c37007f720dcf5d4000e6ad"),
    "native": (f"occt-{VERSION}-x86_64_unknown_linux_gnu.tar.gz",
               "95e068936c0cb4ba2668707c0dca209d3396103dfc1db85eb72d192badfb4143"),
}
IMAGE = "boardstudio-cadrum-wasm:0.8.20-rust-1.98.0-wasi-sdk-33-bindgen-0.2.129"
# CI publishes the toolchain image here (.github/workflows/cad-image.yaml), tagged by the
# Containerfile's content so a pull never returns an image built from different steps.
REGISTRY_IMAGE = "ghcr.io/runinit/boardstudio-cadrum-wasm"
CONTAINERFILE = "wasm/Containerfile"


def run(command, **kwargs):
    subprocess.run(command, cwd=CAD_ROOT, check=True, **kwargs)


def prepare(target):
    archive_name, expected = BUILDS[target]
    cache = CAD_ROOT / ".cache/cadrum" / target
    archive = cache / archive_name
    root = cache / archive_name.removesuffix(".tar.gz")
    cache.mkdir(parents=True, exist_ok=True)
    if not archive.exists():
        temporary = archive.with_name(archive.name + ".download")
        url = f"https://github.com/lzpel/cadrum/releases/download/occt-{VERSION}/{archive_name}"
        try:
            with urllib.request.urlopen(url) as response, temporary.open("wb") as output:
                shutil.copyfileobj(response, output)
            temporary.replace(archive)
        finally:
            temporary.unlink(missing_ok=True)
    with archive.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    if digest != expected:
        raise RuntimeError(f"OCCT archive checksum mismatch for {archive_name}: {digest}")
    if not (root / "include/opencascade").exists():
        run(["tar", "-xzf", str(archive), "-C", str(cache)])
    if not (root / "include/opencascade").is_dir() or not (root / "lib").is_dir():
        raise RuntimeError(f"Verified OCCT archive has an unexpected layout: {root}")
    return root


def available(command):
    try:
        return subprocess.run([command, "--version"], stdout=subprocess.DEVNULL,
                              stderr=subprocess.DEVNULL).returncode == 0
    except OSError:
        return False


def succeeds(command):
    return subprocess.run(command, cwd=CAD_ROOT).returncode == 0


def registry_image():
    digest = hashlib.sha256((CAD_ROOT / CONTAINERFILE).read_bytes()).hexdigest()[:16]
    return f"{REGISTRY_IMAGE}:containerfile-{digest}"


def provide_image(runtime):
    """Pull the published toolchain image, or build it locally when it is unavailable."""
    if os.environ.get("CADRUM_BUILD_IMAGE") != "1":
        published = registry_image()
        if succeeds([runtime, "pull", published]):
            run([runtime, "tag", published, IMAGE])
            return
        print(f"cadrum: {published} is unavailable; building the toolchain image locally", flush=True)
    run([runtime, "build", "--file", CONTAINERFILE, "--tag", IMAGE, "."])


def wasm_inputs_digest():
    """Hash everything the container build reads, so an unchanged provider is not rebuilt."""
    contracts = CAD_ROOT.parent / "contracts/rust"
    digest = hashlib.sha256(f"{IMAGE}\n{VERSION}\n".encode())
    digest.update(Path(__file__).read_bytes())
    digest.update((CAD_ROOT / CONTAINERFILE).read_bytes())
    for base, names in ((CAD_ROOT.parent, ("Cargo.toml",)),
                        (CAD_ROOT / "wasm", ("Cargo.toml", "Cargo.lock", "src", "assets")),
                        (contracts, ("Cargo.toml", "src"))):
        for name in names:
            path = base / name
            files = sorted(path.rglob("*")) if path.is_dir() else [path]
            for file in files:
                if file.is_file():
                    digest.update(file.relative_to(CAD_ROOT.parent).as_posix().encode() + b"\0")
                    digest.update(file.read_bytes())
    return digest.hexdigest()


def _toml_value(value):
    """Encode workspace lint tables as TOML inline tables without a third-party writer."""
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{json.dumps(key)} = {_toml_value(item)}" for key, item in value.items()) + " }"
    return json.dumps(value)


def _container_workspace_manifest():
    """Keep contract lint inheritance while excluding the independently built CAD crates."""
    workspace = tomllib.loads((CAD_ROOT.parent / "Cargo.toml").read_text())["workspace"]
    contracts = tomllib.loads((CAD_ROOT.parent / "contracts/rust/Cargo.toml").read_text())

    def validate_inheritance(table, path=()):
        for key, value in table.items():
            if key == "workspace" and value is True and path != ("lints",):
                raise RuntimeError(f"CAD container does not support contracts workspace inheritance at {'.'.join(path)}")
            if isinstance(value, dict):
                validate_inheritance(value, (*path, key))

    validate_inheritance(contracts)
    return ('[workspace]\n'
            f'resolver = {json.dumps(workspace["resolver"])}\n'
            'members = ["contracts/rust"]\n'
            'exclude = ["cad/wasm", "cad/step-oracle"]\n'
            f'lints = {_toml_value(workspace.get("lints", {}))}\n')


def build_wasm():
    stamp = CAD_ROOT / ".cache/cadrum/wasm-build.sha256"
    inputs = wasm_inputs_digest()
    output = CAD_ROOT / "wasm/pkg/boardstudio_cadrum_wasm_bg.wasm"
    if output.is_file() and stamp.is_file() and stamp.read_text().strip() == inputs:
        print("cadrum: wasm/pkg is current; skipping the container build", flush=True)
        return
    occt = prepare("wasm").relative_to(CAD_ROOT).as_posix()
    runtime = os.environ.get("CADRUM_CONTAINER_RUNTIME") or next(
        (name for name in ("podman", "docker") if available(name)), None)
    if not runtime:
        raise RuntimeError("Building the Cadrum WASM module requires Podman or Docker")
    workspace_manifest = _container_workspace_manifest()
    # CI may build and load the pinned image itself, then set CADRUM_IMAGE_READY.
    if not os.environ.get("CADRUM_IMAGE_READY"):
        provide_image(runtime)
    contracts = CAD_ROOT.parent / "contracts/rust"
    label = ":Z" if runtime == "podman" else ""
    cad_mount = f"{CAD_ROOT}:/workspace/cad{label}"
    # wasm-pack 0.15 mistakes its previous package manifest for wasm-bindgen's
    # dependency map. Remove only this generated manifest before rebuilding.
    (CAD_ROOT / "wasm/pkg/package.json").unlink(missing_ok=True)
    # Keep crates.io downloads between runs (cad/.cache is gitignored and cached in CI).
    registry = CAD_ROOT / ".cache/cargo-registry"
    git_deps = CAD_ROOT / ".cache/cargo-git"
    for directory in (registry, git_deps):
        directory.mkdir(parents=True, exist_ok=True)
    stamp.parent.mkdir(parents=True, exist_ok=True)
    jobs = ["--env", f"CARGO_BUILD_JOBS={os.environ['CARGO_BUILD_JOBS']}"] if os.environ.get("CARGO_BUILD_JOBS") else []
    # The full root workspace references crates absent from these narrow mounts.
    # Give contracts its inherited lints without joining CAD to that workspace.
    with tempfile.TemporaryDirectory(prefix="workspace-", dir=stamp.parent) as directory:
        manifest = Path(directory) / "Cargo.toml"
        manifest.write_text(workspace_manifest)
        run([runtime, "run", "--rm", "--volume", cad_mount,
             "--volume", f"{registry}:/root/.cargo/registry{label}",
             "--volume", f"{git_deps}:/root/.cargo/git{label}",
             "--volume", f"{manifest}:/workspace/Cargo.toml:ro{',Z' if label else ''}",
             "--volume", f"{contracts}:/workspace/contracts/rust:ro",
             "--workdir", "/workspace/cad", "--env", f"OCCT_ROOT=/workspace/cad/{occt}",
             "--env", "CARGO_TARGET_DIR=/workspace/cad/wasm/target", *jobs, IMAGE,
             "wasm-pack", "build", "wasm", "--target", "web", "--out-dir", "pkg",
             "--out-name", "boardstudio_cadrum_wasm", "--release", "--locked"])
    stamp.write_text(inputs + "\n")


def build_step_oracle(occt_root):
    """Build the test-only OCCT STEP reimport oracle that the native tests drive as a subprocess."""
    env = dict(os.environ, OCCT_ROOT=str(occt_root),
               CARGO_TARGET_DIR=str(CAD_ROOT / "step-oracle/target"))
    run(["cargo", "build", "--manifest-path", "step-oracle/Cargo.toml", "--locked"], env=env)


def test_cadrum():
    run(["cargo", "build", "--manifest-path", "../core/Cargo.toml",
         "--example", "prepare_case", "--locked"])
    occt_root = prepare("native")
    build_step_oracle(occt_root)
    env = dict(os.environ, OCCT_ROOT=str(occt_root),
               CARGO_TARGET_DIR=str(CAD_ROOT / "wasm/target"))
    # The WASM provider is built by build-cadrum-wasm.py (the `build` check step).
    run(["cargo", "test", "--manifest-path", "wasm/Cargo.toml", "--locked"], env=env)
