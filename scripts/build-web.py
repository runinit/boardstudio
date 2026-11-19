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
from check import check_jobs, OVERLAP_CSS
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
TAILWIND_ENV = "BOARDSTUDIO_TAILWINDCSS"
DX_TOOLS = Path.home() / ".local/share/.dx/tools"
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
    if command[:2] == ["dx", "build"]:
        signal_css_ready(cwd)


def signal_css_ready(web: Path | None = None) -> None:
    """Unblock check.py's WASM Clippy once the generated stylesheet exists."""
    if css_sync := os.environ.get(OVERLAP_CSS):
        css = (web or WEB) / "assets/tailwind.css"
        if css.is_file() and css.stat().st_size:
            (Path(css_sync) / "ready").touch()


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
        # Every checked wasm build, the page included, shares the `wasm-check` profile
        # (dev semantics instead of release; `check.py release` and CI keep `--release` and `wasm-opt`).
        for command, _ in commands:
            if command[:2] == ["wasm-pack", "build"]:
                command[command.index("--release"):command.index("--release") + 1] = [
                    "--profile", "wasm-check"]
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
        command[3:3] = ["--profile", "wasm-check", "--debug-symbols", "false"]
    return command, WEB


def tailwind_cli() -> Path | None:
    """A standalone Tailwind v4 CLI: $BOARDSTUDIO_TAILWINDCSS, PATH, then the newest dx-installed one."""
    def is_v4(path: Path) -> bool:
        try:
            result = subprocess.run([str(path), "--help"], capture_output=True, text=True)
        except OSError:
            return False
        return "tailwindcss v4." in result.stdout + result.stderr

    if explicit := os.environ.get(TAILWIND_ENV):
        if not is_v4(Path(explicit)):
            raise ValueError(f"{TAILWIND_ENV}={explicit} is not a standalone Tailwind v4 CLI")
        return Path(explicit)
    candidates = [Path(found)] if (found := shutil.which("tailwindcss")) else []
    installed = [path / "tailwindcss" if path.is_dir() else path for path in DX_TOOLS.glob("tailwindcss-*")]
    candidates += sorted((path for path in installed if path.is_file() and os.access(path, os.X_OK)),
                         key=lambda path: path.stat().st_mtime, reverse=True)
    return next((path for path in candidates if is_v4(path)), None)


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


def page_tools() -> tuple[Path, Path, str] | None:
    """Tools for the plain-Cargo checked page, or None to fall back to `dx build`."""
    tools = {"standalone tailwindcss": tailwind_cli(), "locked wasm-bindgen": locked_wasm_bindgen(),
             f"dx {PLAIN_PAGE_DX_VERSION}": matching_dx()}
    if missing := [name for name, path in tools.items() if not path]:
        print(f"build-web: {', '.join(missing)} unavailable; building the checked page with dx",
              file=sys.stderr, flush=True)
        return None
    tailwind, bindgen, dx = tools.values()
    return tailwind, bindgen, dx


def generate_css(tailwind: Path) -> None:
    """Generate the stylesheet the page's `include_str!`s and WASM Clippy read, as dx would."""
    print(f"build-web: stylesheet via {tailwind}", flush=True)
    run([str(tailwind), "-i", "tailwind.css", "-o", "assets/tailwind.css"], cwd=WEB)
    signal_css_ready()


def cargo_page_artifact(output=None) -> Path:
    out = output or sys.stdout
    print("$ " + " ".join(PAGE_CARGO), file=out, flush=True)
    fd = browser_tools._inherited_build_lock()
    result = subprocess.run(PAGE_CARGO, cwd=ROOT, text=True, capture_output=True,
                            pass_fds=() if fd is None else (fd,))
    artifacts = []
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            print(line, file=out)
            continue
        if message.get("reason") == "compiler-message":
            if rendered := message["message"].get("rendered"):
                print(rendered, end="", file=out)
        elif (message.get("reason") == "compiler-artifact" and message["target"]["kind"] == ["bin"]
              and message["target"]["name"] == "boardstudio-web"
              and Path(message["manifest_path"]) == WEB / "Cargo.toml"
              and (executable := message.get("executable")) and executable.endswith(".wasm")):
            artifacts.append(Path(executable))
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


def plain_page(route: str, tools: tuple[Path, Path, str], public: Path = PAGE_PUBLIC, output=None) -> None:
    """Build the checked page with plain Cargo, so no `dx` rustc wrapper splits its build units."""
    _tailwind, bindgen, dx = tools
    artifact = cargo_page_artifact(output)
    public.mkdir(parents=True, exist_ok=True)
    # `dx tools assets` writes the bundled asset paths into the module it reads, so leave Cargo's alone.
    module = public.parent / "boardstudio-web.wasm"
    shutil.copy2(artifact, module)
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


def build(output: Path, *, release: bool = True, prepare_providers: bool = True,
          routes: tuple[str, ...] = DEFAULT_ROUTES, fast: bool = False) -> Path:
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    tools = page_tools() if fast and not release else None
    public = PAGE_PUBLIC if tools else dx_public(release)
    overlap = fast and not release and prepare_providers and len(routes) == 1 and check_jobs() > 1
    if overlap and public.exists():
        shutil.rmtree(public)
    if tools:
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
        if not public.is_dir():
            raise FileNotFoundError(f"the page build did not produce its web public directory: {public}")
        site = route_site(output, route)
        if site.exists():
            shutil.rmtree(site)
        site.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(public, site)
        copy_runtime_assets(site)
        embed_offline_worker(site)
    print(output, flush=True)
    return output


def prepare_runtime_providers(fast: bool = False, *, page_build=None) -> None:
    """Build the providers; `page_build`, a step for `run_lanes`, overlaps them with the CAD build."""
    commands = provider_commands(fast)
    if page_build is None:
        for command, cwd in commands:
            run(command, cwd=cwd)
    else:
        # Prepare Ergogen before the page build sees the asset directory. The wasm-pack providers
        # and the page share the `wasm-check` target directory lock, so they run in one lane.
        wasm_packs, cad, ergogen = commands[:-2], commands[-2], commands[-1]
        command, cwd = ergogen
        run(command, cwd=cwd)
        run_lanes([[*wasm_packs, page_build], [cad]])
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
