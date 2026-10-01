#!/usr/bin/env python3
"""Build a uniquely staged M1 candidate and retain commands/source/asset hashes."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys

REPO = Path(__file__).resolve().parents[1]
WEB = REPO / "web"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources():
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=REPO).decode().split("\0")
    return {name: digest(REPO / name) for name in paths if name and
            (name.startswith(("web/", "application/", "core/", "contracts/", "renderer/", "cad/", "app/src/", "app/public/", "scripts/")) or name == "rust-toolchain.toml") and (REPO / name).is_file()}


def main():
    if len(sys.argv) != 2 or not sys.argv[1].replace("-", "").isalnum():
        raise SystemExit("usage: scripts/build-m1.py <unique-build-id>")
    build_id = sys.argv[1]
    output = WEB / "target" / "builds" / build_id
    output.mkdir(parents=True, exist_ok=False)
    (output / "tmp").mkdir()
    environment = dict(os.environ, TMPDIR=str(output / "tmp"))
    provenance = {"source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
                  "sources": sources(), "commands": [], "scope": "Development candidate; acceptance is recorded separately."}

    def run(name, command, cwd=REPO):
        print(f"{name}: {' '.join(map(str,command))}", flush=True)
        log = output / f"{name}.log"
        with log.open("w") as stream:
            result = subprocess.run(list(map(str, command)), cwd=cwd, env=environment, stdout=stream, stderr=subprocess.STDOUT)
        provenance["commands"].append({"argv": list(map(str, command)), "cwd": str(cwd), "exit": result.returncode, "log": str(log)})
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        if result.returncode:
            print(log.read_text()[-10000:], file=sys.stderr)
            raise SystemExit(result.returncode)

    run("core", ["wasm-pack", "build", REPO / "core", "--target", "web", "--release", "--locked"])
    run("core-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_core_worker", "--out-dir", output / "core-worker", "--release", "--locked", "--no-default-features", "--features", "core-worker"])
    run("renderer", ["wasm-pack", "build", REPO / "renderer", "--target", "web", "--out-dir", output / "renderer", "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"])
    run("fixtures", ["node", REPO / "scripts/prepare-m1-fixtures.mjs", output / "fixtures"])
    for mode, prefix in [("root", "/"), ("subpath", "/boardstudio/")]:
        run(f"page-{mode}", ["dx", "build", "--web", "--release", "--base-path", prefix, "--cargo-args=--locked"], WEB)
        public = WEB / "target/dx/boardstudio-web/release/web/public"
        destination = output / f"site-{mode}"
        if mode == "subpath":
            destination /= "boardstudio"
        shutil.copytree(public, destination)
        assets = destination / "assets"
        shutil.copytree(WEB / "assets", assets, dirs_exist_ok=True)
        for name in ["core-worker", "renderer", "fixtures"]:
            shutil.copytree(output / name, assets / name, dirs_exist_ok=True)
        (assets / "core-worker/entry.js").write_text('import init, { start_core_worker } from "./m1_core_worker.js";\nawait init();\nstart_core_worker();\n')
        provenance[mode] = {"prefix": prefix, "site": str(destination), "assets": {
            str(path.relative_to(destination)): digest(path) for path in sorted(destination.rglob("*")) if path.is_file()}}
    if sources() != provenance["sources"]:
        raise SystemExit("Source changed during build: candidate has mixed provenance and must be rebuilt.")
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
    print(output, flush=True)


if __name__ == "__main__":
    main()
