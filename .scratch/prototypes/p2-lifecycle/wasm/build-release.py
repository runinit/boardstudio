#!/usr/bin/env python3
"""Build exact provider inputs and both P2 deployment prefixes from this checkout."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

WASM = Path(__file__).resolve().parent
REPO = WASM.parents[3]
P1 = WASM.parent.parent / "p1-core"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def provider_sources():
    paths = []
    for package in [REPO / "core", REPO / "contracts/rust", REPO / "renderer", P1]:
        paths.extend((package / "src").rglob("*.rs"))
        paths.extend(path for path in [package / "Cargo.toml", package / "Cargo.lock"] if path.is_file())
        paths.extend(package.glob("build.rs"))
    paths.extend((REPO / "renderer/vendor").rglob("*.rs"))
    paths.extend((REPO / "renderer/vendor").rglob("Cargo.toml"))
    paths.append(REPO / "rust-toolchain.toml")
    return {str(path.relative_to(REPO)): digest(path) for path in sorted(set(paths))}


def main():
    if len(sys.argv) != 2 or not sys.argv[1].replace("-", "").isalnum():
        raise SystemExit("usage: build-release.py <unique-alphanumeric-build-id>")
    build_id = sys.argv[1]
    output = WASM / "target/builds" / build_id
    output.mkdir(parents=True, exist_ok=False)
    temporary = output / "tmp"
    temporary.mkdir()
    environment = dict(os.environ, TMPDIR=str(temporary))
    commands = []

    def run(name, command, cwd=REPO):
        print(f"{name}: {' '.join(command)}", flush=True)
        log = output / f"{name}.log"
        with log.open("w") as stream:
            result = subprocess.run(command, cwd=cwd, env=environment, stdout=stream, stderr=subprocess.STDOUT)
        commands.append({"command": command, "cwd": str(cwd.relative_to(REPO)),
                         "exit": result.returncode, "log": str(log.relative_to(REPO))})
        (output / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
        if result.returncode:
            print(log.read_text()[-6000:], file=sys.stderr)
            raise SystemExit(result.returncode)

    sources = provider_sources()
    run("worker", ["wasm-pack", "build", str(P1), "--target", "web", "--out-name", "p1_core",
                   "--out-dir", str(output / "worker"), "--release", "--locked", "--no-default-features", "--features", "worker"])
    run("renderer", ["wasm-pack", "build", str(REPO / "renderer"), "--target", "web",
                     "--out-name", "boardstudio_renderer_wasm", "--out-dir", str(output / "renderer"), "--release", "--locked"])
    if provider_sources() != sources:
        raise SystemExit("provider sources changed during build; refuse to record mixed provenance")
    for built, target in [("worker", "p1-worker"), ("renderer", "renderer")]:
        destination = WASM / "assets" / target
        destination.mkdir(parents=True, exist_ok=True)
        for source in (output / built).iterdir():
            if source.suffix in {".js", ".wasm"}:
                shutil.copy2(source, destination / source.name)
    entry = WASM / "assets/worker/worker-entry.js"
    entry.parent.mkdir(parents=True, exist_ok=True)
    entry.write_text('import init, { start_worker } from "../p1-worker/p1_core.js";\n\nawait init();\nstart_worker();\n')
    assets = {str(path.relative_to(WASM / "assets")): digest(path)
              for path in sorted((WASM / "assets").rglob("*")) if path.suffix in {".js", ".wasm"}}
    provenance = {
        "schema": 1, "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
        "source_files": sources, "assets": assets, "commands": list(commands),
        "tool_versions": {tool: subprocess.check_output([tool, "--version"], text=True).strip()
                          for tool in ["rustc", "wasm-pack", "dx"]},
        "scope": "Generated WASM/JS, plus initialization-only worker entry; application policy is Rust.",
    }
    (WASM / "asset-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    for mode, prefix in [("root", "/"), ("subpath", "/boardstudio/")]:
        run(f"dx-{mode}", ["dx", "build", "--web", "--release", "--base-path", prefix, "--cargo-args=--locked"], WASM)
        run(f"stage-{mode}", ["python3", str(WASM / "stage-site.py"), mode, build_id,
                              str(WASM / "target/dx/p2-lifecycle-probe/release/web/public")])
    print(f"Build and staging passed. Logs: {output}", flush=True)


if __name__ == "__main__":
    main()
