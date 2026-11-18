#!/usr/bin/env python3
"""Build BoardStudio's web providers and static Dioxus sites."""

from __future__ import annotations

import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
import hashlib
import io
import time
import json
from pathlib import Path
import shutil
import subprocess
import sys
import browser_tools
from check import check_jobs

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "web"
OFFLINE_WORKER = WEB / "target/providers/offline-worker"
# Route name -> Dioxus base path. Pages deploys only `subpath`.
ROUTES = {"subpath": "/boardstudio/", "root": "/"}
DEFAULT_ROUTES = ("subpath",)
# .cargo/config.toml already passes dx's getrandom cfg to every wasm build; adding it twice would
# give dx's units different flags from wasm-pack's and stop them being shared.
DX_FLAGS = ("--wasm-js-cfg", "false")


def dx_public(release: bool) -> Path:
    profile = "release" if release else "debug"
    # Cargo workspace target, shared with every other crate build.
    return ROOT / f"target/dx/boardstudio-web/{profile}/web/public"


def route_site(output: Path, route: str) -> Path:
    return output / ("site-root" if route == "root" else "site-subpath/boardstudio")


def run(command: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None,
        output=None) -> None:
    options = {}
    if output is not None:
        options.update(stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print("$ " + " ".join(command), file=output or sys.stdout, flush=True)
    fd = browser_tools._inherited_build_lock()
    try:
        result = subprocess.run(command, cwd=cwd, env=env, check=True,
                                pass_fds=() if fd is None else (fd,), **options)
    except subprocess.CalledProcessError as error:
        if output is not None:
            output.write(error.stdout or "")
        raise
    if output is not None:
        output.write(result.stdout)


def run_lanes(lanes: list[list[tuple[list[str], Path]]]) -> None:
    def run_lane(lane, output):
        for command, cwd in lane:
            run(command, cwd=cwd, output=output)

    outputs = [io.StringIO() for _ in lanes]
    with ThreadPoolExecutor(max_workers=min(check_jobs(), len(lanes)), thread_name_prefix="web-build") as pool:
        futures = [pool.submit(run_lane, lane, output) for lane, output in zip(lanes, outputs)]
    failures = []
    for output, future in zip(outputs, futures):
        print(output.getvalue(), end="", flush=True)
        if error := future.exception():
            print(f"build-web: {error}", file=sys.stderr, flush=True)
            failures.append(error)
    if failures:
        raise failures[0]


def provider_commands(fast: bool = False) -> list[tuple[list[str], Path]]:
    """Commands required to create assets consumed by the Dioxus application.

    `fast` skips wasm-opt for the checked builds; the shipped build keeps it.
    """
    commands = [
        (["wasm-pack", "build", str(WEB), "--target", "web", "--out-name", "m1_core_worker",
          "--out-dir", str(WEB / "target/providers/core-worker"), "--release", "--locked",
          "--no-default-features", "--features", "core-worker"], ROOT),
        (["wasm-pack", "build", str(WEB), "--target", "web", "--out-name", "m1_cad_worker",
          "--out-dir", str(WEB / "target/providers/cad-worker"), "--release", "--locked",
          "--no-default-features", "--features", "cad-worker"], ROOT),
        # Route-independent: each route's asset manifest is passed in by its bootstrap.
        (["wasm-pack", "build", str(WEB), "--target", "web", "--out-name",
          "boardstudio_offline_worker", "--out-dir", str(OFFLINE_WORKER), "--release", "--locked",
          "--no-default-features", "--features", "service-worker"], ROOT),
        (["wasm-pack", "build", "renderer", "--target", "web", "--out-dir", "pkg",
          "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"], ROOT),
        ([sys.executable, "cad/scripts/build-cad-wasm.py",
          *(["--no-opt"] if fast else [])], ROOT),
        ([sys.executable, "scripts/stage-ergogen-models.py", "--source-root",
          "ergogen/library/vendor", "--destination", "web/assets/ergogen-models",
          "--manifest", "web/target/providers/ergogen-models.json"], ROOT),
    ]
    if fast:
        for command, _ in commands:
            if command[:2] == ["wasm-pack", "build"]:
                command.insert(command.index("--locked"), "--no-opt")
    return commands


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
        'start_cad_worker(new URL("../cad/boardstudio_cad_wasm.js", import.meta.url).href);\n'
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
        'start_cad_worker(new URL("../cad/boardstudio_cad_wasm.js", import.meta.url).href);\n'
    )


def validate_offline_manifest(manifest: dict) -> None:
    """Mirror `OfflineManifest::validate` so a bad manifest fails the build, not the install."""
    version, assets = manifest["version"], manifest["assets"]
    if not version or not all(c.isascii() and (c.isalnum() or c in "-_.") for c in version):
        raise ValueError(f"offline manifest version has unsupported characters: {version!r}")
    if "index.html" not in assets:
        raise ValueError("offline manifest must include index.html")
    if len(set(assets)) != len(assets):
        raise ValueError("offline manifest contains duplicate asset paths")
    for path in assets:
        if (not path or path.startswith("/") or any(c in path for c in "\\?#:%")
                or any(segment in ("", ".", "..") for segment in path.split("/"))):
            raise ValueError(f"offline asset path is not normalized and scope-relative: {path}")


def embed_offline_worker(site: Path, worker_dir: Path = OFFLINE_WORKER) -> None:
    """Stage the prebuilt offline worker with this route's asset inventory."""
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
    validate_offline_manifest(manifest)
    js_name = "boardstudio_offline_worker.js"
    wasm_name = "boardstudio_offline_worker_bg.wasm"
    wasm = (worker_dir / wasm_name).read_bytes()
    shutil.copy2(worker_dir / js_name, site / js_name)
    encoded = base64.b64encode(wasm).decode("ascii")
    bootstrap = (
        "/* Offline cache policy is implemented in Rust. */\n"
        f'import {{ initSync, start_offline_worker }} from "./{js_name}";\n'
        f'initSync({{ module: Uint8Array.from(atob("{encoded}"), c => c.charCodeAt(0)) }});\n'
        f"start_offline_worker({json.dumps(manifest['version'])}, {json.dumps(manifest['assets'])});\n"
    )
    (site / "service-worker.js").write_text(bootstrap)


def dx_command(route: str, *, release: bool, fast: bool) -> tuple[list[str], Path]:
    command = ["dx", "build", "--web", "--base-path", ROUTES[route], *DX_FLAGS,
               "--no-default-features", "--features", "page", "--cargo-args=--locked"]
    if release:
        command.insert(3, "--release")
    elif fast:
        # The checked build shares the `wasm-check` profile with every wasm test build.
        command[3:3] = ["--profile", "wasm-check"]
    return command, WEB


def build(output: Path, *, release: bool = True, prepare_providers: bool = True,
          routes: tuple[str, ...] = DEFAULT_ROUTES, fast: bool = False) -> Path:
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    public = dx_public(release)
    overlap = fast and not release and prepare_providers and len(routes) == 1 and check_jobs() > 1
    if overlap and public.exists():
        shutil.rmtree(public)
    if prepare_providers:
        page_build = dx_command(routes[0], release=release, fast=fast) if overlap else None
        prepare_runtime_providers(fast, page_build=page_build)
    for route in routes:
        if not overlap:
            if public.exists():
                shutil.rmtree(public)
            command, cwd = dx_command(route, release=release, fast=fast)
            run(command, cwd=cwd)
        if not public.is_dir():
            raise FileNotFoundError(f"Dioxus did not produce its web public directory: {public}")
        site = route_site(output, route)
        if site.exists():
            shutil.rmtree(site)
        site.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(public, site)
        copy_runtime_assets(site)
        embed_offline_worker(site)
    print(output, flush=True)
    return output


def prepare_runtime_providers(fast: bool = False, *, page_build: tuple[list[str], Path] | None = None) -> None:
    commands = provider_commands(fast)
    if page_build is None:
        for command, cwd in commands:
            run(command, cwd=cwd)
    else:
        # Prepare Ergogen before dx sees the asset directory. Keep the root release
        # providers together because they share the same target/profile lock.
        release_providers, cad, ergogen = commands[:-2], commands[-2], commands[-1]
        command, cwd = ergogen
        run(command, cwd=cwd)
        run_lanes([release_providers, [cad], [page_build]])
    shutil.copy2(ROOT / "catalogue/modules/imported-modules.json",
                 WEB / "assets/imported-modules.json")
    stage_runtime_assets()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "web/target/site",
                        help="directory for site-root and site-subpath builds")
    parser.add_argument("--debug", action="store_true", help="build Dioxus without release optimization")
    parser.add_argument("--fast", action="store_true",
                        help="checked build: no wasm-opt and an unoptimized Dioxus build; "
                             "the default keeps the shipped, optimized build")
    parser.add_argument("--skip-providers", action="store_true",
                        help="reuse already generated provider files in the working tree")
    parser.add_argument("--routes", default=",".join(DEFAULT_ROUTES),
                        help="comma-separated routes to package: subpath (/boardstudio/, deployed "
                             "to Pages) and/or root (/)")
    parser.add_argument("--providers-only", action="store_true",
                        help="build runtime providers for `python3 scripts/dev-web.py` without packaging routes")
    args = parser.parse_args(argv)
    routes = tuple(dict.fromkeys(name.strip() for name in args.routes.split(",") if name.strip()))
    unknown = [name for name in routes if name not in ROUTES]
    if unknown or not routes:
        parser.error(f"--routes must name one or more of: {', '.join(ROUTES)}")
    try:
        if args.providers_only:
            prepare_runtime_providers(args.fast)
        else:
            build(args.output, release=not (args.debug or args.fast),
                  prepare_providers=not args.skip_providers, routes=routes, fast=args.fast)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"build-web: {error}", file=sys.stderr)
        return error.returncode if isinstance(error, subprocess.CalledProcessError) else 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
