#!/usr/bin/env python3
"""Run BoardStudio's named check steps in order.

  check.py                run the full check (the steps marked with * below)
  check.py STEP...        run only the named steps
  check.py --list         print every step and its commands

Steps:
  repo*       documentation link check and its tests
  tooling*    tests for the Python build, content and CAD tooling
  build*      production web build (providers and release packaging)
  test*       native Rust tests (Core, application, footprints, renderer, CAD, web)
  typecheck   WASM page compilation (the build step already compiles the page for WASM)
  browser*    mounted headless-browser tests and the real-browser CAD smoke gate
  security    dependency audit of every Rust lockfile (separate CI workflow)
  precommit   typecheck the page
"""

from __future__ import annotations

import argparse
from pathlib import Path
import shlex
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
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
    "scripts/test-stage-ergogen-models.py", "scripts/test-check.py", "scripts/test-catalogue-tools.py",
    "scripts/test-import-kicad-parts.py",
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


STEPS: dict[str, tuple[bool, list[Command]]] = {
    "repo": (True, [[PY, "-B", "scripts/test-check-doc-links.py"], [PY, "scripts/check-doc-links.py"]]),
    "tooling": (True, [[PY, "-B", script] for script in TOOLING_TESTS]),
    "build": (True, [[PY, "scripts/build-web.py"]]),
    "test": (True, [*CARGO_TESTS, [PY, "cad/scripts/test-cadrum.py"]]),
    "typecheck": (False, typecheck()),
    "browser": (True, browser()),
    "security": (False, [[PY, "-B", "scripts/security-audit.test.py"], [PY, "scripts/security-audit.py"]]),
    "precommit": (False, typecheck()),
}
DEFAULT = tuple(name for name, (included, _) in STEPS.items() if included)


def run(names: list[str]) -> int:
    for name in names:
        print(f"== {name}", flush=True)
        for command in STEPS[name][1]:
            print("$ " + shlex.join(command), flush=True)
            if subprocess.run(command, cwd=ROOT).returncode:
                print(f"check: step '{name}' failed", file=sys.stderr)
                return 1
    print("check: all steps passed: " + ", ".join(names))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("steps", nargs="*", metavar="STEP", help="steps to run (default: the full check)")
    parser.add_argument("--list", action="store_true", help="print every step and its commands")
    args = parser.parse_args(argv)
    unknown = [name for name in args.steps if name not in STEPS]
    if unknown:
        parser.error(f"unknown step(s): {', '.join(unknown)}; choose from {', '.join(STEPS)}")
    if args.list:
        for name, (included, commands) in STEPS.items():
            print(f"{name}{'*' if included else ''}")
            for command in commands:
                print("  " + shlex.join(command))
        return 0
    return run(args.steps or list(DEFAULT))


if __name__ == "__main__":
    raise SystemExit(main())
