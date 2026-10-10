#!/usr/bin/env python3
"""Run BoardStudio's named check steps in order.

  check.py                run the full check (the steps marked with * below)
  check.py STEP...        run only the named steps
  check.py quick          the routine loop: only the checks that cover your changed files
  check.py quick --files F...   the same for the named files instead of the git diff
  check.py --list         print every step and its commands

Steps:
  repo*       documentation link check and its tests
  tooling*    tests for the Python build, content and CAD tooling
  build*      checked web build (providers without wasm-opt, unoptimized page)
  release     production web build: providers with wasm-opt and the optimized page
  test*       native Rust tests (Core, application, footprints, renderer, CAD, web)
  lint*       rustfmt and Clippy with warnings denied (workspace lints live in Cargo.toml)
  typecheck   WASM page compilation (the build step already compiles the page for WASM)
  browser*    mounted headless-browser tests and the real-browser CAD smoke gate; crate launches
              run up to BOARDSTUDIO_CHECK_JOBS (default 4) at once, without debug info;
              the independent native test commands also run together
  security    dependency audit of every Rust lockfile (separate CI workflow)
  deny        cargo-deny license, ban and source policy from deny.toml (separate CI workflow)
  precommit   typecheck the page
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))
PY = "python3"
# One workspace invocation compiles Core once for every crate's tests. Footprints runs
# separately: its serde_json `preserve_order` dev-feature must not leak into Core's tests.
CARGO_TESTS: list[list[str]] = [
    ["cargo", "test", "--workspace", "--locked", "--exclude", "boardstudio-footprints"],
    ["cargo", "test", "-p", "boardstudio-footprints", "--locked"],
]
TOOLING_TESTS = (
    "cad/scripts/test-cadrum-build.py", "scripts/test-build-web.py", "scripts/test-serve-web.py",
    "scripts/test-dev-web.py", "scripts/test-run-wasm-tests.py", "scripts/test-check-wasm-tests.py",
    "scripts/test-stage-ergogen-models.py", "scripts/test-check.py", "scripts/test-check-quick.py", "scripts/test-catalogue-tools.py", "scripts/test-gitnexus-refresh.py",
    "scripts/test-import-kicad-parts.py", "scripts/test-gitnexus-stop-hook.py",
)
WASM_PAGE = ["--no-default-features", "--features", "page"]
ISOLATED = ("presentation::case_workspace::",)

Command = list[str]


def typecheck() -> list[Command]:
    return [["cargo", "check", "-p", "boardstudio-web", "--locked", "--target", "wasm32-unknown-unknown",
             *WASM_PAGE, "--bin", "boardstudio-web"]]


def browser() -> list[Command]:
    wasm_pack = ["wasm-pack", "test", "--headless", "--chrome", "web", "--locked", *WASM_PAGE]
    return [
        [PY, "scripts/check-wasm-tests.py"],
        [PY, "cad/scripts/test-cadrum-browser.py"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/host", "--locked", "--features", "page",
         "--lib", "--", "host::storage::"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/runtime", "--locked", "--lib"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/ui-model", "--locked", "--lib"],
        # Panel tests run on their own, as they did in the page suite.
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/ui-shared", "--locked", "--lib", "--", "--skip", "panels::"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/ui-shared", "--locked", "--lib", "--", "panels::"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/keycaps", "--locked", "--lib"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/library", "--locked", "--lib"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/keymap", "--locked", "--lib"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/catalogue", "--locked", "--lib"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/case", "--locked", "--lib"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/parts", "--locked", "--lib"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/pcb", "--locked", "--lib"],
        # The setup guide test runs on its own, as it did in the page suite; web/crates/layout/
        # webdriver.json gives it the desktop viewport it needs.
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/layout", "--locked", "--lib", "--", "--skip", "setup_guide::"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/layout", "--locked", "--lib", "--", "setup_guide::"],
        [*wasm_pack, "--bin", "boardstudio-web", "--", "--list"],
        [PY, "scripts/run-wasm-tests.py", "--all", "--depth", "1", *[f"--isolate={name}" for name in ISOLATED],
         "--result-json", "web/target/test-results/browser.json"],
    ]


# `--no-deps` keeps Clippy off the vendored cgmath patch; its lints are not ours to fix.
LINT: list[list[str]] = [
    ["cargo", "fmt", "--all", "--check"],
    ["cargo", "clippy", "--workspace", "--locked", "--all-targets", "--no-deps", "--", "-D", "warnings"],
]

STEPS: dict[str, tuple[bool, list[Command]]] = {
    "repo": (True, [[PY, "-B", "scripts/test-check-doc-links.py"], [PY, "scripts/check-doc-links.py"]]),
    "tooling": (True, [[PY, "-B", script] for script in TOOLING_TESTS]),
    "lint": (True, LINT),
    # The routine build skips wasm-opt and the optimized Dioxus build; `release` keeps both.
    "build": (True, [[PY, "scripts/build-web.py", "--fast"]]),
    "release": (False, [[PY, "scripts/build-web.py"]]),
    "test": (True, [*CARGO_TESTS, [PY, "cad/scripts/test-cadrum.py"]]),
    "typecheck": (False, typecheck()),
    "browser": (True, browser()),
    "security": (False, [[PY, "-B", "scripts/security-audit.test.py"], [PY, "scripts/security-audit.py"]]),
    "deny": (False, [["cargo", "deny", "--locked", "check", "licenses", "bans", "sources"]]),
    "precommit": (False, typecheck()),
}
DEFAULT = tuple(name for name, (included, _) in STEPS.items() if included)


# Launches that take longest start first, so the short ones fill the gaps behind them.
HEAVY_CRATES = ("layout", "case", "pcb", "parts")


def is_crate_launch(command: Command) -> bool:
    """One `wasm-pack test` launch of a presentation crate: independent of every other launch."""
    return command[:2] == ["wasm-pack", "test"] and any(part.startswith("web/crates/") for part in command)


def check_jobs() -> int:
    try:
        return max(1, int(os.environ.get("BOARDSTUDIO_CHECK_JOBS", "4")))
    except ValueError:
        return 4


def launch_order(command: Command) -> int:
    return 0 if any(f"web/crates/{crate}" in part for crate in HEAVY_CRATES for part in command) else 1


def run_serial(command: Command, environment: dict[str, str]) -> tuple[float, int]:
    print("$ " + shlex.join(command), flush=True)
    started = time.monotonic()
    code = subprocess.run(command, cwd=ROOT, env=environment).returncode
    elapsed = time.monotonic() - started
    print(f"  ({elapsed:.1f}s)", flush=True)
    return elapsed, code


def run_parallel(batch: list[Command], environment: dict[str, str]) -> tuple[float, list[Command]]:
    """Runs independent launches together, printing each one's output whole and in order."""
    def launch(command: Command) -> tuple[int, str, float]:
        started = time.monotonic()
        done = subprocess.run(command, cwd=ROOT, env=environment, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True)
        return done.returncode, done.stdout, time.monotonic() - started

    ordered = sorted(batch, key=launch_order)
    started = time.monotonic()
    failed: list[Command] = []
    with ThreadPoolExecutor(max_workers=check_jobs()) as pool:
        futures = [(command, pool.submit(launch, command)) for command in ordered]
        for command, future in futures:
            code, output, elapsed = future.result()
            print("$ " + shlex.join(command), flush=True)
            print(output, end="" if output.endswith("\n") or not output else "\n", flush=True)
            print(f"  ({elapsed:.1f}s)", flush=True)
            if code:
                failed.append(command)
    return time.monotonic() - started, failed


def run_commands(name: str, commands: list[Command], environment: dict[str, str]) -> tuple[float, bool]:
    """Runs a step's commands; consecutive crate launches in `browser`, and the independent
    workspaces of `test`, run together."""
    if name == "test" and check_jobs() > 1 and len(commands) > 1:
        elapsed, failed = run_parallel(commands, environment)
        for command in failed:
            print("check: command failed: " + shlex.join(command), file=sys.stderr)
        return elapsed, not failed
    total = 0.0
    index = 0
    while index < len(commands):
        command = commands[index]
        if name == "browser" and check_jobs() > 1 and is_crate_launch(command):
            end = index
            while end < len(commands) and is_crate_launch(commands[end]):
                end += 1
            elapsed, failed = run_parallel(commands[index:end], environment)
            total += elapsed
            if failed:
                for command in failed:
                    print("check: launch failed: " + shlex.join(command), file=sys.stderr)
                return total, False
            index = end
            continue
        elapsed, code = run_serial(command, environment)
        total += elapsed
        if code:
            return total, False
        index += 1
    return total, True


def run(names: list[str]) -> int:
    return run_plan([(name, STEPS[name][1]) for name in names])


def run_plan(plan: list[tuple[str, list[Command]]]) -> int:
    names = [name for name, _ in plan]
    totals: dict[str, float] = {}
    for name, commands in plan:
        print(f"== {name}", flush=True)
        environment = os.environ.copy()
        if name == "test":
            # An editor's AppImage directory redirects system KiCad's library lookup.
            environment.pop("APPDIR", None)
        if name == "browser":
            # Match the page runner's bounded budget for direct wasm-pack crate batches.
            environment.setdefault("WASM_BINDGEN_TEST_TIMEOUT", "120")
            # Test wasm without DWARF is about a seventh of the size, which is most of a launch.
            environment.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
        elapsed, passed = run_commands(name, commands, environment)
        totals[name] = totals.get(name, 0.0) + elapsed
        if not passed:
            print(f"check: step '{name}' failed", file=sys.stderr)
            return 1
    print("check: step times: " + ", ".join(f"{name} {seconds:.1f}s" for name, seconds in totals.items())
          + f", total {sum(totals.values()):.1f}s")
    print("check: all steps passed: " + ", ".join(dict.fromkeys(names)))
    return 0


def quick_plan(selection: "check_quick.Selection", packages: "dict[str, check_quick.Package]") -> list[tuple[str, list[Command]]]:
    """The checks that cover the selected change: never `build`, only the affected tests."""
    plan: list[tuple[str, list[Command]]] = [("repo", STEPS["repo"][1])]
    if selection.tooling:
        plan.append(("tooling", STEPS["tooling"][1]))
    if selection.rust:
        plan += [("lint", LINT), ("typecheck", typecheck())]
        if selection.full_rust:
            native = list(CARGO_TESTS)
        else:
            names = sorted(selection.packages - {"boardstudio-footprints"})
            native = [["cargo", "test", "--locked", *[part for name in names for part in ("-p", name)]]] if names else []
            if "boardstudio-footprints" in selection.packages:
                native.append(CARGO_TESTS[1])
        plan.append(("test", native))
        crates = {Path(package.directory).name for name, package in packages.items()
                  if name in selection.packages and package.directory.startswith("web/crates/")}
        if selection.full_rust:
            commands = browser()
        else:
            commands = [command for command in browser() if is_crate_launch(command)
                        and any(part == f"web/crates/{crate}" for crate in crates for part in command)]
            if selection.page_files:
                commands.append([PY, "scripts/run-wasm-tests.py", "--files", *selection.page_files,
                                 "--result-json", "web/target/test-results/browser-quick.json"])
        if commands:
            plan.append(("browser", commands))
    if selection.cad:
        plan.append(("test", [[PY, "cad/scripts/test-cadrum.py"]]))
    return plan


def quick(files: list[str] | None) -> int:
    import check_quick

    chosen = files if files else check_quick.changed_files(ROOT)
    packages = check_quick.workspace_packages(ROOT)
    selection = check_quick.select(chosen, packages)
    print(f"quick: {len(chosen)} changed file(s)")
    for note in selection.notes:
        print(f"quick: {note}")
    for path in selection.unplaced:
        print(f"quick: {path}: not mapped to a crate, so every Rust check applies")
    plan = quick_plan(selection, packages)
    print("quick: running " + ", ".join(dict.fromkeys(name for name, _ in plan)))
    return run_plan(plan)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("steps", nargs="*", metavar="STEP", help="steps to run (default: the full check)")
    parser.add_argument("--list", action="store_true", help="print every step and its commands")
    parser.add_argument("--files", nargs="+", metavar="FILE",
                        help="with `quick`: the changed files, instead of the git diff against main")
    args = parser.parse_args(argv)
    if args.steps == ["quick"]:
        return quick(args.files)
    if args.files:
        parser.error("--files applies to `quick` only")
    unknown = [name for name in args.steps if name not in STEPS]
    if unknown:
        parser.error(f"unknown step(s): {', '.join(unknown)}; choose from {', '.join(STEPS)}")
    if args.list:
        for name, (included, commands) in STEPS.items():
            print(f"{name}{'*' if included else ''}")
            for command in commands:
                print("  " + shlex.join(command))
        print("quick  (changed files only; see `check.py quick`)")
        return 0
    return run(args.steps or list(DEFAULT))


if __name__ == "__main__":
    raise SystemExit(main())
