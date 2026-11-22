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
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import browser_tools
import cargo_json
from tailwind_tools import DX_TOOLS, IN_OVERLAP_ENV, check_jobs, tailwind_cli
import wasm_bindgen_tools

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "web"
OFFLINE_WORKER = WEB / "target/providers/offline-worker"
# Route name -> Dioxus base path. Pages deploys only `subpath`.
ROUTES = {"subpath": "/boardstudio/", "root": "/"}
DEFAULT_ROUTES = ("subpath",)
# .cargo/config.toml already passes dx's getrandom cfg to every wasm build; adding it twice would
# give dx's units different flags from wasm-pack's and stop them being shared.
DX_FLAGS = ("--wasm-js-cfg", "false")
# The checked page built by plain Cargo, outside dx's rustc wrapper.
PAGE_PUBLIC = WEB / "target/page/public"
PLAIN_PAGE_DX_VERSION = "0.7.10"
PAGE_CARGO = ["cargo", "build", "--locked", "--target", "wasm32-unknown-unknown", "--profile", "wasm-check",
              "-p", "boardstudio-web", "--bin", "boardstudio-web", "--no-default-features",
              "--features", "page", "--message-format=json"]
# Dioxus web auto-init, as `dx build` appends it to the wasm-bindgen `--target web` module.
DX_FOOTER = """export {{ wasm as __wasm }}

globalThis.__wasm_split_main_initSync = initSync;

// Actually perform the load
__wbg_init({{module_or_path: "{base_path}wasm/boardstudio-web_bg.wasm"}}).then((wasm) => {{
    // assign this module to be accessible globally
    globalThis.__dx_mainWasm = wasm;
    globalThis.__dx_mainInit = __wbg_init;
    globalThis.__dx_mainInitSync = initSync;
    globalThis.__dx___wbg_get_imports = __wbg_get_imports;

    if (wasm.__wbindgen_start == undefined) {{
        wasm.main();
    }}
}});
"""
# The footer refers to these module-scope wasm-bindgen 0.2.129 `--target web` bindings.
BINDGEN_BINDINGS = (r"^let [^;\n]*\bwasm\b[^;\n]*;", r"^function __wbg_get_imports\(",
                    r"^function initSync\(", r"^async function __wbg_init\(")


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


def run_lanes(lanes: list[list]) -> None:
    """Run each lane's steps in order. A step is `(command, cwd)` or a callable taking the lane output."""
    def run_lane(lane, output):
        for step in lane:
            if callable(step):
                step(output)
            else:
                command, cwd = step
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
        # Every checked wasm build, the page included, shares the `wasm-check` profile
        # (dev semantics instead of release; `check.py release` and CI keep `--release` and `wasm-opt`).
        for command, _ in commands:
            if command[:2] == ["wasm-pack", "build"]:
                command[command.index("--release"):command.index("--release") + 1] = [
                    "--profile", "wasm-check"]
                command.insert(command.index("--locked"), "--no-opt")
    return commands


def _stage_baked_step(worker: Path, provider_root: Path) -> str:
    source = provider_root / "web/target/providers/bundled-step"
    manifest = source / "manifest.json"
    destination = worker / "bundled-step"
    if destination.exists():
        shutil.rmtree(destination)
    if not manifest.is_file():
        return ""
    assets = json.loads(manifest.read_text())["assets"]
    destination.mkdir()
    for digest in set(assets.values()):
        shutil.copy2(source / f"{digest}.bstep", destination)
    # The generated inventory authorizes sidecars for known source bytes only.
    return (
        f'const bakedSteps = new Map(Object.entries({json.dumps(assets)}));\n'
        'async function loadBakedStep(sourceSha256) {\n'
        '    const digest = bakedSteps.get(sourceSha256);\n'
        '    if (!digest) return;\n'
        '    const response = await fetch(new URL(`./bundled-step/${digest}.bstep`, import.meta.url));\n'
        '    if (!response.ok) return;\n'
        '    const bytes = await response.arrayBuffer();\n'
        '    const actual = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),\n'
        '        byte => byte.toString(16).padStart(2, "0")).join("");\n'
        '    if (actual !== digest) return;\n'
        '    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));\n'
        '    return new Uint8Array(await new Response(stream).arrayBuffer());\n'
        '}\n'
    )


def _stage_provider_entries(assets: Path, provider_root: Path, *, include_site_assets: bool = False) -> None:
    """Copy the generated providers into `assets` and write the worker entry modules."""
    if include_site_assets:
        shutil.copytree(provider_root / "web/assets", assets, dirs_exist_ok=True,
                        ignore=shutil.ignore_patterns("layout-generators.js", "layout-generators",
                                                     "preview-generator", "bundled-step"))
        shutil.copy2(provider_root / "catalogue/modules/imported-modules.json",
                     assets / "imported-modules.json")
    shutil.copytree(provider_root / "cad/wasm/pkg", assets / "cad", dirs_exist_ok=True)
    shutil.copytree(provider_root / "web/target/providers/cad-worker", assets / "cad-worker",
                    dirs_exist_ok=True)
    shutil.copytree(provider_root / "renderer/pkg", assets / "renderer", dirs_exist_ok=True)
    baked_loader = _stage_baked_step(assets / "cad-worker", provider_root)
    loader_argument = ", loadBakedStep" if baked_loader else ""
    (assets / "cad-worker/entry.js").write_text(
        'import init, { start_cad_worker } from "./m1_cad_worker.js";\n'
        + baked_loader
        + 'await init();\n'
        + f'start_cad_worker(new URL("../cad/boardstudio_cad_wasm.js", import.meta.url).href{loader_argument});\n'
    )


def copy_runtime_assets(site: Path, provider_root: Path | None = None) -> None:
    _stage_provider_entries(site / "assets", provider_root or ROOT, include_site_assets=True)


def stage_runtime_assets(provider_root: Path = ROOT) -> None:
    """Copy generated providers into the asset directory watched by dx serve."""
    _stage_provider_entries(provider_root / "web/assets", provider_root)


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
        command[3:3] = ["--profile", "wasm-check", "--debug-symbols", "false"]
    return command, WEB


def locked_wasm_bindgen() -> Path | None:
    """The `wasm-bindgen` CLI at Cargo.lock's version, as `dx build` would select it."""
    version = wasm_bindgen_tools.locked_version(ROOT)
    candidates = []
    if directory := os.environ.get(wasm_bindgen_tools.DIRECTORY_ENV):
        candidates.append(Path(directory) / "wasm-bindgen")
    if found := shutil.which("wasm-bindgen"):
        candidates.append(Path(found))
    candidates.append(DX_TOOLS / f"wasm-bindgen-{version}/wasm-bindgen")
    cache = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    candidates += sorted(cache.glob(".wasm-pack/wasm-bindgen-*/wasm-bindgen"))
    for path in candidates:
        try:
            result = subprocess.run([str(path), "--version"], capture_output=True, text=True)
        except OSError:
            continue
        if result.stdout.strip() == f"wasm-bindgen {version}":
            return path
    return None


def matching_dx() -> str | None:
    """The `dx` CLI matching the plain-page template version, or None to fall back to `dx build`."""
    dx = shutil.which("dx")
    if not dx:
        return None
    try:
        result = subprocess.run([dx, "--version"], capture_output=True, text=True)
    except OSError:
        return None
    if result.returncode == 0 and result.stdout.strip().split()[:2] == ["dioxus", PLAIN_PAGE_DX_VERSION]:
        return dx
    return None


def page_tools(quiet: bool = False) -> tuple[Path, Path, str] | None:
    """Tools for the plain-Cargo checked page, or None to fall back to `dx build`.

    `quiet` skips the fallback notice; check.py probes this to decide whether the
    overlapped steps may share one up-front stylesheet.
    """
    tools = {"standalone tailwindcss": tailwind_cli(), "locked wasm-bindgen": locked_wasm_bindgen(),
             f"dx {PLAIN_PAGE_DX_VERSION}": matching_dx()}
    if missing := [name for name, path in tools.items() if not path]:
        if not quiet:
            print(f"build-web: {', '.join(missing)} unavailable; building the checked page with dx",
                  file=sys.stderr, flush=True)
        return None
    tailwind, bindgen, dx = tools.values()
    return tailwind, bindgen, dx


def generate_css(tailwind: Path) -> None:
    """Generate the stylesheet the page's `include_str!`s and WASM Clippy read, as dx would."""
    print(f"build-web: stylesheet via {tailwind}", flush=True)
    run([str(tailwind), "-i", "tailwind.css", "-o", "assets/tailwind.css"], cwd=WEB)


def cargo_page_artifact(output=None) -> Path:
    out = output or sys.stdout
    print("$ " + " ".join(PAGE_CARGO), file=out, flush=True)
    fd = browser_tools._inherited_build_lock()
    result = subprocess.run(PAGE_CARGO, cwd=ROOT, text=True, capture_output=True,
                            pass_fds=() if fd is None else (fd,))
    artifacts = [Path(executable) for message in cargo_json.messages(result.stdout, out)
                 if (executable := cargo_json.wasm_executable(message, kind="bin", name="boardstudio-web"))
                 and Path(message["manifest_path"]) == WEB / "Cargo.toml"]
    out.write(result.stderr)
    out.flush()
    if result.returncode:
        raise subprocess.CalledProcessError(result.returncode, PAGE_CARGO)
    if len(artifacts) != 1 or not artifacts[0].is_file():
        raise FileNotFoundError(f"expected one existing boardstudio-web wasm artifact from Cargo, got {artifacts}")
    return artifacts[0]


def index_html(base_path: str) -> str:
    asset_root = base_path.strip("/")
    # Dioxus resolves assets under `/<root>/assets/`; an empty root would give `//assets/`.
    meta = f'\n        <meta name="DIOXUS_ASSET_ROOT" content="{asset_root}">' if asset_root else ""
    return (
        "<!DOCTYPE html>\n<html>\n    <head>\n        <title>BoardStudio</title>\n"
        '        <meta content="text/html;charset=utf-8" http-equiv="Content-Type">\n'
        '        <meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f'        <meta charset="UTF-8">{meta}\n    </head>\n    <body>\n        <div id="main"></div>\n'
        f'        <script type="module" async src="{base_path}wasm/boardstudio-web.js"></script>\n'
        "    </body>\n</html>\n"
    )


def plain_page_cargo(public: Path = PAGE_PUBLIC, output=None) -> Path:
    """Compile the checked page with plain Cargo and stage its module for `plain_page_stage`.

    This is the page build's last Cargo invocation; everything after it is file staging that
    holds no Cargo lock, so check.py can overlap it with the wasm test prebuilds.
    """
    artifact = cargo_page_artifact(output)
    public.mkdir(parents=True, exist_ok=True)
    # `dx tools assets` writes the bundled asset paths into the module it reads, so leave Cargo's alone.
    module = public.parent / "boardstudio-web.wasm"
    shutil.copy2(artifact, module)
    return module


def plain_page_stage(route: str, tools: tuple[Path, Path, str], public: Path = PAGE_PUBLIC,
                     output=None) -> None:
    """Run `dx tools assets`, wasm-bindgen and the Dioxus loader for a staged page module.

    No Cargo runs here; the module must come from `plain_page_cargo` (or `plain_page`).
    """
    _tailwind, bindgen, dx = tools
    module = public.parent / "boardstudio-web.wasm"
    if not module.is_file():
        raise FileNotFoundError(f"checked page module missing; compile it before staging: {module}")
    public.mkdir(parents=True, exist_ok=True)
    assets = public / "assets"
    run([dx, "tools", "assets", str(module), str(assets)], cwd=WEB, output=output)
    if not assets.is_dir() or not any(assets.iterdir()):
        raise FileNotFoundError(f"dx tools assets extracted no files into {assets}")
    # dx's checked page has no name section either; it would triple the module the browser loads.
    run([str(bindgen), "--target", "web", "--no-typescript", "--remove-name-section",
         "--out-dir", str(public / "wasm"), "--out-name", "boardstudio-web", str(module)],
        env={**os.environ, "RAYON_NUM_THREADS": "1"}, output=output)
    script = public / "wasm/boardstudio-web.js"
    module_js = script.read_text()
    if missing := [binding for binding in BINDGEN_BINDINGS if not re.search(binding, module_js, re.MULTILINE)]:
        raise ValueError(f"{script} lacks the wasm-bindgen bindings the Dioxus loader uses: {missing}")
    script.write_text(module_js.rstrip("\n") + "\n" + DX_FOOTER.format(base_path=ROUTES[route]))
    (public / "index.html").write_text(index_html(ROUTES[route]))


def plain_page(route: str, tools: tuple[Path, Path, str], public: Path = PAGE_PUBLIC, output=None) -> None:
    """Build the checked page with plain Cargo, so no `dx` rustc wrapper splits its build units."""
    plain_page_cargo(public, output)
    plain_page_stage(route, tools, public, output)


def package_route(output: Path, public: Path, route: str) -> Path:
    """Copy a built `public` directory into the route's site with runtime assets and the offline worker."""
    if not public.is_dir():
        raise FileNotFoundError(f"the page build did not produce its web public directory: {public}")
    site = route_site(output, route)
    if site.exists():
        shutil.rmtree(site)
    site.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(public, site)
    copy_runtime_assets(site)
    embed_offline_worker(site)
    return site


def phased_build(output: Path, *, routes: tuple[str, ...], fast: bool, release: bool,
                 prepare_providers: bool, phase: str) -> Path:
    """The checked page build split so check.py can overlap post-Cargo staging with test prebuilds.

    `cargo` runs everything through the last Cargo invocation (stylesheet, providers, page
    compile); `stage` runs only file operations after it (`dx tools assets`, wasm-bindgen,
    provider staging, route packaging). Only the checked single-route provider build splits;
    anything else, the `dx build` fallback included, stays in `build()`.
    """
    if phase not in ("cargo", "stage"):
        raise ValueError(f"--phase must be cargo or stage, got {phase!r}")
    if not (fast and not release and prepare_providers and len(routes) == 1):
        raise ValueError("--phase needs the checked single-route provider build (--fast with providers)")
    tools = page_tools()
    if tools is None:
        raise ValueError("--phase needs the plain-page tools; without them the page builds "
                         "with dx and cannot split")
    public = PAGE_PUBLIC
    if phase == "cargo":
        if public.exists():
            shutil.rmtree(public)
        # Beside lint and test, check.py has already written the stylesheet WASM Clippy is reading.
        if IN_OVERLAP_ENV not in os.environ:
            generate_css(tools[0])
        if check_jobs() > 1:
            page_build = lambda lane_output: plain_page_cargo(public, lane_output)
            prepare_runtime_providers(fast, page_build=page_build, defer_staging=True)
        else:
            prepare_runtime_providers(fast, defer_staging=True)
            plain_page_cargo(public)
        module = public.parent / "boardstudio-web.wasm"
        print(module, flush=True)
        return output
    module = public.parent / "boardstudio-web.wasm"
    if not module.is_file():
        raise FileNotFoundError(f"--phase stage needs --phase cargo first (missing {module})")
    # Content-hashed asset names would leave stale files beside fresh ones without a clean directory.
    if public.exists():
        shutil.rmtree(public)
    plain_page_stage(routes[0], tools, public)
    finish_provider_staging()
    package_route(output, public, routes[0])
    print(output, flush=True)
    return output


def build(output: Path, *, release: bool = True, prepare_providers: bool = True,
          routes: tuple[str, ...] = DEFAULT_ROUTES, fast: bool = False,
          phase: str | None = None) -> Path:
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if phase is not None:
        return phased_build(output, routes=routes, fast=fast, release=release,
                            prepare_providers=prepare_providers, phase=phase)
    tools = page_tools() if fast and not release else None
    public = PAGE_PUBLIC if tools else dx_public(release)
    overlap = fast and not release and prepare_providers and len(routes) == 1 and check_jobs() > 1
    if overlap and public.exists():
        shutil.rmtree(public)
    # Beside lint and test, check.py has already written the stylesheet WASM Clippy is reading.
    if tools and IN_OVERLAP_ENV not in os.environ:
        generate_css(tools[0])
    if prepare_providers:
        page_build = None
        if overlap and tools:
            page_build = lambda lane_output: plain_page(routes[0], tools, public, lane_output)
        elif overlap:
            page_build = dx_command(routes[0], release=release, fast=fast)
        prepare_runtime_providers(fast, page_build=page_build)
    for route in routes:
        if not overlap:
            if public.exists():
                shutil.rmtree(public)
            if tools:
                plain_page(route, tools, public)
            else:
                command, cwd = dx_command(route, release=release, fast=fast)
                run(command, cwd=cwd)
        package_route(output, public, route)
    print(output, flush=True)
    return output


def finish_provider_staging() -> None:
    """Copy provider metadata into the asset directory; the staging half `defer_staging` skips."""
    shutil.copy2(ROOT / "catalogue/modules/imported-modules.json",
                 WEB / "assets/imported-modules.json")
    stage_runtime_assets()


def prepare_runtime_providers(fast: bool = False, *, page_build=None, defer_staging: bool = False) -> None:
    """Build the providers; `page_build`, a step for `run_lanes`, overlaps them with the CAD build.

    `defer_staging` leaves the trailing asset copies for `--phase stage`, which runs them beside
    the wasm test prebuilds instead of behind the Cargo lock.
    """
    commands = provider_commands(fast)
    bake = (["node", "cad/scripts/bake-bundled-step.cjs"], ROOT)
    if page_build is None:
        for command, cwd in [*commands, bake]:
            run(command, cwd=cwd)
    else:
        # Prepare Ergogen before the page build sees the asset directory. The wasm-pack providers
        # and the page share the `wasm-check` target directory lock, so they run in one lane.
        wasm_packs, cad, ergogen = commands[:-2], commands[-2], commands[-1]
        command, cwd = ergogen
        run(command, cwd=cwd)
        run_lanes([[*wasm_packs, page_build], [cad, bake]])
    if defer_staging:
        return
    finish_provider_staging()


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
    parser.add_argument("--phase", choices=("cargo", "stage"), default=None,
                        help="split the checked build so check.py overlaps post-Cargo staging with the "
                             "wasm test prebuilds: cargo stops after the last Cargo invocation, stage "
                             "runs only the file staging after it (single-route --fast with providers)")
    args = parser.parse_args(argv)
    routes = tuple(dict.fromkeys(name.strip() for name in args.routes.split(",") if name.strip()))
    unknown = [name for name in routes if name not in ROUTES]
    if unknown or not routes:
        parser.error(f"--routes must name one or more of: {', '.join(ROUTES)}")
    if args.phase and (args.providers_only or args.skip_providers or not args.fast or len(routes) != 1):
        parser.error("--phase needs the checked single-route provider build (--fast with providers)")
    try:
        if args.providers_only:
            prepare_runtime_providers(args.fast)
        else:
            build(args.output, release=not (args.debug or args.fast),
                  prepare_providers=not args.skip_providers, routes=routes, fast=args.fast,
                  phase=args.phase)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"build-web: {error}", file=sys.stderr)
        return error.returncode if isinstance(error, subprocess.CalledProcessError) else 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
