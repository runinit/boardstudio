#!/usr/bin/env python3
"""Build a uniquely staged M1 candidate and retain commands/source/asset hashes."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone

REPO = Path(__file__).resolve().parents[1]
WEB = REPO / "web"
BUILD_ROOT = WEB / "target" / "builds"

# Reuse is safe only for these audited presentation inputs. Keep this exact and
# narrow: other presentation modules can be compiled into workers or tests.
PAGE_ONLY_ALLOWLIST = {
    "web/src/presentation/panels.rs",
    "web/src/presentation/panels_scroll_tests.rs",
    "web/assets/m1.css",
}
PAGE_ONLY_PROOF_PATHS = (
    "web/src/main.rs",
    "web/src/lib.rs",
    "web/src/presentation.rs",
    "web/Cargo.toml",
    "web/build.rs",
    "scripts/build-m1.py",
)
REUSED_PROVIDER_PREFIXES = (
    "assets/cad/",
    "assets/cad-worker/",
    "assets/core-worker/",
    "assets/renderer/",
    "assets/fixtures/",
    "assets/ergogen-models/",
)

# Full builds produce these generated files. Their maintained source inputs are
# still hashed; generated copies are validated as packaged assets instead.
GENERATED_SOURCE_PATHS = (
    "web/target/",
    "cad/wasm/pkg/",
    "cad/.cache/",
    "renderer/pkg/",
    "web/assets/ergogen-models/",
    "web/assets/layout-generators/",
    "web/assets/preview-generator/",
    "web/assets/layout-generators.js",
)
ROOT_BUILD_INPUTS = frozenset({
    ".cargo/config",
    ".cargo/config.toml",
    ".node-version",
    ".npmrc",
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "rust-toolchain.toml",
})


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources():
    # Include relevant untracked source files as well as tracked inputs. This
    # makes adding a module/source fail closed before it is committed. Generated
    # build outputs have explicit exclusions above and their generators remain
    # in the source inventory.
    tracked_and_unignored = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=REPO
    ).decode().split("\0")
    ignored_untracked = subprocess.check_output(
        ["git", "ls-files", "--others", "--ignored", "--exclude-standard", "-z"], cwd=REPO
    ).decode().split("\0")
    # Match the provider's recursive model enumeration, including newly added
    # source models that have not yet been staged in Git.
    vendor = REPO / "ergogen/library/vendor"
    paths = sorted(set(tracked_and_unignored) | set(ignored_untracked) | {
        path.relative_to(REPO).as_posix()
        for path in vendor.glob("*/3d_models/**/*")
        if path.is_file() and path.suffix.lower() in {".step", ".stp", ".stl", ".wrl"}
    })
    return {name: digest(REPO / name) for name in paths if name and
            not name.startswith(GENERATED_SOURCE_PATHS) and
            not {"node_modules", "__pycache__", "target", "dist"}.intersection(Path(name).parts) and
            (name.startswith(("web/", "application/", "core/", "contracts/", "renderer/", "cad/", "kicad/src/", "app/src/", "app/public/", "scripts/", "ergogen/")) or name in ROOT_BUILD_INPUTS) and (REPO / name).is_file()}


def valid_build_id(value):
    return bool(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]*", value))


def sha256_file(path):
    try:
        return digest(path)
    except OSError as error:
        raise ValueError(f"required file is missing or unreadable: {path}: {error}") from error


def current_tools():
    tools = [
        ("rustc-version", ["rustc", "--version"]),
        ("cargo-version", ["cargo", "--version"]),
        ("dx-version", ["dx", "--version"]),
        ("wasm-pack-version", ["wasm-pack", "--version"]),
        ("node-version", ["node", "--version"]),
        ("pnpm-version", ["pnpm", "--version"]),
    ]
    observed = {}
    for label, command in tools:
        try:
            observed[label] = subprocess.check_output(command, cwd=REPO, text=True, stderr=subprocess.STDOUT).strip()
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError(f"could not capture {label}: {error}") from error
    return observed


def baseline_tools(provenance, baseline):
    expected = {}
    for label in ("rustc-version", "cargo-version", "dx-version", "wasm-pack-version", "node-version", "pnpm-version"):
        matches = [row for row in provenance["commands"] if Path(row.get("log", "")).name == f"{label}.log"]
        if len(matches) != 1 or matches[0].get("exit") != 0:
            raise ValueError(f"baseline has no unique successful {label} receipt")
        log = Path(matches[0]["log"])
        if not log.is_absolute():
            log = baseline / log
        log = log.resolve()
        try:
            log.relative_to(baseline)
        except ValueError as error:
            raise ValueError(f"baseline {label} receipt escapes the build directory") from error
        sha256_file(log)  # also ensures the retained receipt exists
        try:
            expected[label] = log.read_text().strip()
        except OSError as error:
            raise ValueError(f"could not read baseline {label} receipt: {error}") from error
    return expected


def expected_full_commands(baseline):
    output = baseline
    commands = [
        ("rustc-version", ["rustc", "--version"], REPO, {}),
        ("cargo-version", ["cargo", "--version"], REPO, {}),
        ("dx-version", ["dx", "--version"], REPO, {}),
        ("wasm-pack-version", ["wasm-pack", "--version"], REPO, {}),
        ("node-version", ["node", "--version"], REPO, {}),
        ("pnpm-version", ["pnpm", "--version"], REPO, {}),
        ("core", ["wasm-pack", "build", REPO / "core", "--target", "web", "--release", "--locked"], REPO, {}),
        ("core-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_core_worker", "--out-dir", output / "core-worker", "--release", "--locked", "--no-default-features", "--features", "core-worker"], REPO, {}),
        ("cad-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_cad_worker", "--out-dir", output / "cad-worker", "--release", "--locked", "--no-default-features", "--features", "cad-worker"], REPO, {}),
        ("renderer", ["wasm-pack", "build", REPO / "renderer", "--target", "web", "--out-dir", output / "renderer", "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"], REPO, {}),
        ("cad", ["pnpm", "--dir", "cad", "run", "build:wasm"], REPO, {}),
        ("fixtures", ["node", REPO / "scripts/prepare-m1-fixtures.mjs", output / "fixtures"], REPO, {}),
        ("ergogen-catalogue", ["pnpm", "--dir", "ergogen", "run", "prepare:catalog"], REPO, {}),
        ("layout-generators", ["node", REPO / "scripts/web/build-layout-generators.mjs", WEB / "assets"], REPO, {}),
        ("preview-generator", ["node", REPO / "scripts/web/build-preview-generator.mjs", WEB / "assets"], REPO, {}),
        ("ergogen-models", [sys.executable, REPO / "scripts/stage-ergogen-models.py", "--source-root", REPO / "ergogen/library/vendor", "--destination", WEB / "assets/ergogen-models", "--manifest", output / "ergogen-models-catalog.json"], REPO, {}),
    ]
    for mode, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
        commands.append((f"page-{mode}", ["dx", "build", "--web", "--release", "--base-path", prefix, "--no-default-features", "--features", "page", "--cargo-args=--locked"], WEB, {}))
        manifest = output / f"offline-manifest-{mode}.json"
        commands.append((f"offline-worker-{mode}", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "boardstudio_offline_worker", "--out-dir", output / f"offline-{mode}", "--release", "--locked", "--no-default-features", "--features", "service-worker"], REPO, {"BOARDSTUDIO_OFFLINE_MANIFEST": str(manifest)}))
        destination = output / f"site-{mode}"
        if mode == "subpath":
            destination /= "boardstudio"
        commands.append((f"embed-offline-{mode}", ["node", REPO / "scripts/web/embed-worker-wasm.mjs", output / f"offline-{mode}", manifest, destination / "service-worker.js"], REPO, {}))
    return [(label, list(map(str, argv)), str(cwd), environment) for label, argv, cwd, environment in commands]


def checked_baseline(build_id):
    if not valid_build_id(build_id):
        raise ValueError("baseline build id must be alphanumeric with optional hyphens")
    root = BUILD_ROOT.resolve()
    baseline = (BUILD_ROOT / build_id).resolve()
    if baseline.parent != root or not baseline.is_dir():
        raise ValueError("baseline must be an existing direct child of the builds directory")
    provenance_path = baseline / "provenance.json"
    try:
        provenance_path.resolve().relative_to(baseline)
    except ValueError as error:
        raise ValueError("baseline provenance escapes the build directory") from error
    provenance_hash = sha256_file(provenance_path)
    try:
        provenance = json.loads(provenance_path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"baseline provenance is malformed: {error}") from error
    if not isinstance(provenance, dict):
        raise ValueError("baseline provenance must contain a JSON object")
    if provenance.get("reuse_mode") or provenance.get("status") != "complete":
        raise ValueError("reuse-derived or incomplete baselines cannot be reused")
    if not re.fullmatch(r"[0-9a-f]{40}", str(provenance.get("source_commit", ""))):
        raise ValueError("baseline source commit identity is missing or malformed")
    try:
        subprocess.check_output(["git", "cat-file", "-e", f"{provenance['source_commit']}^{{commit}}"], cwd=REPO, stderr=subprocess.PIPE)
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError("baseline source commit is unavailable in this repository") from error
    if (not isinstance(provenance.get("sources"), dict) or not provenance["sources"]
            or not all(isinstance(path, str) for path in provenance["sources"])):
        raise ValueError("baseline source manifest is missing")
    if any(not re.fullmatch(r"[0-9a-f]{64}", str(value)) for value in provenance["sources"].values()):
        raise ValueError("baseline source manifest contains an invalid SHA-256")
    if not isinstance(provenance.get("commands"), list) or len(provenance["commands"]) != 22:
        raise ValueError("baseline is not a complete 22-command full build")
    if not all(isinstance(row, dict) and isinstance(row.get("log"), str) for row in provenance["commands"]):
        raise ValueError("baseline command receipts are malformed")
    command_labels = [Path(row.get("log", "")).name.removesuffix(".log") for row in provenance["commands"]]
    required_labels = ["rustc-version", "cargo-version", "dx-version", "wasm-pack-version", "node-version", "pnpm-version",
                       "core", "core-worker", "cad-worker", "renderer", "cad", "fixtures", "ergogen-catalogue",
                       "layout-generators", "preview-generator", "ergogen-models", "page-root", "offline-worker-root",
                       "embed-offline-root", "page-subpath", "offline-worker-subpath", "embed-offline-subpath"]
    if command_labels != required_labels or any(row.get("exit") != 0 for row in provenance["commands"]):
        raise ValueError("baseline command lineage is not the successful full-build sequence")
    expected_commands = expected_full_commands(baseline)
    command_log_hashes = {}
    for row, (label, expected_argv, expected_cwd, expected_env) in zip(provenance["commands"], expected_commands):
        log = Path(row.get("log", ""))
        if not log.is_absolute():
            log = baseline / log
        log = log.resolve()
        try:
            log.relative_to(baseline)
        except ValueError as error:
            raise ValueError(f"baseline command log escapes the build directory: {label}") from error
        if not log.is_file():
            raise ValueError(f"baseline command log is missing: {label}")
        command_log_hashes[label] = sha256_file(log)
        argv = row.get("argv")
        if (argv != expected_argv or row.get("cwd") != expected_cwd or row.get("environment", {}) != expected_env):
            raise ValueError(f"baseline full-build command identity is unexpected: {label}")
    for mode, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
        route = provenance.get(mode)
        if not isinstance(route, dict) or route.get("prefix") != prefix or not isinstance(route.get("assets"), dict):
            raise ValueError(f"baseline {mode} route manifest is missing or has the wrong prefix")
        if not isinstance(route.get("site"), str) or not all(
            isinstance(path, str) and isinstance(value, str) for path, value in route["assets"].items()
        ):
            raise ValueError(f"baseline {mode} route manifest is malformed")
        site = Path(route["site"])
        if not site.is_absolute():
            site = baseline / site
        site = site.resolve()
        try:
            site.relative_to(baseline)
        except ValueError as error:
            raise ValueError(f"baseline {mode} site escapes the build directory") from error
        if not site.is_dir():
            raise ValueError(f"baseline {mode} site directory is missing")
        actual = {p.relative_to(site).as_posix() for p in site.rglob("*") if p.is_file()}
        recorded = set(route["assets"])
        if actual != recorded:
            raise ValueError(f"baseline {mode} asset path set differs from provenance")
        required_assets = {
            "assets/m1.css", "assets/core-worker/entry.js", "assets/core-worker/m1_core_worker_bg.wasm",
            "assets/cad-worker/entry.js", "assets/cad-worker/m1_cad_worker_bg.wasm",
            "assets/cad/boardstudio_cadrum_wasm_bg.wasm", "assets/renderer/boardstudio_renderer_wasm_bg.wasm",
            "assets/fixtures/provenance.json", "service-worker.js", "boardstudio_offline_worker.js",
        }
        if not required_assets.issubset(recorded):
            raise ValueError(f"baseline {mode} is missing required provider/staged assets: {sorted(required_assets - recorded)}")
        for relative, expected in route["assets"].items():
            asset = (site / relative).resolve()
            try:
                asset.relative_to(site)
            except ValueError as error:
                raise ValueError(f"baseline {mode} asset escapes its site: {relative}") from error
            if sha256_file(asset) != expected:
                raise ValueError(f"baseline {mode} asset hash differs: {relative}")
    expected_tools = baseline_tools(provenance, baseline)
    observed_tools = current_tools()
    if observed_tools != expected_tools:
        raise ValueError("current tool versions differ from the verified baseline")
    return baseline, provenance, provenance_path, provenance_hash, expected_tools, command_log_hashes


def validate_reuse(build_id, baseline_id):
    if not valid_build_id(build_id):
        raise ValueError("build id must be alphanumeric with optional hyphens")
    if not valid_build_id(baseline_id):
        raise ValueError("baseline build id must be alphanumeric with optional hyphens")
    output = BUILD_ROOT.resolve() / build_id
    if output.exists() or output.is_symlink():
        raise ValueError(f"output already exists: {output}")
    baseline, provenance, provenance_path, provenance_hash, tools, command_log_hashes = checked_baseline(baseline_id)
    current = sources()
    old = provenance["sources"]
    if set(current) != set(old):
        added = sorted(set(current) - set(old))
        removed = sorted(set(old) - set(current))
        raise ValueError(f"source path set differs from baseline; added={added[:8]}, removed={removed[:8]}")
    changed = sorted(path for path in current if current[path] != old[path])
    disallowed = sorted(set(changed) - PAGE_ONLY_ALLOWLIST)
    if disallowed:
        raise ValueError(f"changed inputs are outside the page-only allowlist: {disallowed}")
    for path in PAGE_ONLY_PROOF_PATHS:
        if path not in old or current.get(path) != old[path]:
            raise ValueError(f"page-only dependency proof input changed or is absent: {path}")
    if not set(PAGE_ONLY_ALLOWLIST).issubset(old):
        raise ValueError("baseline does not contain every audited page-only input")
    # Tie allowed prior bytes back to the recorded source commit. The baseline
    # source manifest alone is not enough when the working tree has page edits.
    for path in changed:
        try:
            baseline_bytes = subprocess.check_output(
                ["git", "show", f"{provenance['source_commit']}:{path}"], cwd=REPO, stderr=subprocess.PIPE
            )
            head_bytes = subprocess.check_output(["git", "show", f"HEAD:{path}"], cwd=REPO, stderr=subprocess.PIPE)
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError(f"baseline or current source commit does not contain changed allowlisted input: {path}") from error
        if hashlib.sha256(baseline_bytes).hexdigest() != old[path]:
            raise ValueError(f"baseline source manifest does not match its recorded commit: {path}")
        if hashlib.sha256(head_bytes).hexdigest() != current[path]:
            raise ValueError(f"page-only input has uncommitted changes: {path}")
    return output, baseline, provenance, provenance_path, provenance_hash, current, changed, tools, command_log_hashes


def ignore_rebuilt_assets(directory, names):
    if Path(directory) != WEB / "assets":
        return []
    return [name for name in names if name in {
        "cad", "cad-worker", "core-worker", "renderer", "fixtures", "ergogen-models",
        "layout-generators", "layout-generators.js", "preview-generator",
    }]


def build_reuse(build_id, baseline_id):
    (output, baseline, base_provenance, baseline_provenance_path,
     baseline_provenance_hash, source_before, changed, tools, command_log_hashes) = validate_reuse(build_id, baseline_id)
    # All guards run before this point. Reserve output only after validation.
    output.mkdir(parents=True, exist_ok=False)
    (output / "tmp").mkdir()
    environment = dict(os.environ, TMPDIR=str(output / "tmp"))
    provenance = {
        "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
        "build_id": build_id,
        "reuse_mode": "page-only-provider-reuse",
        "scope": "Fresh page and offline route packages; verified full-build providers reused.",
        "status": "running",
        "base_build": str(baseline),
        "base_source_commit": base_provenance["source_commit"],
        "base_provenance_sha256": baseline_provenance_hash,
        "base_provenance": str(baseline_provenance_path),
        "inherited_full_build_commands": len(base_provenance["commands"]),
        "inherited_full_build_lineage": [
            {
                "label": Path(row["log"]).stem,
                "argv": row["argv"],
                "cwd": row["cwd"],
                "exit": row["exit"],
                "environment": row.get("environment", {}),
                "log": row["log"],
                "log_sha256": command_log_hashes[Path(row["log"]).stem],
            }
            for row in base_provenance["commands"]
        ],
        "changed_allowlisted_inputs": changed,
        "dependency_proof": {
            "statement": "main.rs registers presentation only for wasm32+page; panels is private to presentation; scroll tests compile only for wasm tests; m1.css is linked only by the page and staged as a site asset.",
            "source_hashes": {path: source_before[path] for path in PAGE_ONLY_PROOF_PATHS},
        },
        "sources": source_before,
        "tool_observations": tools,
        "commands": [],
    }
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")

    def run(name, command, cwd=REPO, extra_env=None):
        print(f"{name}: {' '.join(map(str,command))}", flush=True)
        log = output / f"{name}.log"
        started = datetime.now(timezone.utc).isoformat()
        with log.open("w") as stream:
            result = subprocess.run(list(map(str, command)), cwd=cwd, env=dict(environment, **(extra_env or {})), stdout=stream, stderr=subprocess.STDOUT)
        provenance["commands"].append({"argv": list(map(str, command)), "cwd": str(cwd), "exit": result.returncode, "log": str(log), "environment": extra_env or {}, "started": started, "finished": datetime.now(timezone.utc).isoformat()})
        if result.returncode:
            provenance["status"] = f"failed-{name}"
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        if result.returncode:
            print(log.read_text()[-10000:], file=sys.stderr)
            raise SystemExit(result.returncode)

    run("layout-generators", ["node", REPO / "scripts/web/build-layout-generators.mjs", output / "layout-generator-assets"])
    run("preview-generator", ["node", REPO / "scripts/web/build-preview-generator.mjs", output / "preview-generator-assets"])
    for mode, prefix in [("root", "/"), ("subpath", "/boardstudio/")]:
        public = WEB / "target/dx/boardstudio-web/release/web/public"
        # Keep the existing CLI behavior: preserve Dioxus's public output before
        # asking it to emit the fresh page route.
        if public.exists():
            shutil.move(public, output / f"previous-dx-public-{mode}")
        run(f"page-{mode}", ["dx", "build", "--web", "--release", "--base-path", prefix, "--no-default-features", "--features", "page", "--cargo-args=--locked"], WEB)
        destination = output / f"site-{mode}"
        if mode == "subpath":
            destination /= "boardstudio"
        route = base_provenance[mode]
        destination.mkdir(parents=True, exist_ok=False)
        assets = destination / "assets"
        assets.mkdir()
        # Stage only the exact inherited providers from the validated baseline.
        # The candidate site never starts as a copy of the prior site, so stale
        # or unrelated baseline files cannot leak into this route.
        inherited_providers = {
            path: file_hash for path, file_hash in route["assets"].items()
            if path.startswith(REUSED_PROVIDER_PREFIXES)
        }
        for relative in sorted(inherited_providers):
            source = Path(route["site"]) / relative
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        shutil.copytree(public, destination, dirs_exist_ok=True)
        shutil.copytree(WEB / "assets", assets, dirs_exist_ok=True, ignore=ignore_rebuilt_assets)
        shutil.copytree(output / "layout-generator-assets", assets, dirs_exist_ok=True)
        shutil.copytree(output / "preview-generator-assets", assets, dirs_exist_ok=True)
        manifest = output / f"offline-manifest-{mode}.json"
        required = sorted({str(path.relative_to(destination)) for path in destination.rglob("*") if path.is_file()} | {"service-worker.js", "boardstudio_offline_worker.js"})
        manifest.write_text(json.dumps({"version": f"{build_id}-{mode}", "assets": required}, indent=2)+"\n")
        run(f"offline-worker-{mode}", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "boardstudio_offline_worker", "--out-dir", output / f"offline-{mode}", "--release", "--locked", "--no-default-features", "--features", "service-worker"], extra_env={"BOARDSTUDIO_OFFLINE_MANIFEST": str(manifest)})
        run(f"embed-offline-{mode}", ["node", REPO / "scripts/web/embed-worker-wasm.mjs", output / f"offline-{mode}", manifest, destination / "service-worker.js"])
        provenance[mode] = {"prefix": prefix, "site": str(destination), "assets": {
            str(path.relative_to(destination)): digest(path) for path in sorted(destination.rglob("*")) if path.is_file()}}
        candidate_providers = {
            path: file_hash for path, file_hash in provenance[mode]["assets"].items()
            if path.startswith(REUSED_PROVIDER_PREFIXES)
        }
        if candidate_providers != inherited_providers:
            provenance["status"] = "failed-reused-provider-drift"
            provenance["reused_provider_error"] = mode
            (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
            raise SystemExit(f"Reused provider path set or bytes changed during staging: {mode}")
        for path, expected in inherited_providers.items():
            if provenance[mode]["assets"].get(path) != expected:
                provenance["status"] = "failed-reused-provider-drift"
                provenance["reused_provider_error"] = f"{mode}:{path}"
                (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
                raise SystemExit(f"Reused provider asset changed during staging: {mode}:{path}")
        provenance[mode]["reused_provider_assets"] = inherited_providers
        provenance["commands"][-1]["site_manifest_sha256"] = digest(manifest)
        provenance["commands"][-1]["site_asset_count"] = len(provenance[mode]["assets"])
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
    source_after = sources()
    if source_after != source_before:
        provenance["status"] = "failed-source-drift"
        provenance["source_drift"] = sorted(
            path for path in set(source_before) | set(source_after)
            if source_before.get(path) != source_after.get(path)
        )
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        raise SystemExit("Source changed during build: candidate has mixed provenance and must be rebuilt.")
    if sha256_file(baseline_provenance_path) != baseline_provenance_hash:
        provenance["status"] = "failed-baseline-drift"
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        raise SystemExit("Baseline provenance changed during build: candidate must be rebuilt.")
    # Full baseline validation includes every route path, asset hash, provenance
    # identity and tool receipt. Source identity is compared to the pre-build
    # snapshot above because the exact allowlisted page delta is intentional.
    try:
        post_baseline = checked_baseline(baseline_id)
        if post_baseline[3] != baseline_provenance_hash or post_baseline[5] != command_log_hashes:
            raise ValueError("baseline provenance changed during build")
    except ValueError as error:
        provenance["status"] = "failed-baseline-drift"
        provenance["baseline_validation_error"] = str(error)
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        raise SystemExit(f"Baseline changed during build: {error}") from error
    provenance["status"] = "complete"
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
    print(output, flush=True)


def build_full(build_id):
    output = BUILD_ROOT / build_id
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

    for name, command in [("rustc-version", ["rustc", "--version"]), ("cargo-version", ["cargo", "--version"]), ("dx-version", ["dx", "--version"]), ("wasm-pack-version", ["wasm-pack", "--version"]), ("node-version", ["node", "--version"]), ("pnpm-version", ["pnpm", "--version"]),
                          ("core", ["wasm-pack", "build", REPO / "core", "--target", "web", "--release", "--locked"]),
                          ("core-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_core_worker", "--out-dir", output / "core-worker", "--release", "--locked", "--no-default-features", "--features", "core-worker"]),
                          ("cad-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_cad_worker", "--out-dir", output / "cad-worker", "--release", "--locked", "--no-default-features", "--features", "cad-worker"]),
                          ("renderer", ["wasm-pack", "build", REPO / "renderer", "--target", "web", "--out-dir", output / "renderer", "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"]),
                          ("cad", ["pnpm", "--dir", "cad", "run", "build:wasm"]),
                          ("fixtures", ["node", REPO / "scripts/prepare-m1-fixtures.mjs", output / "fixtures"]),
                          ("ergogen-catalogue", ["pnpm", "--dir", "ergogen", "run", "prepare:catalog"]),
                          ("layout-generators", ["node", REPO / "scripts/web/build-layout-generators.mjs", WEB / "assets"]),
                          ("preview-generator", ["node", REPO / "scripts/web/build-preview-generator.mjs", WEB / "assets"]),
                          ("ergogen-models", [sys.executable, REPO / "scripts/stage-ergogen-models.py", "--source-root", REPO / "ergogen/library/vendor", "--destination", WEB / "assets/ergogen-models", "--manifest", output / "ergogen-models-catalog.json"])]:
        run(name, command)
    for mode, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
        public = WEB / "target/dx/boardstudio-web/release/web/public"
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
        for name in ("core-worker", "cad-worker", "renderer", "fixtures"):
            shutil.copytree(output / name, assets / name, dirs_exist_ok=True)
        (assets / "core-worker/entry.js").write_text('import init, { start_core_worker } from "./m1_core_worker.js";\nawait init();\nstart_core_worker();\n')
        (assets / "cad-worker/entry.js").write_text('import init, { start_cad_worker } from "./m1_cad_worker.js";\nawait init();\nstart_cad_worker(new URL("../cad/boardstudio_cadrum_wasm.js", import.meta.url).href);\n')
        manifest = output / f"offline-manifest-{mode}.json"
        required = sorted({str(path.relative_to(destination)) for path in destination.rglob("*") if path.is_file()} | {"service-worker.js", "boardstudio_offline_worker.js"})
        manifest.write_text(json.dumps({"version": f"{build_id}-{mode}", "assets": required}, indent=2)+"\n")
        run(f"offline-worker-{mode}", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "boardstudio_offline_worker", "--out-dir", output / f"offline-{mode}", "--release", "--locked", "--no-default-features", "--features", "service-worker"], extra_env={"BOARDSTUDIO_OFFLINE_MANIFEST": str(manifest)})
        run(f"embed-offline-{mode}", ["node", REPO / "scripts/web/embed-worker-wasm.mjs", output / f"offline-{mode}", manifest, destination / "service-worker.js"])
        provenance[mode] = {"prefix": prefix, "site": str(destination), "assets": {str(path.relative_to(destination)): digest(path) for path in sorted(destination.rglob("*")) if path.is_file()}}
    source_after = sources()
    if source_after != provenance["sources"]:
        provenance["status"] = "failed-source-drift"
        provenance["source_drift"] = sorted(
            path for path in set(provenance["sources"]) | set(source_after)
            if provenance["sources"].get(path) != source_after.get(path)
        )
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        raise SystemExit("Source changed during build: candidate has mixed provenance and must be rebuilt.")
    provenance["status"] = "complete"
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
    print(output, flush=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description="Build a full M1 candidate or a guarded page-only reuse candidate.")
    parser.add_argument("build_id", help="unique alphanumeric/hyphen build identifier")
    parser.add_argument("--reuse-providers-from", metavar="FULL_BUILD_ID", help="reuse verified providers from a complete full build")
    args = parser.parse_args(argv)
    if not valid_build_id(args.build_id):
        parser.error("build id must be alphanumeric with optional hyphens")
    if args.reuse_providers_from is not None:
        if not valid_build_id(args.reuse_providers_from):
            parser.error("baseline build id must be alphanumeric with optional hyphens")
        try:
            build_reuse(args.build_id, args.reuse_providers_from)
        except ValueError as error:
            parser.error(str(error))
        return
    build_full(args.build_id)


if __name__ == "__main__":
    main()
