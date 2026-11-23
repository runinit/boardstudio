#!/usr/bin/env python3
"""Build BoardStudio's web providers and static Dioxus sites."""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import base64
import hashlib
import time
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import browser_tools
import cargo_json
from tailwind_tools import DX_TOOLS, IN_OVERLAP_ENV, check_jobs, tailwind_cli
import wasm_bindgen_tools

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "web"
OFFLINE_WORKER = WEB / "target/providers/offline-worker"
ROUTES = {"subpath": "/boardstudio/", "root": "/"}
DEFAULT_ROUTES = ("subpath",)
MODULE_CATALOGUE = "catalogue/modules/imported-modules.json"
ON_DEMAND_PREFIXES = ("assets/ergogen-models/", "assets/cad/", "assets/cad-worker/bundled-step/", "assets/modules/")
# Cargo config already passes the getrandom WASM cfg to every build.
DX_FLAGS = ("--wasm-js-cfg", "false")
PAGE_TARGET = WEB / "target/page"
PLAIN_PAGE_DX_VERSION = "0.7.10"
PAGE_CARGO = ["cargo", "build", "--locked", "--target", "wasm32-unknown-unknown", "--profile", "wasm-check",
              "-p", "boardstudio-web", "--bin", "boardstudio-web", "--no-default-features",
              "--features", "page", "--message-format=json"]
# Dioxus web auto-init, as `dx build` appends it to the wasm-bindgen `--target web` module.
DX_FOOTER = """export {{ wasm as __wasm }}
globalThis.__wasm_split_main_initSync = initSync;
__wbg_init({{module_or_path: "{base_path}wasm/boardstudio-web_bg.wasm"}}).then((wasm) => {{
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


def run(command, *, cwd=ROOT, env=None):
    print("$ " + " ".join(command), flush=True)
    fd = browser_tools._inherited_build_lock()
    subprocess.run(command, cwd=cwd, env=env, check=True, pass_fds=() if fd is None else (fd,))


def worker_providers():
    return (
        ("m1_cad_worker", WEB / "target/providers/cad-worker", "cad-worker", ROOT / "target/wasm-workers"),
        ("boardstudio_offline_worker", OFFLINE_WORKER, "service-worker", ROOT / "target/wasm-offline-worker"),
    )


def provider_commands(fast: bool = False) -> list[list[str]]:
    profile = ["--profile", "wasm-check", "--no-opt"] if fast else ["--release"]
    return [
        *[["wasm-pack", "build", str(WEB), "--target", "web", "--out-name", name,
           "--out-dir", str(output), *profile, "--locked", "--no-default-features",
           "--features", feature] for name, output, feature, _ in worker_providers()],
        ["wasm-pack", "build", "renderer", "--target", "web", "--out-dir", "pkg",
         "--out-name", "boardstudio_renderer_wasm", *profile, "--locked"],
        [sys.executable, "cad/scripts/build-cad-wasm.py", *(["--no-opt"] if fast else [])],
        [sys.executable, "scripts/stage-ergogen-models.py", "--source-root",
          "ergogen/library/vendor", "--destination", "web/assets/ergogen-models",
          "--manifest", "web/target/providers/ergogen-models.json"],
    ]


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


def stage_module_catalogue(source: Path, assets: Path) -> None:
    package = json.loads(source.read_text())
    if package["formatVersion"] != 1:
        raise ValueError(f"unsupported module source format: {package['formatVersion']}")
    records = package["modules"]
    host = next((part for record in records for part in record["definition"].get("circuit", {}).get("definitions", [])
                 if part.get("hardwareProfile", {}).get("vikRole") == "host" and "horizontal" in part["name"].lower()), None)
    if host is None:
        raise ValueError("module catalogue has no horizontal VIK host connector")
    modules = assets / "modules"
    stale = [path for path in (assets / "imported-modules.json", modules) if path.exists()]
    if stale:
        trash = Path.home() / ".cache/tmp/boardstudio-trash/p0-1"
        trash.mkdir(parents=True, exist_ok=True)
        backup = Path(tempfile.mkdtemp(dir=trash))
        for path in stale:
            shutil.move(path, backup / path.name)
    modules.mkdir(parents=True)
    host_asset = "assets/modules/vik-host-connector.json"
    compact = json.JSONEncoder(separators=(",", ":")).encode
    (modules / Path(host_asset).name).write_text(compact(host))
    entries = []
    for record in records:
        definition = record["definition"]
        if not re.fullmatch(r"[A-Za-z0-9:-]+", definition["id"]):
            raise ValueError(f"module id cannot be staged: {definition['id']}")
        output = modules / (definition["id"].replace(":", "_") + ".json")
        with output.open("x") as file:
            file.write(compact(record))
        points = [point for contour in definition["board"]["contours"] for point in contour["points"]]
        bounds = [[extent(point[axis] for point in points) for axis in "xy"] for extent in (min, max)] if points else None
        entries.append({**{key: definition[key] for key in ("id", "name", "family", "variant")},
                        "row": record["row"], "asset": f"assets/modules/{output.name}", "bounds": bounds,
                        "protocol": definition["electrical"]["protocol"], "gateCount": len(definition.get("gates", [])),
                        "pinoutSummary": sorted({signal for port in definition.get("interfaces", []) for signal in port["signals"]})})
    index = compact({"formatVersion": 1, "hostConnectorAsset": host_asset, "modules": entries})
    if len(index) >= 200 * 1024:
        raise ValueError("module catalogue index must be smaller than 200 KiB")
    (assets / "modules-index.json").write_text(index)


def _stage_provider_entries(assets: Path, provider_root: Path, *, include_site_assets: bool = False) -> None:
    """Copy the generated providers into `assets` and write the worker entry modules."""
    if include_site_assets:
        shutil.copytree(provider_root / "web/assets", assets, dirs_exist_ok=True,
                        ignore=shutil.ignore_patterns("layout-generators.js", "layout-generators",
                                                     "preview-generator", "bundled-step", "modules",
                                                     "modules-index.json", "imported-modules.json"))
    stage_module_catalogue(provider_root / MODULE_CATALOGUE, assets)
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


def validate_offline_manifest(manifest: dict) -> None:
    version, shell, prefixes = (manifest[key] for key in ("version", "shellAssets", "runtimeCachePrefixes"))
    if not re.fullmatch(r"[A-Za-z0-9_.-]+", version) or "index.html" not in shell:
        raise ValueError("offline manifest requires a valid version and index.html")
    if (len(set(shell)) != len(shell) or len(set(prefixes)) != len(prefixes)
            or any(not prefix.endswith("/") for prefix in prefixes)
            or any(path.startswith(tuple(prefixes)) for path in shell)):
        raise ValueError("offline manifest has duplicate, malformed or overlapping asset lists")
    for path in [*shell, *(prefix.rstrip("/") for prefix in prefixes)]:
        if re.search(r"[\\?#:%]", path) or any(segment in ("", ".", "..") for segment in path.split("/")):
            raise ValueError(f"offline asset path is not normalized and scope-relative: {path}")


def embed_offline_worker(site: Path, worker_dir: Path = OFFLINE_WORKER) -> None:
    js_name = "boardstudio_offline_worker.js"
    wasm = (worker_dir / "boardstudio_offline_worker_bg.wasm").read_bytes()
    shutil.copy2(worker_dir / js_name, site / js_name)
    assets = sorted(path.relative_to(site).as_posix() for path in site.rglob("*") if path.is_file())
    digest = hashlib.sha256(wasm)
    for relative in assets:
        digest.update(relative.encode())
        digest.update((site / relative).read_bytes())
    shell = sorted({relative for relative in assets if not relative.startswith(ON_DEMAND_PREFIXES)} | {"service-worker.js"})
    manifest = {"version": f"boardstudio-{digest.hexdigest()[:16]}-{time.time_ns()}",
                "shellAssets": shell, "runtimeCachePrefixes": list(ON_DEMAND_PREFIXES)}
    validate_offline_manifest(manifest)
    arguments = ", ".join(map(json.dumps, (manifest["version"], shell, manifest["runtimeCachePrefixes"])))
    encoded = base64.b64encode(wasm).decode("ascii")
    (site / "service-worker.js").write_text(
        "/* Offline cache policy is implemented in Rust. */\n"
        f'import {{ initSync, start_offline_worker }} from "./{js_name}";\n'
        f'initSync({{ module: Uint8Array.from(atob("{encoded}"), c => c.charCodeAt(0)) }});\n'
        f"start_offline_worker({arguments});\n"
    )


def dx_command(route: str, *, release: bool, fast: bool) -> list[str]:
    command = ["dx", "build", "--web", "--base-path", ROUTES[route], *DX_FLAGS,
               "--no-default-features", "--features", "page", "--cargo-args=--locked"]
    if release:
        command.insert(3, "--release")
    elif fast:
        # The checked build shares the `wasm-check` profile with every wasm test build.
        command[3:3] = ["--profile", "wasm-check", "--debug-symbols", "false"]
    return command


def locked_wasm_bindgen() -> Path | None:
    """The `wasm-bindgen` CLI at Cargo.lock's version, as `dx build` would select it."""
    version = wasm_bindgen_tools.locked_version(ROOT)
    return wasm_bindgen_tools.find_bindgen(ROOT, [DX_TOOLS / f"wasm-bindgen-{version}/wasm-bindgen"])


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
    tailwind, bindgen, dx = tailwind_cli(), locked_wasm_bindgen(), matching_dx()
    if tailwind and bindgen and dx:
        return tailwind, bindgen, dx
    if not quiet:
        print(f"build-web: plain page needs Tailwind, locked wasm-bindgen and dx {PLAIN_PAGE_DX_VERSION}; "
              "using dx build", file=sys.stderr)
    return None


def cargo_page_artifact() -> Path:
    out = sys.stdout
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


def plain_page(route: str, tools: tuple[Path, Path, str], public: Path) -> None:
    _tailwind, bindgen, dx = tools
    artifact = cargo_page_artifact()
    dx, bindgen = Path(dx).resolve(), bindgen.resolve()
    with artifact.open("rb") as wasm:
        digest = hashlib.file_digest(wasm, "sha256")
    # Versions were validated by page_tools. Include timestamps for same-version tool replacements
    # and the packaging script for changes to the generated loader, HTML, or tool flags.
    digest.update(json.dumps([
        route, str(dx), PLAIN_PAGE_DX_VERSION, dx.stat().st_mtime_ns,
        str(bindgen), wasm_bindgen_tools.locked_version(ROOT), bindgen.stat().st_mtime_ns,
    ]).encode())
    digest.update(Path(__file__).read_bytes())
    key = digest.hexdigest().encode("ascii")
    stamp = public.parent / "plain-page.sha256"
    assets = public / "assets"
    script = public / "wasm/boardstudio-web.js"
    bound_wasm = public / "wasm/boardstudio-web_bg.wasm"
    index = public / "index.html"
    if (stamp.is_file() and stamp.read_bytes() == key and assets.is_dir() and any(assets.iterdir())
            and all(path.is_file() for path in (script, bound_wasm, index))):
        print(f"build-web: reusing plain page for {ROUTES[route]}", flush=True)
        return

    # A failed extraction or bindgen run must leave no reusable stamp.
    stamp.unlink(missing_ok=True)
    if public.exists():
        shutil.rmtree(public)
    module = public.parent / "boardstudio-web.wasm"
    public.mkdir(parents=True, exist_ok=True)
    # Dioxus extracts assets by rewriting the module, so pass it a copy.
    shutil.copy2(artifact, module)
    run([str(dx), "tools", "assets", str(module), str(assets)], cwd=WEB)
    if not assets.is_dir() or not any(assets.iterdir()):
        raise FileNotFoundError(f"dx tools assets extracted no files into {assets}")
    # dx's checked page has no name section either; it would triple the module the browser loads.
    run([str(bindgen), "--target", "web", "--no-typescript", "--remove-name-section",
         "--out-dir", str(public / "wasm"), "--out-name", "boardstudio-web", str(module)],
        env={**os.environ, "RAYON_NUM_THREADS": "1"})
    if not bound_wasm.is_file():
        raise FileNotFoundError(f"wasm-bindgen did not generate {bound_wasm}")
    module_js = script.read_text()
    if missing := [binding for binding in BINDGEN_BINDINGS if not re.search(binding, module_js, re.MULTILINE)]:
        raise ValueError(f"{script} lacks the wasm-bindgen bindings the Dioxus loader uses: {missing}")
    script.write_text(module_js.rstrip("\n") + "\n" + DX_FOOTER.format(base_path=ROUTES[route]))
    index.write_text(index_html(ROUTES[route]))
    stamp.write_bytes(key)


def snapshot(paths):
    result = []
    for root in paths:
        if not root.exists():
            result.append([str(root), None])
            continue
        files = sorted(root.rglob("*")) if root.is_dir() else [root]
        for file in files:
            if file.is_file():
                stat = file.stat()
                # Generated entry files are rewritten even when their bytes are unchanged.
                identity = hashlib.sha256(file.read_bytes()).hexdigest() if file.name == "entry.js" else stat.st_mtime_ns
                result.append([str(file), stat.st_size, identity])
    return result


def stamp_matches(stamp, expected):
    try:
        return json.loads(stamp.read_text()) == expected()
    except (OSError, ValueError):
        return False


def package_route(output: Path, public: Path, route: str, *, fast=False) -> None:
    site = output / ("site-root" if route == "root" else "site-subpath/boardstudio")
    stamp = output / f".package-{route}.json"
    inputs = [public, WEB / "assets", OFFLINE_WORKER, *provider_inputs()]
    current = snapshot(inputs) if fast else None
    if fast and stamp.is_file() and site.is_dir() and stamp_matches(
            stamp, lambda: {"inputs": current, "site": snapshot([site])}):
        return
    stamp.unlink(missing_ok=True)
    if site.exists():
        shutil.rmtree(site)
    shutil.copytree(public, site)
    _stage_provider_entries(site / "assets", ROOT, include_site_assets=True)
    embed_offline_worker(site)
    if fast:
        stamp.write_text(json.dumps({"inputs": current, "site": snapshot([site])}))


def fast_provider_stamp(name):
    return WEB / "target/providers" / ("." + name + ".bindgen.sha256")


def fast_provider(name, output, package, feature, target, bindgen):
    """Build the Rust library every time, but bind only changed WASM bytes."""
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    cargo = ["cargo", "build", "--locked", "--target", "wasm32-unknown-unknown",
             "--profile", "wasm-check", "--lib", "-p", package]
    if feature:
        cargo += ["--no-default-features", "--features", feature]
    run(cargo, env=env)
    artifact = target / "wasm32-unknown-unknown/wasm-check" / (package.replace("-", "_") + ".wasm")
    args = ["--target", "web", "--out-dir", str(output), "--out-name", name]
    digest = hashlib.sha256(artifact.read_bytes())
    digest.update(bindgen.read_bytes())
    digest.update(json.dumps(args).encode())
    stamp = fast_provider_stamp(name)
    key = digest.hexdigest()
    products = (output / (name + ".js"), output / (name + "_bg.wasm"))

    def product_hashes():
        return {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in products}

    if stamp.is_file() and all(path.is_file() for path in products) and stamp_matches(
            stamp, lambda: {"key": key, "outputs": product_hashes()}):
        return
    stamp.unlink(missing_ok=True)
    output.mkdir(parents=True, exist_ok=True)
    run([str(bindgen), *args, str(artifact)])
    if not all(path.is_file() for path in products):
        raise FileNotFoundError(f"wasm-bindgen did not generate {products}")
    stamp.parent.mkdir(parents=True, exist_ok=True)
    stamp.write_text(json.dumps({"key": key, "outputs": product_hashes()}))


def bake_bundled_step():
    """Skip Node only when all bake inputs and generated sidecars match the last run."""
    providers = WEB / "target/providers"
    manifest = providers / "bundled-step/manifest.json"
    stamp = providers / ".bundled-step.sha256"

    def fingerprint():
        inventory = json.loads((providers / "ergogen-models.json").read_text())
        assets = json.loads(manifest.read_text())["assets"]
        files = [ROOT / "cad/scripts/bake-bundled-step.cjs", ROOT / "cad/wasm/tests/load-provider.cjs",
                 ROOT / "cad/wasm/pkg/boardstudio_cad_wasm.js",
                 ROOT / "cad/wasm/pkg/boardstudio_cad_wasm_bg.wasm", providers / "ergogen-models.json", manifest]
        files += [Path(inventory["destination"]) / Path(entry["emittedPath"]).name
                  for entry in inventory["entries"] if entry.get("mediaType") == "model/step"]
        files += [manifest.parent / (value + ".bstep") for value in assets.values()]
        digest = hashlib.sha256()
        for file in sorted(set(files)):
            digest.update(str(file).encode())
            digest.update(hashlib.sha256(file.read_bytes()).digest())
        return digest.hexdigest()

    try:
        if stamp.is_file() and stamp.read_text() == fingerprint():
            return
    except (OSError, ValueError, KeyError, TypeError):
        pass
    stamp.unlink(missing_ok=True)
    run(["node", "cad/scripts/bake-bundled-step.cjs"])
    stamp.write_text(fingerprint())


def provider_inputs():
    return [ROOT / "cad/wasm/pkg", ROOT / "renderer/pkg", WEB / "target/providers/cad-worker",
            WEB / "target/providers/bundled-step", ROOT / MODULE_CATALOGUE, Path(__file__)]


def prepare_runtime_providers(fast=False):
    commands = provider_commands(fast)
    bindgen = locked_wasm_bindgen() if fast else None
    targets = {name: target for name, _, _, target in worker_providers()}
    handled = set()
    if bindgen:
        providers = [(name, output, "boardstudio-web", feature, target)
                     for name, output, feature, target in worker_providers()]
        providers.append(("boardstudio_renderer_wasm", ROOT / "renderer/pkg", "boardstudio-renderer", None, ROOT / "target"))
        def build_provider(provider):
            fast_provider(*provider, bindgen)
        with ThreadPoolExecutor(max_workers=min(check_jobs(), len(providers))) as pool:
            list(pool.map(build_provider, providers))
        handled = {name for name, *_ in providers}
    for command in commands:
        if command[:2] != ["wasm-pack", "build"]:
            run(command)
            continue
        name = command[command.index("--out-name") + 1]
        if name in handled:
            continue
        fast_provider_stamp(name).unlink(missing_ok=True)
        if name in targets:
            run(command, env=dict(os.environ, CARGO_TARGET_DIR=str(targets[name])))
        else:
            run(command)
    bake_bundled_step()
    outputs = [WEB / "assets" / name for name in ("cad", "renderer", "cad-worker", "modules-index.json", "modules")]
    stamp = WEB / "target/providers/.staged-assets.json"
    current = snapshot(provider_inputs()) if fast else None
    if fast and stamp.is_file() and stamp_matches(stamp, lambda: {"inputs": current, "outputs": snapshot(outputs)}):
        return
    stamp.unlink(missing_ok=True)
    _stage_provider_entries(WEB / "assets", ROOT)
    if fast:
        stamp.parent.mkdir(parents=True, exist_ok=True)
        stamp.write_text(json.dumps({"inputs": current, "outputs": snapshot(outputs)}))


def build(output, *, release=True, prepare_providers=True, routes=DEFAULT_ROUTES, fast=False):
    output = output.resolve()
    tools = page_tools() if fast and not release else None
    profile = "release" if release else "debug"
    if prepare_providers:
        prepare_runtime_providers(fast)
    if tools and IN_OVERLAP_ENV not in os.environ:
        run([str(tools[0]), "-i", "tailwind.css", "-o", "assets/tailwind.css"], cwd=WEB)
    for route in routes:
        if tools:
            public = PAGE_TARGET / route / "public"
            plain_page(route, tools, public)
        else:
            public = ROOT / f"target/dx/boardstudio-web/{profile}/web/public"
            if public.exists():
                shutil.rmtree(public)
            run(dx_command(route, release=release, fast=fast), cwd=WEB)
        package_route(output, public, route, fast=fast)
    print(output, flush=True)
    return output


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "web/target/site",
                        help="directory for site-root and site-subpath builds")
    parser.add_argument("--debug", action="store_true", help="build Dioxus without release optimization")
    parser.add_argument("--fast", action="store_true", help="checked build without wasm-opt")
    parser.add_argument("--skip-providers", action="store_true",
                        help="reuse already generated provider files in the working tree")
    parser.add_argument("--routes", default=",".join(DEFAULT_ROUTES),
                        help="comma-separated routes: subpath (/boardstudio/) and/or root (/)")
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
