#!/usr/bin/env python3
"""Build a uniquely staged M1 candidate and retain commands/source/asset hashes."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone

REPO = Path(__file__).resolve().parents[1]
WEB = REPO / "web"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources():
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=REPO).decode().split("\0")
    # Match the provider's recursive model enumeration, including newly added
    # source models that have not yet been staged in Git.
    vendor = REPO / "ergogen/library/vendor"
    paths = sorted(set(paths) | {
        path.relative_to(REPO).as_posix()
        for path in vendor.glob("*/3d_models/**/*")
        if path.is_file() and path.suffix.lower() in {".step", ".stp", ".stl", ".wrl"}
    })
    return {name: digest(REPO / name) for name in paths if name and
            (name.startswith(("web/", "application/", "core/", "contracts/", "renderer/", "cad/", "kicad/src/", "app/src/", "app/public/", "scripts/", "ergogen/library/vendor/", "ergogen/src/", "ergogen/generated/")) or name == "rust-toolchain.toml") and (REPO / name).is_file()}


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

    def run(name, command, cwd=REPO, extra_env=None):
        print(f"{name}: {' '.join(map(str,command))}", flush=True)
        log = output / f"{name}.log"
        started = datetime.now(timezone.utc).isoformat()
        with log.open("w") as stream:
            result = subprocess.run(list(map(str, command)), cwd=cwd, env=dict(environment, **(extra_env or {})), stdout=stream, stderr=subprocess.STDOUT)
        provenance["commands"].append({"argv": list(map(str, command)), "cwd": str(cwd), "exit": result.returncode, "log": str(log), "environment": extra_env or {}, "started": started, "finished": datetime.now(timezone.utc).isoformat()})
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        if result.returncode:
            print(log.read_text()[-10000:], file=sys.stderr)
            raise SystemExit(result.returncode)

    for name, command in [("rustc-version", ["rustc", "--version"]), ("cargo-version", ["cargo", "--version"]), ("dx-version", ["dx", "--version"]), ("wasm-pack-version", ["wasm-pack", "--version"]), ("node-version", ["node", "--version"]), ("pnpm-version", ["pnpm", "--version"])]:
        run(name, command)
    run("core", ["wasm-pack", "build", REPO / "core", "--target", "web", "--release", "--locked"])
    run("core-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_core_worker", "--out-dir", output / "core-worker", "--release", "--locked", "--no-default-features", "--features", "core-worker"])
    run("cad-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_cad_worker", "--out-dir", output / "cad-worker", "--release", "--locked", "--no-default-features", "--features", "cad-worker"])
    run("renderer", ["wasm-pack", "build", REPO / "renderer", "--target", "web", "--out-dir", output / "renderer", "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"])
    run("cad", ["pnpm", "--dir", "cad", "run", "build:wasm"])
    run("fixtures", ["node", REPO / "scripts/prepare-m1-fixtures.mjs", output / "fixtures"])
    run("layout-generators", ["node", REPO / "scripts/web/build-layout-generators.mjs", WEB / "assets"])
    run("preview-generator", ["node", REPO / "scripts/web/build-preview-generator.mjs", WEB / "assets"])
    run("ergogen-models", [sys.executable, REPO / "scripts/stage-ergogen-models.py",
                           "--source-root", REPO / "ergogen/library/vendor",
                           "--destination", WEB / "assets/ergogen-models",
                           "--manifest", output / "ergogen-models-catalog.json"])
    for mode, prefix in [("root", "/"), ("subpath", "/boardstudio/")]:
        public = WEB / "target/dx/boardstudio-web/release/web/public"
        # Dioxus retains prior hashed assets. Preserve them outside this release
        # and build into a fresh public directory so cache contents are repeatable.
        if public.exists():
            shutil.move(public, output / f"previous-dx-public-{mode}")
        run(f"page-{mode}", ["dx", "build", "--web", "--release", "--base-path", prefix, "--no-default-features", "--features", "page", "--cargo-args=--locked"], WEB)
        destination = output / f"site-{mode}"
        if mode == "subpath":
            destination /= "boardstudio"
        shutil.copytree(public, destination)
        assets = destination / "assets"
        shutil.copytree(WEB / "assets", assets, dirs_exist_ok=True)
        shutil.copytree(REPO / "cad/wasm/pkg", assets / "cad", dirs_exist_ok=True)
        for name in ["core-worker", "cad-worker", "renderer", "fixtures"]:
            shutil.copytree(output / name, assets / name, dirs_exist_ok=True)
        (assets / "core-worker/entry.js").write_text('import init, { start_core_worker } from "./m1_core_worker.js";\nawait init();\nstart_core_worker();\n')
        (assets / "cad-worker/entry.js").write_text('import init, { start_cad_worker } from "./m1_cad_worker.js";\nawait init();\nstart_cad_worker(new URL("../cad/boardstudio_cadrum_wasm.js", import.meta.url).href);\n')
        manifest = output / f"offline-manifest-{mode}.json"
        required = sorted({str(path.relative_to(destination)) for path in destination.rglob("*") if path.is_file()} | {"service-worker.js", "boardstudio_offline_worker.js"})
        manifest.write_text(json.dumps({"version": f"{build_id}-{mode}", "assets": required}, indent=2)+"\n")
        run(f"offline-worker-{mode}", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "boardstudio_offline_worker", "--out-dir", output / f"offline-{mode}", "--release", "--locked", "--no-default-features", "--features", "service-worker"], extra_env={"BOARDSTUDIO_OFFLINE_MANIFEST": str(manifest)})
        run(f"embed-offline-{mode}", ["node", REPO / "scripts/web/embed-worker-wasm.mjs", output / f"offline-{mode}", manifest, destination / "service-worker.js"])
        provenance[mode] = {"prefix": prefix, "site": str(destination), "assets": {
            str(path.relative_to(destination)): digest(path) for path in sorted(destination.rglob("*")) if path.is_file()}}
    if sources() != provenance["sources"]:
        raise SystemExit("Source changed during build: candidate has mixed provenance and must be rebuilt.")
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
    print(output, flush=True)


if __name__ == "__main__":
    main()
