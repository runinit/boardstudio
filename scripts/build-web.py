#!/usr/bin/env python3
"""Build BoardStudio's web providers and static Dioxus sites."""

from __future__ import annotations

import argparse
import base64
import hashlib
import time
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "web"
def dx_public(release: bool) -> Path:
    profile = "release" if release else "debug"
    return WEB / f"target/dx/boardstudio-web/{profile}/web/public"


def run(command: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None) -> None:
    print("$ " + " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def provider_commands() -> list[tuple[list[str], Path]]:
    """Commands required to create assets consumed by the Dioxus application."""
    return [
        (["wasm-pack", "build", "core", "--target", "web", "--release", "--locked"], ROOT),
        (["wasm-pack", "build", str(WEB), "--target", "web", "--out-name", "m1_core_worker",
          "--out-dir", str(WEB / "target/providers/core-worker"), "--release", "--locked",
          "--no-default-features", "--features", "core-worker"], ROOT),
        (["wasm-pack", "build", str(WEB), "--target", "web", "--out-name", "m1_cad_worker",
          "--out-dir", str(WEB / "target/providers/cad-worker"), "--release", "--locked",
          "--no-default-features", "--features", "cad-worker"], ROOT),
        (["wasm-pack", "build", "renderer", "--target", "web", "--out-dir", "pkg",
          "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"], ROOT),
        (["pnpm", "--dir", "cad", "run", "build:wasm"], ROOT),
        (["cargo", "run", "--manifest-path", "core/Cargo.toml", "--locked", "--release",
          "--example", "prepare_demo_projects", "--", "web/assets/fixtures"], ROOT),
        ([sys.executable, "scripts/stage-ergogen-models.py", "--source-root",
          "ergogen/library/vendor", "--destination", "web/assets/ergogen-models",
          "--manifest", "web/target/providers/ergogen-models.json"], ROOT),
    ]


def copy_runtime_assets(site: Path, provider_root: Path | None = None) -> None:
    provider_root = provider_root or ROOT
    assets = site / "assets"
    shutil.copytree(provider_root / "web/assets", assets, dirs_exist_ok=True,
                    ignore=shutil.ignore_patterns("layout-generators.js", "layout-generators",
                                                 "preview-generator"))
    shutil.copy2(provider_root / "catalogue/modules/imported-modules.json",
                 assets / "imported-modules.json")
    shutil.copytree(provider_root / "cad/wasm/pkg", assets / "cad", dirs_exist_ok=True)
    for name in ("core-worker", "cad-worker"):
        shutil.copytree(provider_root / f"web/target/providers/{name}", assets / name,
                        dirs_exist_ok=True)
    shutil.copytree(provider_root / "renderer/pkg", assets / "renderer", dirs_exist_ok=True)
    (assets / "core-worker/entry.js").write_text(
        'import init, { start_core_worker } from "./m1_core_worker.js";\n'
        'await init();\nstart_core_worker();\n'
    )
    (assets / "cad-worker/entry.js").write_text(
        'import init, { start_cad_worker } from "./m1_cad_worker.js";\n'
        'await init();\n'
        'start_cad_worker(new URL("../cad/boardstudio_cadrum_wasm.js", import.meta.url).href);\n'
    )


def stage_runtime_assets(provider_root: Path = ROOT) -> None:
    """Copy generated providers into the asset directory watched by dx serve."""
    assets = provider_root / "web/assets"
    shutil.copytree(provider_root / "cad/wasm/pkg", assets / "cad", dirs_exist_ok=True)
    shutil.copytree(provider_root / "web/target/providers/core-worker", assets / "core-worker",
                    dirs_exist_ok=True)
    shutil.copytree(provider_root / "web/target/providers/cad-worker", assets / "cad-worker",
                    dirs_exist_ok=True)
    shutil.copytree(provider_root / "renderer/pkg", assets / "renderer", dirs_exist_ok=True)
    (assets / "core-worker/entry.js").write_text(
        'import init, { start_core_worker } from "./m1_core_worker.js";\n'
        'await init();\nstart_core_worker();\n'
    )
    (assets / "cad-worker/entry.js").write_text(
        'import init, { start_cad_worker } from "./m1_cad_worker.js";\n'
        'await init();\n'
        'start_cad_worker(new URL("../cad/boardstudio_cadrum_wasm.js", import.meta.url).href);\n'
    )


def embed_offline_worker(site: Path, prefix: str, build_temp: Path) -> None:
    """Build the Dioxus offline worker against this route's asset inventory."""
    worker_dir = build_temp / f"offline-{prefix}"
    manifest_path = build_temp / f"offline-manifest-{prefix}.json"
    assets = sorted(path.relative_to(site).as_posix() for path in site.rglob("*") if path.is_file())
    required = set(assets) | {"service-worker.js", "boardstudio_offline_worker.js"}
    digest = hashlib.sha256()
    for relative in sorted(required):
        digest.update(relative.encode())
        path = site / relative
        if path.is_file():
            digest.update(path.read_bytes())
    manifest = {"version": f"boardstudio-{digest.hexdigest()[:16]}-{time.time_ns()}",
                "assets": sorted(required)}
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    run(["wasm-pack", "build", "web", "--target", "web", "--out-name",
         "boardstudio_offline_worker", "--out-dir", str(worker_dir), "--release", "--locked",
         "--no-default-features", "--features", "service-worker"], env=dict(
             os.environ, BOARDSTUDIO_OFFLINE_MANIFEST=str(manifest_path)))
    js_name = "boardstudio_offline_worker.js"
    wasm_name = "boardstudio_offline_worker_bg.wasm"
    wasm = (worker_dir / wasm_name).read_bytes()
    shutil.copy2(worker_dir / js_name, site / js_name)
    encoded = base64.b64encode(wasm).decode("ascii")
    bootstrap = (
        "/* Offline cache policy is implemented in Rust. */\n"
        f'import {{ initSync }} from "./{js_name}";\n'
        f'initSync({{ module: Uint8Array.from(atob("{encoded}"), c => c.charCodeAt(0)) }});\n'
    )
    (site / "service-worker.js").write_text(bootstrap)


def build(output: Path, *, release: bool = True, prepare_providers: bool = True) -> Path:
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if prepare_providers:
        prepare_runtime_providers()
    public = dx_public(release)

    build_temp = output / ".build"
    build_temp.mkdir(exist_ok=True)
    for route, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
        if public.exists():
            shutil.rmtree(public)
        command = ["dx", "build", "--web", "--base-path", prefix,
                   "--no-default-features", "--features", "page", "--cargo-args=--locked"]
        if release:
            command.insert(3, "--release")
        run(command, cwd=WEB)
        if not public.is_dir():
            raise FileNotFoundError(f"Dioxus did not produce its web public directory: {public}")
        site = output / ("site-root" if route == "root" else "site-subpath/boardstudio")
        if site.exists():
            shutil.rmtree(site)
        site.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(public, site)
        copy_runtime_assets(site)
        embed_offline_worker(site, route, build_temp)
    print(output, flush=True)
    return output


def prepare_runtime_providers() -> None:
    for command, cwd in provider_commands():
        run(command, cwd=cwd)
    shutil.copy2(ROOT / "catalogue/modules/imported-modules.json",
                 WEB / "assets/imported-modules.json")
    stage_runtime_assets()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "web/target/site",
                        help="directory for site-root and site-subpath builds")
    parser.add_argument("--debug", action="store_true", help="build Dioxus without release optimization")
    parser.add_argument("--skip-providers", action="store_true",
                        help="reuse already generated provider files in the working tree")
    parser.add_argument("--providers-only", action="store_true",
                        help="build runtime providers for `pnpm dev` without packaging routes")
    args = parser.parse_args(argv)
    try:
        if args.providers_only:
            prepare_runtime_providers()
        else:
            build(args.output, release=not args.debug, prepare_providers=not args.skip_providers)
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"build-web: {error}", file=sys.stderr)
        return error.returncode if isinstance(error, subprocess.CalledProcessError) else 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
