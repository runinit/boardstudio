#!/usr/bin/env python3
"""Run BoardStudio's named check steps in order.

  check.py                run the full check (the steps marked with * below)
  check.py STEP...        run only the named steps
  check.py browser --only CRATE...   run only the named presentation crate browser launch(es)
                          (partial subset; skips full page build and drag smoke)
  check.py browser --keep-going      continue independent browser launches after a failure
  check.py quick          the routine loop: only the checks that cover your changed files
                          run the full check once after focused checks and fixes
  check.py quick --files F...   the same for the named files instead of the git diff
  check.py test --update-native-baseline   after the tests pass, record the executed native test
                          names as the new baseline (use when tests were removed or renamed on purpose)
  check.py --list         print every step and its commands

For a shared local build lock, open and flock ~/.cache/tmp/boardstudio-build.lock
on a shell file descriptor, then export BOARDSTUDIO_BUILD_LOCK_FD with that descriptor
number before starting check.py. The scheduler validates and holds it for the whole run.

Steps:
  repo*       documentation link check and its tests
  tooling*    tests for the Python build, content and CAD tooling
  build*      checked web build (providers without wasm-opt, unoptimized page)
  release     production web build: providers with wasm-opt and the optimized page
  test*       native Rust tests (Core, application, footprints, renderer, CAD, web); fails when a
              test named in scripts/check-native-tests-baseline.json no longer executes
              (`check.py test --update-native-baseline` records a deliberate change)
  lint*       rustfmt and Clippy with warnings denied (workspace lints live in Cargo.toml)
  typecheck   WASM page compilation (the build step already compiles the page for WASM)
  browser*    mounted headless-browser tests and the real-browser CAD smoke gate; crate launches
              share one WASM test build without debug info and run with the page suite up to
              BOARDSTUDIO_CHECK_JOBS (up to 6 by default, bounded by CPU count) at once,
              beside two lanes of packaged checks (the drag smoke; the CAD browser checks);
              the independent native test commands also run together
  security    dependency audit of every Rust lockfile (separate CI workflow)
  deny        cargo-deny license, ban and source policy from deny.toml (separate CI workflow)
  precommit   typecheck the page
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from contextlib import contextmanager, ExitStack
import argparse
import codecs
import io
import importlib.util
import json
import os
import shutil
from pathlib import Path
import re
import shlex
import signal
import threading
import subprocess
import sys
import tempfile
import time
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import browser_tools  # noqa: E402  (needs the scripts directory on sys.path)
import cargo_json  # noqa: E402
import check_native_tests  # noqa: E402  (needs the scripts directory on sys.path)
from tailwind_tools import IN_OVERLAP_ENV, check_jobs, tailwind_cli  # noqa: E402
import verification_receipt  # noqa: E402  (needs the scripts directory on sys.path)
import wasm_bindgen_tools  # noqa: E402


PY = "python3"
OVERLAP_MIN_CPUS = 8
OVERLAP_STEPS = frozenset({"build", "lint", "test"})
# One workspace invocation compiles Core once for every crate's tests, including footprints.
CARGO_TESTS: list[list[str]] = [
    ["cargo", "test", "--workspace", "--locked"],
]
# Longest first, so the parallel batch ends with short scripts; the serial scripts run after it.
TOOLING_TESTS = (
    "scripts/test-verification-receipt.py", "cad/scripts/test-cad-build.py", "scripts/test-ci-workflow.py",
    "scripts/test-check-quick.py", "scripts/test-check.py", "scripts/test-browser-tools.py",
    "tools/browser-harness/test-response-probe-cleanup.py", "cad/scripts/test-measure-cad.py",
    "scripts/test-mutation-check.py", "scripts/test-graphite-preflight.py", "scripts/test-run-wasm-tests.py",
    "scripts/test-gitnexus-stop-hook.py", "scripts/test-stage-ergogen-models.py", "scripts/test-dev-web.py",
    "scripts/test-build-web.py", "scripts/test-build-time.py", "scripts/test-build-patched-wasm-bindgen.py",
    "scripts/test-gitnexus-refresh.py", "cad/scripts/test-face-completeness-gate.py",
    "scripts/test-bump-chrome-for-testing.py", "scripts/test-chrome-for-testing.py",
    "scripts/test-check-native-tests.py", "scripts/test-verify-no-node.py", "scripts/test-drag-receipt-bytes.py",
    "scripts/test-check-wasm-tests.py", "scripts/test-catalogue-tools.py",
    "scripts/test-serve-web.py", "scripts/test-parallel-interrupt.py",
)
SERIAL_TOOLING_SCRIPTS = {
    "scripts/test-serve-web.py",  # Releases its selected port before starting the preview process.
    "scripts/test-parallel-interrupt.py",  # Owns detached signal-test groups outside the outer batch's cleanup.
}
KICAD_PARTS_TEST = [PY, "scripts/test-import-kicad-parts.py"]
WASM_PAGE = ["--no-default-features", "--features", "page"]
# Every wasm test build shares the checked page build's profile, and so its units; see Cargo.toml.
WASM_CRATE_BUILD = ["cargo", "test", "--no-run", "--lib", "--target", "wasm32-unknown-unknown",
                    "--profile", "wasm-check", "--locked", "--message-format=json"]
DRAG_SMOKE = [PY, "scripts/run-drag-browser.py"]
CAD_BROWSER = [PY, "cad/scripts/test-cad-browser.py"]
# Each of these first runs a Cargo build of the browser harness (a no-op after `build`). In a parallel
# browser batch they run one after another behind the Cargo-free CAD browser check, which outlasts
# the drag smoke's own harness build, so two harness builds do not overlap.
HARNESS_CHECKS = [
    [PY, "cad/scripts/face-completeness-gate.py"],
    [PY, "cad/scripts/test-cad-capture-controls.py"],
    [PY, "cad/scripts/test-cad-provider.py"],
    [PY, "cad/scripts/test-cold-responses.py"],
    [PY, "-B", "scripts/ergogen-parity/test_generate.py", "--source"],
    [PY, "scripts/model-lookup-pilot.py", "--gate"],
]
# Two depth-one pages. Each further page costs ~15s of loading beside the crate launches, more than
# splitting the larger group with `--max-group-tests` saves (measured 2026-11-19).
PAGE_SUITE = [PY, "scripts/run-wasm-tests.py", "--all", "--depth", "1",
              "--result-json", "web/target/test-results/browser.json"]
PAGE_TEST_BUILD = [PY, "scripts/run-wasm-tests.py", "--build-only"]
HARNESS_BUILD = [PY, "scripts/browser_tools.py"]
PAGE_PACKAGE = "boardstudio-web"
# Must equal run-wasm-tests.PAGE_ARTIFACT; check.py cannot import that module at load time
# because it imports check back, so the equality is pinned by scripts/test-check.py instead.
PAGE_TEST_ARTIFACT_ENV = "BOARDSTUDIO_PAGE_TEST_ARTIFACT"
# The page test artifact from this process's combined prebuild, reused by its browser step.
_PAGE_TEST_ARTIFACT: Path | None = None
BUILD_LOCK_SCRIPTS = (
    "scripts/run-drag-browser.py", "cad/scripts/test-cad-capture-controls.py", "cad/scripts/test-cold-responses.py",
    "cad/scripts/face-completeness-gate.py", "cad/scripts/test-cad-provider.py", "scripts/build-web.py",
    "scripts/run-wasm-tests.py", "cad/scripts/test-cad.py", KICAD_PARTS_TEST[1], "scripts/browser_tools.py",
    "scripts/model-lookup-pilot.py", "scripts/ergogen-parity/test_generate.py", "scripts/check.py",
)
def test_css_command() -> list[str]:
    """The standalone Tailwind CLI's stylesheet build, or else the page build that also writes it."""
    if compiler := tailwind_cli():
        return [str(compiler), "-i", "tailwind.css", "-o", "assets/tailwind.css"]
    return ["dx", "build", "--web", "--profile", "wasm-check", "--debug-symbols", "false",
            "--wasm-js-cfg", "false", "--no-default-features", "--features", "page",
            "--cargo-args=--locked"]


# These browser-test targets include the generated Tailwind stylesheet at compile time.
CSS_TEST_CRATES = {"ui-shared", "keycaps", "case", "layout"}

Command = list[str]
# The active opt-in receipt collector for `--receipt`; set by `main` and read by `run_plan`.
_RECEIPT: verification_receipt.Collector | None = None


def typecheck() -> list[Command]:
    return [["cargo", "check", "-p", "boardstudio-web", "--locked", "--target", "wasm32-unknown-unknown",
             *WASM_PAGE, "--bin", "boardstudio-web"]]


def browser() -> list[Command]:
    return [
        DRAG_SMOKE,
        CAD_BROWSER,
        *HARNESS_CHECKS,
        *with_crate_build([
            crate_launch("host"),
            crate_launch("worker-host"),
            crate_launch("runtime"),
            crate_launch("ui-model"),
            crate_launch("ui-shared"),
            crate_launch("keycaps"),
            crate_launch("library"),
            crate_launch("keymap"),
            crate_launch("catalogue"),
            crate_launch("case"),
            crate_launch("parts"),
            crate_launch("pcb"),
            crate_launch("layout"),
        ]),
        PAGE_SUITE,
    ]


def crate_launch(crate: str, *filters: str) -> Command:
    # The plan uses the crate directory as a stable identity. Cargo supplies the binary at runtime.
    return [wasm_bindgen_tools.TEST_RUNNER, f"web/crates/{crate}", *filters]


def launch_crate(command: Command) -> str | None:
    """The presentation crate targeted by a browser launch."""
    key = verification_receipt.crate_launch_key(command)
    return key[0] if key else None


def is_crate_build(command: Command) -> bool:
    return command[:len(WASM_CRATE_BUILD)] == WASM_CRATE_BUILD


def with_crate_build(launches: list[Command]) -> list[Command]:
    crates = tuple(dict.fromkeys(crate for command in launches if (crate := launch_crate(command))))
    if not crates:
        return launches
    packages = [arg for crate in crates for arg in ("-p", f"boardstudio-web-{crate}")]
    features = [
        *(["boardstudio-web-host/page"] if "host" in crates else []),
        *(["boardstudio-web-worker-host/cad"] if "worker-host" in crates else []),
    ]
    flags = ["--features", ",".join(features)] if features else []
    return [[*WASM_CRATE_BUILD, *packages, *flags], *launches]


WEB_WASM_PACKAGES: tuple[str, ...] = (
    "boardstudio-web",
    *dict.fromkeys(f"boardstudio-web-{crate}" for cmd in browser() if (crate := launch_crate(cmd))),
)

BROWSER_CRATE_BUILD = next(command for command in browser() if is_crate_build(command))
BROWSER_PREBUILDS = [BROWSER_CRATE_BUILD, PAGE_TEST_BUILD, HARNESS_BUILD]


def plain_page_tools_available() -> bool:
    """Whether `build-web.py --fast` uses the splittable plain-Cargo page (not the `dx` fallback)."""
    return importlib.import_module("build-web").page_tools(quiet=True) is not None


def _cargo_option_values(command: Command, flag: str) -> list[str]:
    """Values of `--flag value` and `--flag=value` occurrences, in command order."""
    values = [command[index + 1] for index, token in enumerate(command[:-1]) if token == flag]
    return values + [token.split("=", 1)[1] for token in command if token.startswith(flag + "=")]


def combined_wasm_prebuild(crate_build: Command) -> Command | None:
    """One `cargo test --no-run` for the full crate prebuild plus the page test build, or None.

    The two prebuilds share profile, target and (every presentation crate's dev-dependencies
    already unify `test-support`) features, so one invocation compiles the page's library units
    beside the crate tests instead of after them. Returns None when either side's flags drifted
    from the combinable shape, so the caller keeps the serial scheduling.
    """
    if crate_build != BROWSER_CRATE_BUILD:
        return None
    page = importlib.import_module("run-wasm-tests").PAGE_BUILD
    if page[:3] != ["cargo", "test", "--no-run"] or "--locked" not in page:
        return None
    shared = ("--target", "--profile", "--message-format")
    expected = {flag: _cargo_option_values(WASM_CRATE_BUILD, flag) for flag in shared}
    actual = {flag: _cargo_option_values(page, flag) for flag in shared}
    if any(len(values) != 1 for values in (*expected.values(), *actual.values())) or expected != actual:
        return None
    if any(token in ("--lib", "--bins", "--examples", "--tests", "--benches", "--all-targets",
                     "--doc", "--test", "--example", "--bench") for token in page):
        return None
    packages = _cargo_option_values(page, "-p")
    bins = _cargo_option_values(page, "--bin")
    features = _cargo_option_values(page, "--features")
    if (packages != [PAGE_PACKAGE] or bins != [PAGE_PACKAGE] or len(features) != 1 or not features[0]
            or "--no-default-features" not in page):
        return None
    rest = crate_build[len(WASM_CRATE_BUILD):]
    crates: list[str] = []
    index = 0
    while index < len(rest) and rest[index] == "-p":
        if index + 1 >= len(rest):
            return None
        crates.append(rest[index + 1])
        index += 2
    flags = rest[index:]
    if flags == []:
        crate_features: list[str] = []
    elif len(flags) == 2 and flags[0] == "--features" and flags[1]:
        crate_features = flags[1].split(",")
    else:
        return None
    merged = list(dict.fromkeys([*crate_features,
                                 *(feature if "/" in feature else f"{PAGE_PACKAGE}/{feature}"
                                   for feature in features[0].split(","))]))
    return [*WASM_CRATE_BUILD, *(part for crate in crates for part in ("-p", crate)),
            "-p", PAGE_PACKAGE, "--bin", PAGE_PACKAGE, "--no-default-features",
            "--features", ",".join(merged)]


def page_test_artifact(output: str, cwd: Path) -> Path:
    """The existing boardstudio-web test wasm in combined-prebuild Cargo JSON output."""
    found = []
    for line in output.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(message, dict):
            continue
        if ((executable := cargo_json.wasm_executable(message, kind="bin", name=PAGE_PACKAGE, test=True))
                and Path(message.get("manifest_path", "")) == cwd / "web/Cargo.toml"):
            found.append(cwd / executable)
    if len(found) != 1 or not found[0].is_file():
        raise ValueError(f"expected one existing {PAGE_PACKAGE} test wasm artifact from Cargo, got {found}")
    return found[0]

# `--no-deps` keeps Clippy off the vendored cgmath fork; its lints are audited separately.
# The browser harness lives outside the workspace with its own lockfile, so it
# gets its own manifest-scoped format and lint commands.
HARNESS_MANIFEST = "tools/browser-harness/Cargo.toml"
LINT: list[list[str]] = [
    ["cargo", "fmt", "--all", "--check"],
    ["cargo", "clippy", "--profile", "lint", "--workspace", "--locked", "--all-targets", "--no-deps", "--", "-D", "warnings"],
    # Native Clippy compiles non-WASM stubs for browser-gated modules and tests, so all
    # web workspace crates are also linted under an explicit WASM target.
    ["cargo", "clippy", "--profile", "lint", *[arg for pkg in WEB_WASM_PACKAGES for arg in ("--package", pkg)],
     "--target", "wasm32-unknown-unknown", "--all-targets", "--locked", "--no-deps", "--", "-D", "warnings"],
    # The browser harness lives outside the workspace with its own lockfile, so
    # it gets its own manifest-scoped format and lint commands.
    ["cargo", "fmt", "--manifest-path", HARNESS_MANIFEST, "--", "--check"],
    ["cargo", "clippy", "--manifest-path", HARNESS_MANIFEST, "--all-targets", "--locked", "--no-deps", "--", "-D", "warnings"],
]

STEPS: dict[str, tuple[bool, list[Command]]] = {
    "repo": (True, [
        [PY, "-B", "scripts/test-check-doc-links.py"],
        [PY, "scripts/check-doc-links.py"],
        [PY, "-B", "scripts/test-check-core-tests.py"],
        [PY, "scripts/check-core-tests.py"],
        [PY, "-B", "scripts/test-check-adr-index.py"],
        [PY, "scripts/check-adr-index.py"],
        [PY, "-B", "scripts/test-check-old-css.py"],
        [PY, "scripts/check-old-css.py"],
        # A static check of the page's browser-test modules; it needs no browser or built site.
        [PY, "scripts/check-wasm-tests.py"],
    ]),
    "tooling": (True, [[PY, "-B", "scripts/ergogen-parity/test_generate.py", "--hermetic"],
                       [PY, "-B", "cad/scripts/test-face-completeness-web.py", "--hermetic"],
                       [PY, "-B", "cad/scripts/test-provider-pages.py", "--hermetic"],
                       [PY, "-B", "cad/scripts/test-cad-provider-gate.py"], *[
        [PY, "-B", script]
        for script in TOOLING_TESTS]]),
    # The routine build skips wasm-opt and the optimized Dioxus build; `release` keeps both.
    "build": (True, [[PY, "scripts/build-web.py", "--fast"], *BROWSER_PREBUILDS]),
    "lint": (True, LINT),
    "release": (False, [[PY, "scripts/build-web.py"]]),
    "test": (True, [*CARGO_TESTS, [PY, "cad/scripts/test-cad.py"], KICAD_PARTS_TEST]),
    "typecheck": (False, typecheck()),
    "browser": (True, browser()),
    "browser-packaged": (False, [cmd for cmd in browser() if not (is_crate_build(cmd) or launch_crate(cmd))]),
    "browser-wasm": (False, [cmd for cmd in browser() if is_crate_build(cmd) or launch_crate(cmd)]),
    "drag-smoke": (False, [DRAG_SMOKE]),
    "security": (False, [[PY, "-B", "scripts/security-audit.test.py"], [PY, "scripts/security-audit.py"]]),
    # Fails while any .board-*, .m1-* selector or --wb-* variable remains (the final merge gate).
    "old-css": (False, [[PY, "scripts/check-old-css.py", "--strict"]]),
    "deny": (False, [["cargo", "deny", "--locked", "check", "licenses", "bans", "sources"]]),
    "precommit": (False, typecheck()),
}
DEFAULT = tuple(name for name, (included, _) in STEPS.items() if included)


# Launches that take longest start first, so the short ones fill the gaps behind them.
HEAVY_CRATES = ("layout", "case", "pcb", "parts")
HARNESS_TIMEOUT_LINE = "Failed to detect test as having been run. It might have timed out."
PANIC_HOOK_CASCADE_TEXT = "cannot modify the panic hook from a panicking thread"
TEST_STATUS_RE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|FAIL)\b", re.M)
INVOKED_TEST_RE = re.compile(r"^\s*Invoking test: (\S+)")
FAILURE_HEADER_RE = re.compile(r"^---- (\S+) output ----$")
PANICKED_AT_RE = re.compile(r"^\s*panicked at ([^:\n]+:\d+:\d+):\s*(.*)$")
RUNTIME_ERROR_RE = re.compile(r"^\s*RuntimeError:\s*(.+)$")
WASM_FRAME_RE = re.compile(r"^\s*at [^\s(]+\.wasm\.([^\s(]+)")


def is_crate_launch(command: Command) -> bool:
    """One presentation crate's test launch, independent after the shared build."""
    return launch_crate(command) is not None


def browser_crates(commands: Sequence[Command] | None = None) -> tuple[str, ...]:
    """Presentation crate names derived in order from the `browser` command plan."""
    source = STEPS["browser"][1] if commands is None else commands
    return tuple(dict.fromkeys(crate for command in source if (crate := launch_crate(command))))


def select_browser_crate_launches(crates: Iterable[str],
                                  commands: Sequence[Command] | None = None) -> list[Command]:
    chosen = set(crates)
    source = STEPS["browser"][1] if commands is None else commands
    return [command for command in source if launch_crate(command) in chosen]


def parse_only_crates(raw: Sequence[str] | None,
                      available: Sequence[str] | None = None) -> list[str]:
    known = tuple(browser_crates() if available is None else available)
    chosen: list[str] = []
    for item in raw or ():
        for part in item.split(","):
            name = part.strip().removeprefix("web/crates/")
            if name and name not in chosen:
                chosen.append(name)
    if not chosen:
        raise ValueError("--only requires at least one browser crate name")
    unknown = [name for name in chosen if name not in known]
    if unknown:
        raise ValueError(f"unknown browser crate(s): {', '.join(unknown)}; choose from {', '.join(known)}")
    return chosen


def is_browser_step(name: str) -> bool:
    return name in ("browser", "browser-packaged", "browser-wasm") or name.startswith("browser (")


def launch_order(command: Command) -> int:
    return 0 if launch_crate(command) in HEAVY_CRATES else 1


def is_harness_clippy(command: Command) -> bool:
    return command[:2] == ["cargo", "clippy"] and HARNESS_MANIFEST in command


def is_cargo_test(command: Command) -> bool:
    return command[:2] == ["cargo", "test"]


def is_wasm_pack_test(command: Command) -> bool:
    return command[:2] == ["wasm-pack", "test"]


def is_page_suite(command: Command) -> bool:
    return "scripts/run-wasm-tests.py" in command and "--all" in command


def page_suite_jobs(workers: int) -> int:
    """Browser pages for the page suite within a parallel batch of `workers` lanes."""
    return min(2, max(1, workers - 1))


def crate_test_artifacts(output: str, cwd: Path, crates: Iterable[str], *, stream=None) -> dict[str, Path]:
    out = stream if stream is not None else sys.stdout
    flush = stream is None
    manifests = {cwd / "web/crates" / crate / "Cargo.toml": crate for crate in crates}
    artifacts: dict[str, Path] = {}
    for message in cargo_json.messages(output, out, flush=flush):
        if not (executable := cargo_json.wasm_executable(message, kind="lib", test=True)):
            continue
        if crate := manifests.get(Path(message["manifest_path"])):
            binary = cwd / executable
            if crate in artifacts or not binary.is_file():
                raise ValueError(f"invalid or duplicate test artifact for {crate}: {binary}")
            artifacts[crate] = binary
    if missing := set(manifests.values()) - artifacts.keys():
        raise ValueError(f"missing test wasm artifact(s) for {', '.join(sorted(missing))}")
    return artifacts


def output_text(value: object) -> str:
    return value if isinstance(value, str) else ""


def _console_trace_lines(lines: list[str]) -> list[str]:
    for index, line in enumerate(lines):
        if line.strip() == "console output:":
            end = next((pos for pos in range(index + 1, len(lines))
                        if lines[pos].startswith(("driver status:", "Error: some tests failed"))),
                       len(lines))
            return lines[index + 1:end]
    return lines


def summarize_wasm_failure(output: str) -> dict[str, object] | None:
    """Classify the first chronological failure in `wasm-bindgen-test` output."""
    if not isinstance(output, str) or not output.strip():
        return None
    timed_out = HARNESS_TIMEOUT_LINE in output
    terminal = {name: status for name, status in TEST_STATUS_RE.findall(output)}
    failed_names = [name for name, status in TEST_STATUS_RE.findall(output) if status in ("FAIL", "FAILED")]
    all_lines = output.splitlines()

    def scan(scan_lines: list[str]) -> tuple[dict[str, object] | None, list[str], list[str]]:
        current_test: str | None = None
        current_panicked = False
        current_cascaded = False
        invoked: list[str] = []
        first: dict[str, object] | None = None
        cascaded: list[str] = []
        index = 0
        while index < len(scan_lines):
            line = scan_lines[index]
            if match := INVOKED_TEST_RE.match(line):
                current_test = match.group(1)
                current_panicked = False
                current_cascaded = False
                invoked.append(current_test)
                index += 1
                continue
            if match := FAILURE_HEADER_RE.match(line.strip()):
                current_test = match.group(1)
                current_panicked = False
                current_cascaded = False
                index += 1
                continue
            if match := PANICKED_AT_RE.match(line):
                location = match.group(1).strip()
                details = [inline] if (inline := match.group(2).strip()) else []
                look = index + 1
                while look < len(scan_lines) and len(details) < 4:
                    candidate = scan_lines[look].strip()
                    if not candidate or candidate.startswith((
                        "Stack:", "Invoking test:", "wasm-bindgen:", "RuntimeError:",
                        "---- ", "test ", "driver status:", "Error:",
                    )):
                        break
                    details.append(candidate)
                    look += 1
                if (len(details) >= 3 and details[0].startswith("assertion ")
                        and details[1].startswith("left:") and details[2].startswith("right:")):
                    detail = f"{details[0]} ({details[1]}, {details[2]})"
                else:
                    detail = "; ".join(details) if details else "panicked"
                if PANIC_HOOK_CASCADE_TEXT in detail and first is not None:
                    current_cascaded = True
                    current_panicked = True
                    key = current_test or f"<panic-{len(cascaded) + 1}>"
                    if key not in cascaded:
                        cascaded.append(key)
                elif current_cascaded:
                    current_panicked = True
                else:
                    current_panicked = True
                    if first is None:
                        first = {
                            "kind": "rust_panic",
                            "test": current_test,
                            "location": location,
                            "detail": detail,
                        }
                index = look
                continue
            if match := RUNTIME_ERROR_RE.match(line):
                trap_reason = match.group(1).strip()
                wasm_frame = next(
                    (frame_match.group(1).strip()
                     for frame_line in scan_lines[index + 1:index + 7]
                     if (frame_match := WASM_FRAME_RE.match(frame_line))),
                    None,
                )
                post_panic_abort = (current_panicked or current_cascaded) and trap_reason == "unreachable"
                if not post_panic_abort and first is None:
                    first = {
                        "kind": "wasm_trap",
                        "test": current_test,
                        "location": wasm_frame,
                        "detail": f"RuntimeError: {trap_reason}",
                    }
                index += 1
                continue
            index += 1
        return first, invoked, cascaded

    trace_lines = _console_trace_lines(all_lines)
    first_failure, invoked_tests, cascade_tests = scan(trace_lines)
    if first_failure is None and not invoked_tests and trace_lines is not all_lines:
        first_failure, invoked_tests, cascade_tests = scan(all_lines)

    if first_failure is not None and first_failure["test"] is None and failed_names:
        first_failure["test"] = failed_names[0]

    if first_failure is None:
        incomplete = [name for name in invoked_tests if name not in terminal]
        if timed_out or incomplete:
            target = incomplete[0] if incomplete else (invoked_tests[-1] if invoked_tests else None)
            detail = HARNESS_TIMEOUT_LINE if timed_out else "invoked test did not report a terminal result"
            first_failure = {
                "kind": "timeout",
                "test": target,
                "location": None,
                "detail": detail,
            }
        elif failed_names:
            first_failure = {
                "kind": "test_failure",
                "test": failed_names[0],
                "location": None,
                "detail": "test reported FAIL",
            }
        else:
            return None

    first_failure["timed_out_after_failure"] = bool(timed_out and first_failure["kind"] != "timeout")
    first_failure["panic_hook_cascade_count"] = len(cascade_tests)
    return first_failure


def format_wasm_failure_summary(output_or_summary: str | dict[str, object] | None) -> str:
    summary = summarize_wasm_failure(output_or_summary) if isinstance(output_or_summary, str) else output_or_summary
    if not summary:
        return ""
    labels = {
        "wasm_trap": "raw WASM trap",
        "rust_panic": "Rust panic/assertion",
        "timeout": "timeout",
        "test_failure": "test failure",
    }
    label = labels.get(str(summary.get("kind")), "failure")
    target = str(summary.get("test") or "<unknown test>")
    location = summary.get("location")
    where = f" at {location}" if location else ""
    parts = [f"first failure [{label}]: {target}{where} — {summary.get('detail', '')}"]
    cascade_count = int(summary.get("panic_hook_cascade_count") or 0)
    if cascade_count > 0:
        parts.append(
            f"follow-on symptom: {cascade_count} later test(s) hit "
            f"'{PANIC_HOOK_CASCADE_TEXT}' in shared WASM state (re-run after fixing first failure)"
        )
    if summary.get("timed_out_after_failure"):
        parts.append("later harness timeout was a follow-on symptom")
    return "; ".join(parts)


def launch_command(command, *, registry=None, **kwargs):
    if registry is not None:
        return _parallel_process(command, registry, **kwargs)
    return subprocess.run(command, **kwargs)


def run_serial(command: Command, environment: dict[str, str], cwd: Path = ROOT,
               captured: list[tuple[Command, str]] | None = None,
               outputs: list[tuple[Command, str]] | None = None,
               completed=None, build_capture=None, *, invocation=None, defer_output=False,
               tooling=False) -> tuple[float, int, str, str]:
    argv = command
    if invocation:
        argv, cwd, config = invocation
        environment = {**environment, **config}
    print("$ " + shlex.join(argv), flush=True)
    lock_fd = browser_tools._inherited_build_lock()
    if not any(script in command for script in BUILD_LOCK_SCRIPTS):
        environment = environment.copy()
        environment.pop("BOARDSTUDIO_BUILD_LOCK_FD", None)
        if tooling:
            lock_fd = None
    lock_args = {"pass_fds": (lock_fd,)} if lock_fd is not None else {}
    started = time.monotonic()
    started_at = datetime.now(timezone.utc).isoformat()
    print(f"check: start {started_at} $ {shlex.join(argv)}", flush=True)
    def run(*args, **kwargs):
        if build_capture is not None:
            build_capture("before", None)
        try:
            result = launch_command(*args, **kwargs)
        except OSError as error:
            message = f"startup failed: {error}\n"
            if kwargs.get("stdout") != subprocess.PIPE:
                print(message, end="", file=sys.stderr, flush=True)
            result = subprocess.CompletedProcess(command, 127, stdout=message)
        if build_capture is not None:
            build_capture("after", result.returncode)
        if completed is not None:
            completed(result.returncode, time.monotonic() - started, started_at, datetime.now(timezone.utc).isoformat())
        return result
    if is_crate_build(command) and outputs is not None:
        done = run(argv, cwd=cwd, env=environment, stdout=subprocess.PIPE, text=True, **lock_args)
        outputs.append((command, output_text(done.stdout)))
        code = done.returncode
    elif captured is not None and is_cargo_test(command):
        # The native test-name guard reads this output; it is printed whole once the command ends.
        done = run(command, cwd=cwd, env=environment, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, **lock_args)
        text = output_text(done.stdout)
        print(text, end="" if text.endswith("\n") or not text else "\n", flush=True)
        captured.append((command, text))
        if outputs is not None:
            outputs.append((command, text))
        code = done.returncode
    elif outputs is not None and (defer_output or is_crate_launch(command) or is_wasm_pack_test(command)
                                 or "scripts/run-drag-browser.py" in command):
        done = run(argv, cwd=cwd, env=environment, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, **lock_args)
        text = output_text(done.stdout)
        if not defer_output:
            print(text, end="" if text.endswith("\n") or not text else "\n", flush=True)
        outputs.append((command, text))
        code = done.returncode
    else:
        code = run(argv, cwd=cwd, env=environment, **lock_args).returncode
    elapsed = time.monotonic() - started
    ended_at = datetime.now(timezone.utc).isoformat()
    print(f"check: end {ended_at} exit {code} ({elapsed:.1f}s)", flush=True)
    return elapsed, code, started_at, ended_at


def _parallel_process(command, registry, **kwargs):
    with registry["lock"]:
        if registry["cancelled"]:
            return None
        try:
            # A worker inherits the scheduler's blocked signals. Restore the
            # caller's mask in a fresh interpreter, without preexec_fn.
            wrapper = """import os, signal, sys
# Python startup changes its environment and ignores signals that Popen restored.
with open('/proc/self/environ', 'rb') as source:
    environment = dict(entry.split(b'=', 1) for entry in source.read().split(b'\\0') if entry)
for name in ('SIGPIPE', 'SIGXFZ', 'SIGXFSZ'):
    if hasattr(signal, name):
        signal.signal(getattr(signal, name), signal.SIG_DFL)
signal.pthread_sigmask(signal.SIG_SETMASK, [int(s) for s in sys.argv[1].split(',') if s])
try:
    os.execvpe(sys.argv[2], sys.argv[2:], environment)
except OSError as error:
    print(f"startup failed: {error}", file=sys.stderr)
    sys.exit(127)
"""
            mask = ",".join(str(int(s)) for s in registry["mask"])
            process = subprocess.Popen([sys.executable, "-I", "-S", "-c", wrapper, mask, *command],
                                       start_new_session=not registry.get("inside_step_group"), **kwargs)
        except OSError as error:
            return subprocess.CompletedProcess(command, 127, f"startup failed: {error}\n")
        registry["processes"].add(process)
    # Observe completion without reaping the group leader. Keeping its PID
    # reserved until batch cleanup prevents a later cancellation signal from
    # targeting a reused process-group ID when descendants outlive the leader.
    try:
        decoder = io.IncrementalNewlineDecoder(
            codecs.getincrementaldecoder(process.stdout.encoding)(errors=process.stdout.errors), translate=True)
        chunks = []
        while data := process.stdout.buffer.read1(65536):
            chunks.append(decoder.decode(data))
        chunks.append(decoder.decode(b"", final=True))
        output = "".join(chunks)
    except BaseException as error:
        with registry["lock"]:
            registry["read_failed"] = error
        raise
    status = os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOWAIT)
    code = status.si_status if status.si_code == os.CLD_EXITED else -status.si_status
    return subprocess.CompletedProcess(command, code, output)


def run_parallel(batch: list[Command], environment: dict[str, str],
                 captured: list[tuple[Command, str]] | None = None,
                 outputs: list[tuple[Command, str]] | None = None,
                 receipt: verification_receipt.Collector | None = None,
                 step: str = "", *, invocations=None, workers=None, in_order=False,
                 elapsed_by_command=None, sequential_after=None, continue_on_failure=False,
                 validate_output=None) -> tuple[float, list[Command]]:
    """Runs independent launches together, printing each completed launch's output whole.

    `sequential_after` maps a command to the commands that run after it in its lane; they stop at
    the first failure unless `continue_on_failure`."""
    lock_fd = browser_tools._inherited_build_lock()
    invocations = invocations or {}
    # Workers retain completed results even if submit is interrupted before it
    # returns the corresponding future to the main thread.
    completed = []
    # Results of the commands each lane ran after its first, which is the lane's future result.
    followed: dict[tuple[str, ...], list] = {}
    registry = {"lock": threading.Lock(), "cancelled": False, "processes": set(), "workers": 0,
                "inside_step_group": IN_OVERLAP_ENV in os.environ}
    workers_changed = threading.Condition(registry["lock"])

    def launch(command: Command):
        with workers_changed:
            if registry["cancelled"]:
                return None
            registry["workers"] += 1
        try:
            result = last = run_worker(command)
            lane = followed.setdefault(tuple(command), [])
            for following in (sequential_after or {}).get(tuple(command), []) if result is not None else ():
                if last[0] != 0 and not continue_on_failure:
                    break
                last = run_worker(following)
                if last is None:
                    break
                lane.append((following, last))
            return result
        finally:
            with workers_changed:
                registry["workers"] -= 1
                workers_changed.notify_all()

    def run_worker(command: Command):
        attempt = receipt.prepare(command, step=step) if receipt is not None else None
        started = time.monotonic()
        started_at = datetime.now(timezone.utc).isoformat()
        argv, cwd, config = invocations.get(tuple(command), (command, ROOT, {}))
        print(f"check: start {started_at} $ {shlex.join(argv)}", flush=True)
        child_env = {**environment, **config}
        if receipt is not None:
            child_env.update(BOARDSTUDIO_RECEIPT_RUN_ID=receipt.run_id, BOARDSTUDIO_RECEIPT_ATTEMPT_ID=attempt)
        child_lock_fd = lock_fd
        if not any(script in command for script in BUILD_LOCK_SCRIPTS):
            child_env.pop("BOARDSTUDIO_BUILD_LOCK_FD", None)
            if step == "tooling":
                child_lock_fd = None
        done = launch_command(argv, registry=registry, cwd=cwd, env=child_env, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True,
                              **({"pass_fds": (child_lock_fd,)} if child_lock_fd is not None else {}))
        if done is None:
            return None
        code, output = done.returncode, done.stdout
        if validate_output is not None:
            code, output = validate_output(command, code, output)
        ended_at = datetime.now(timezone.utc).isoformat()
        elapsed = time.monotonic() - started
        try:
            print(f"check: end {ended_at} exit {code} $ {shlex.join(argv)}", flush=True)
        except BaseException:
            if receipt is not None:
                receipt.recording_failed(attempt, code, elapsed, started_at, ended_at)
            raise
        result = (code, output, elapsed, started_at, ended_at, attempt)
        completed.append((command, result))
        return result

    # The packaged lanes and then the page suite start before the heavy crates, so the longest
    # browser work overlaps the rest.
    ordered = batch if in_order else sorted(
        batch, key=lambda command: (-2 if command in (DRAG_SMOKE, CAD_BROWSER)
                                    else -1 if "scripts/run-wasm-tests.py" in command else launch_order(command)))
    started = time.monotonic()
    failed: list[Command] = []
    futures = []
    error = None
    submitting = True
    interrupted = None
    pool = ThreadPoolExecutor(max_workers=workers or check_jobs())
    main_thread = threading.current_thread() is threading.main_thread()
    signals = {signal.SIGINT, signal.SIGTERM}
    previous_int = signal.getsignal(signal.SIGINT) if main_thread else None
    previous_term = signal.getsignal(signal.SIGTERM) if main_thread else None
    previous_mask = signal.pthread_sigmask(signal.SIG_BLOCK, signals) if main_thread else None
    registry["mask"] = previous_mask if main_thread else signal.pthread_sigmask(signal.SIG_BLOCK, set())
    def emit(command, result):
        code, output, elapsed, started_at, ended_at, attempt = result
        text = output_text(output)
        if receipt is not None:
            receipt.observe(step, command, code, elapsed, text, started_at=started_at,
                            ended_at=ended_at, attempt_id=attempt)
        if captured is not None and is_cargo_test(command):
            captured.append((command, text))
        if outputs is not None:
            outputs.append((command, text))
        if code:
            failed.append(command)
        if elapsed_by_command is not None:
            elapsed_by_command[tuple(command)] = elapsed
        argv, _, _ = invocations.get(tuple(command), (command, ROOT, {}))
        print("$ " + shlex.join(argv), flush=True)
        print(text, end="" if text.endswith("\n") or not text else "\n", flush=True)
        print(f"  ({elapsed:.1f}s)", flush=True)

    try:
        if main_thread:
            def interrupt(signum, _frame):
                nonlocal interrupted
                interrupted = signum
                signal.pthread_sigmask(signal.SIG_BLOCK, signals)
                if sys.exception() is not None:
                    return
                if signum == signal.SIGINT:
                    raise KeyboardInterrupt
                raise SystemExit(143)
            signal.signal(signal.SIGINT, interrupt)
            signal.signal(signal.SIGTERM, interrupt)
            signal.pthread_sigmask(signal.SIG_SETMASK, previous_mask)
        if main_thread:
            signal.pthread_sigmask(signal.SIG_BLOCK, signals)
        try:
            followers = {tuple(cmd) for lane in (sequential_after or {}).values() for cmd in lane}
            for command in ordered:
                if tuple(command) not in followers:
                    futures.append((command, pool.submit(launch, command)))
        finally:
            if main_thread:
                signal.pthread_sigmask(signal.SIG_SETMASK, previous_mask)
        submitting = False
        pending = {future for _, future in futures}
        commands_by_future = {future: command for command, future in futures}
        while pending:
            finished, pending = wait(pending, return_when=FIRST_COMPLETED)
            with registry["lock"]:
                read_error = registry.get("read_failed")
            if read_error is not None:
                raise read_error
            for future in finished:
                if future.cancelled():
                    continue
                command = commands_by_future[future]
                result = future.result()
                if not in_order:
                    emit(command, result)
                    for following, following_result in followed.get(tuple(command), []):
                        emit(following, following_result)
    except BaseException as caught:
        error = caught
    finally:
        cancel = interrupted or (error is not None and (submitting or isinstance(error, (KeyboardInterrupt, SystemExit)) or registry.get("read_failed", False)))
        cleanup_incomplete = False
        previous_handler = None
        def suppress_signals():
            nonlocal previous_handler
            mask = signal.pthread_sigmask(signal.SIG_BLOCK, signals)
            previous_handler = signal.signal(signal.SIGINT, signal.SIG_IGN)
            signal.signal(signal.SIGTERM, signal.SIG_IGN)
            signal.pthread_sigmask(signal.SIG_SETMASK, mask)

        if cancel and main_thread:
            suppress_signals()
        def cancel_workers():
            with registry["lock"]:
                registry["cancelled"] = True
                processes = list(registry["processes"])
            for _, future in futures:
                future.cancel()
            for process in processes:
                try:
                    if registry["inside_step_group"]:
                        process.terminate()
                    else:
                        os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            _, pending = wait([f for _, f in futures], timeout=0.5)
            for process in processes:
                try:
                    if registry["inside_step_group"]:
                        process.kill()
                    else:
                        os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            _, pending = wait(pending, timeout=2)
            with workers_changed:
                workers_done = workers_changed.wait_for(lambda: registry["workers"] == 0, timeout=2)
            return bool(pending) or not workers_done

        try:
            if cancel:
                cleanup_incomplete = cancel_workers()
            if hasattr(pool, "shutdown"):
                try:
                    pool.shutdown(wait=not cleanup_incomplete, cancel_futures=cancel)
                except (KeyboardInterrupt, SystemExit) as caught:
                    if error is None:
                        error = caught
                    if previous_handler is None and main_thread:
                        suppress_signals()
                    cleanup_incomplete = cancel_workers()
                    try:
                        pool.shutdown(wait=not cleanup_incomplete, cancel_futures=True)
                    except (KeyboardInterrupt, SystemExit):
                        cleanup_incomplete = True
            # Signals are finished; only now may group leaders be reaped and
            # their process-group identifiers become reusable.
            with registry["lock"]:
                processes = list(registry["processes"])
            for process in processes:
                try:
                    process.wait(timeout=0.1)
                except subprocess.TimeoutExpired:
                    cleanup_incomplete = True
                finally:
                    # A reader blocked on this stream holds its internal lock;
                    # closing it here would itself block cleanup indefinitely.
                    if not cleanup_incomplete and process.stdout is not None:
                        process.stdout.close()
            if cleanup_incomplete:
                # ThreadPoolExecutor joins workers at interpreter exit even after shutdown(wait=False).
                print("check: cleanup incomplete; worker termination is unconfirmed; "
                      "interpreter exit may wait indefinitely for Python workers", file=sys.stderr, flush=True)
        finally:
            if main_thread:
                signal.pthread_sigmask(signal.SIG_BLOCK, signals)
                signal.signal(signal.SIGINT, previous_int)
                signal.signal(signal.SIGTERM, previous_term)
                signal.pthread_sigmask(signal.SIG_SETMASK, previous_mask)
        # Include futures from test executors and results from submissions that
        # raised after scheduling work but before returning their future.
        for command, future in futures:
            if future.cancelled() or (cleanup_incomplete and not future.done()):
                continue
            try:
                result = future.result()
                if result is not None and (command, result) not in completed:
                    completed.append((command, result))
            except BaseException as caught:
                if error is None:
                    error = caught
        if receipt is not None and error is not None:
            for command, result in completed:
                code, output, elapsed, started_at, ended_at, attempt = result
                try:
                    receipt.observe(step, command, code, elapsed, output_text(output),
                                    started_at=started_at, ended_at=ended_at, attempt_id=attempt)
                except BaseException as caught:
                    receipt.recording_failed(attempt, code, elapsed, started_at, ended_at)
                    if error is None:
                        error = caught
    if in_order:
        results = {tuple(command): result for command, result in completed}
        for command in ordered:
            if result := results.get(tuple(command)):
                emit(command, result)
    if error is None and interrupted is not None:
        error = KeyboardInterrupt() if interrupted == signal.SIGINT else SystemExit(143)
    if error is not None:
        raise error
    return time.monotonic() - started, failed


def _report_launch_failure(command: Command, text: str) -> None:
    print("check: launch failed: " + shlex.join(command), file=sys.stderr)
    if summary := format_wasm_failure_summary(text):
        print("check: " + summary, file=sys.stderr)


def run_lint_overlap(commands: list[Command], environment: dict[str, str], cwd: Path) -> tuple[float, bool]:
    started = time.monotonic()
    for command in commands:
        if command[:2] == ["cargo", "fmt"] and run_serial(command, environment, cwd)[1]:
            return time.monotonic() - started, False
    harness = next(command for command in commands if is_harness_clippy(command))
    root = [command for command in commands if command[:2] != ["cargo", "fmt"] and command != harness]
    outputs: list[tuple[Command, str]] = []
    with ThreadPoolExecutor(max_workers=1, thread_name_prefix="harness-clippy") as pool:
        harness_env = environment.copy()
        if IN_OVERLAP_ENV in harness_env:
            harness_env.pop("CARGO_TARGET_DIR", None)
        harness_run = pool.submit(run_serial, harness, harness_env, cwd, outputs=outputs, defer_output=True)
        passed = all(run_serial(command, environment, cwd)[1] == 0 for command in root)
        harness_code = harness_run.result()[1]
    for _, output in outputs:
        print(output, end="" if output.endswith("\n") or not output else "\n", flush=True)
    if harness_code:
        print("check: command failed: " + shlex.join(harness), file=sys.stderr)
    return time.monotonic() - started, passed and harness_code == 0


def run_commands(name: str, commands: list[Command], environment: dict[str, str], cwd: Path = ROOT,
                 captured: list[tuple[Command, str]] | None = None,
                 *, keep_going: bool = False,
                 receipt: verification_receipt.Collector | None = None,
                 crate_configs: dict[str, dict[str, str]] | None = None) -> tuple[float, bool]:
    """Runs a step's commands; consecutive crate launches in `browser`, and the independent
    workspaces of `test` and independent tooling self-tests, run together.
    `captured` collects each `cargo test` command's output."""
    if name == "test" and check_jobs() > 1 and len(commands) > 1:
        # Both build in the root target. The parts importer follows the workspace tests, whose build
        # leaves its examples current, instead of waiting beside them on Cargo's target lock.
        chain = ({tuple(CARGO_TESTS[0]): [KICAD_PARTS_TEST]}
                 if CARGO_TESTS[0] in commands and KICAD_PARTS_TEST in commands else None)
        elapsed, failed = run_parallel(commands, environment, captured, receipt=receipt, step=name,
                                       sequential_after=chain, continue_on_failure=True)
        for command in failed:
            print("check: command failed: " + shlex.join(command), file=sys.stderr)
        return elapsed, not failed
    if name == "lint" and check_jobs() > 1 and receipt is None and any(map(is_harness_clippy, commands)):
        return run_lint_overlap(commands, environment, cwd)
    browser_step = is_browser_step(name)
    tooling_workers = check_jobs() if name == "tooling" else 1
    total = 0.0
    any_failed = False
    invocations = {}
    # The packaged checks join the parallel batch that runs the page suite (and the crate launches
    # before it): the drag smoke in one lane, the CAD browser check and then the harness checks in another.
    lanes: list[Command] = []
    if browser_step and check_jobs() > 1 and any(map(is_page_suite, commands)):
        lanes = [command for command in commands if command in (DRAG_SMOKE, CAD_BROWSER, *HARNESS_CHECKS)]
        commands = [command for command in commands if command not in lanes]
    index = 0
    stage_command: Command | None = None
    while index < len(commands):
        command = commands[index]
        # The checked page build splits after its last Cargo invocation, so its file staging
        # overlaps the wasm test prebuilds below. The `dx` fallback, single-job and receipt
        # runs keep the exact serial scheduling.
        tail = commands[1:]
        tail_wasm = [cmd for cmd in tail if cmd in (BROWSER_CRATE_BUILD, PAGE_TEST_BUILD)]
        splittable_build = tail == [*tail_wasm, HARNESS_BUILD]
        if (name == "build" and index == 0 and command == [PY, "scripts/build-web.py", "--fast"]
                and splittable_build
                and check_jobs() > 1 and receipt is None and plain_page_tools_available()):
            cargo_command = [PY, "scripts/build-web.py", "--fast", "--phase", "cargo"]
            elapsed, code, _, _ = run_serial(cargo_command, environment, cwd)
            total += elapsed
            if code:
                print("check: command failed: " + shlex.join(cargo_command), file=sys.stderr)
                return total, False
            stage_command = [PY, "scripts/build-web.py", "--fast", "--phase", "stage"]
            index += 1
            continue
        # The page build must produce CSS first; afterward the root-target prebuilds share one
        # lane while the independent harness (and page staging, when split) build in another.
        prebuilds = commands[index:]
        wasm_prebuilds = [cmd for cmd in prebuilds if cmd in (BROWSER_CRATE_BUILD, PAGE_TEST_BUILD)]
        if (name == "build" and check_jobs() > 1 and receipt is None
                and index > 0 and commands[0] == [PY, "scripts/build-web.py", "--fast"]
                and prebuilds == [*wasm_prebuilds, HARNESS_BUILD]
                and (wasm_prebuilds or stage_command)):
            child_env = environment.copy()
            child_env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
            def validate_prebuild(cmd, code, output):
                if not is_crate_build(cmd):
                    return code, output
                rendered = io.StringIO()
                packages = [cmd[i + 1] for i, token in enumerate(cmd[:-1]) if token == "-p"]
                crates = tuple(package.removeprefix("boardstudio-web-")
                               for package in packages if package != PAGE_PACKAGE)
                try:
                    crate_test_artifacts(output_text(output), cwd, () if code else crates, stream=rendered)
                    if not code and PAGE_PACKAGE in packages:
                        global _PAGE_TEST_ARTIFACT
                        _PAGE_TEST_ARTIFACT = page_test_artifact(output_text(output), cwd)
                except (ValueError, RuntimeError) as error:
                    return 1, rendered.getvalue() + f"check: {error}\n"
                return code, rendered.getvalue()

            combined = (combined_wasm_prebuild(BROWSER_CRATE_BUILD)
                        if wasm_prebuilds == [BROWSER_CRATE_BUILD, PAGE_TEST_BUILD] else None)
            wasm_lane = [combined] if combined else wasm_prebuilds
            batch = ([stage_command] if stage_command else []) + wasm_lane + [HARNESS_BUILD]
            elapsed, failed = run_parallel(
                batch, child_env, captured, step=name, workers=3 if stage_command else 2, in_order=True,
                invocations={tuple(cmd): (cmd, cwd, {}) for cmd in batch},
                sequential_after={tuple(wasm_lane[0]): wasm_lane[1:]} if wasm_lane else None,
                validate_output=validate_prebuild,
            )
            total += elapsed
            for failed_cmd in failed:
                print("check: command failed: " + shlex.join(failed_cmd), file=sys.stderr)
            return total, not failed
        if tooling_workers > 1 and not SERIAL_TOOLING_SCRIPTS.intersection(command):
            end = index + 1
            while end < len(commands) and not SERIAL_TOOLING_SCRIPTS.intersection(commands[end]):
                end += 1
            batch = commands[index:end]
            elapsed, failed = run_parallel(
                batch, environment, receipt=receipt, step=name, workers=tooling_workers, in_order=True,
                invocations={tuple(cmd): (cmd, cwd, {}) for cmd in batch},
            )
            total += elapsed
            for failed_cmd in failed:
                print("check: command failed: " + shlex.join(failed_cmd), file=sys.stderr)
            any_failed |= bool(failed)
            index = end
            continue
        if browser_step and check_jobs() > 1 and (is_crate_launch(command) or (lanes and is_page_suite(command))):
            end = index
            while end < len(commands) and is_crate_launch(commands[end]):
                end += 1
            workers = check_jobs()
            if end < len(commands) and is_page_suite(commands[end]):
                page = commands[end]
                page_workers = page_suite_jobs(workers)
                invocations[tuple(page)] = (page, cwd, {"BOARDSTUDIO_CHECK_JOBS": str(page_workers)})
                workers = max(1, workers - (page_workers - 1))
                end += 1
            batch, lanes = [*lanes, *commands[index:end]], []
            heads = [command for command in batch if command in (DRAG_SMOKE, CAD_BROWSER)]
            chained = [command for command in HARNESS_CHECKS if command in batch]
            if chained and CAD_BROWSER not in batch:
                heads.append(chained.pop(0))
            batch_outputs: list[tuple[Command, str]] = []
            elapsed, failed = run_parallel(batch, environment, outputs=batch_outputs,
                                           receipt=receipt, step=name, invocations=invocations, workers=workers,
                                           sequential_after={tuple(heads[-1]): chained} if chained else None,
                                           continue_on_failure=keep_going)
            total += elapsed
            if failed:
                output_by_cmd = {tuple(cmd): text for cmd, text in batch_outputs}
                for failed_cmd in failed:
                    if is_crate_launch(failed_cmd):
                        _report_launch_failure(failed_cmd, output_by_cmd.get(tuple(failed_cmd), ""))
                    else:
                        print("check: command failed: " + shlex.join(failed_cmd), file=sys.stderr)
                if not keep_going:
                    return total, False
                any_failed = True
            index = end
            continue
        serial_outputs: list[tuple[Command, str]] = []
        attempt = receipt.prepare(command, step=name) if receipt is not None else None
        completed = (lambda code, elapsed, start, end: receipt.recording_failed(attempt, code, elapsed, start, end)) if receipt is not None else None
        child_env = environment.copy()
        if command in BROWSER_PREBUILDS or name == "drag-smoke":
            child_env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
        if receipt is not None:
            child_env.update(BOARDSTUDIO_RECEIPT_RUN_ID=receipt.run_id, BOARDSTUDIO_RECEIPT_ATTEMPT_ID=attempt)
        args = (command, child_env, cwd, captured, serial_outputs)
        build_capture = None
        if receipt is not None and "scripts/build-web.py" in command:
            state = {}
            site = cwd / "web/target/site"
            routes = "subpath"
            for position, arg in enumerate(command):
                if arg.startswith("--routes="):
                    routes = arg.split("=", 1)[1]
                elif arg == "--routes" and position + 1 < len(command):
                    routes = command[position + 1]
                elif arg.startswith("--output="):
                    site = cwd / arg.split("=", 1)[1]
                elif arg == "--output" and position + 1 < len(command):
                    site = cwd / command[position + 1]
            site = site.resolve()
            observes_subpath = ("--providers-only" not in command and
                                "subpath" in (route.strip() for route in routes.split(",")))
            def build_capture(phase, code):
                source = verification_receipt.source_identity(cwd)
                if phase == "before":
                    state["source"] = source
                    _, state["site"] = verification_receipt.site_identity(site)
                    receipt.build_bindings.append(verification_receipt.build_binding(
                        receipt.run_id, attempt, source, None, None, None, False))
                else:
                    output, site_after = verification_receipt.site_identity(site) if code == 0 and observes_subpath else (None, None)
                    receipt.build_bindings[-1] = verification_receipt.build_binding(
                        receipt.run_id, attempt, state.get("source"), source, code, output,
                        verification_receipt.fresh_site(state.get("site"), site_after))
        if invocation := invocations.get(tuple(command)):
            elapsed, code, started_at, ended_at = run_serial(*args, completed, invocation=invocation)
        elif name == "tooling":
            elapsed, code, started_at, ended_at = run_serial(*args, completed, tooling=True)
        else:
            elapsed, code, started_at, ended_at = (run_serial(*args, completed, build_capture)
                                                    if build_capture else
                                                    run_serial(*args, completed) if completed else run_serial(*args))
        total += elapsed
        if receipt is not None:
            try:
                receipt.observe(name, command, code, elapsed, serial_outputs[-1][1] if serial_outputs else "",
                                started_at=started_at, ended_at=ended_at, attempt_id=attempt)
            except BaseException:
                receipt.recording_failed(attempt, code, elapsed, started_at, ended_at)
                raise
        if is_crate_build(command):
            end = index + 1
            while end < len(commands) and commands[end][:1] == [wasm_bindgen_tools.TEST_RUNNER]:
                end += 1
            launches = commands[index + 1:end]
            try:
                crates = tuple(command[idx + 1].removeprefix("boardstudio-web-")
                               for idx, token in enumerate(command) if token == "-p")
                artifacts = crate_test_artifacts(serial_outputs[-1][1], cwd, () if code else crates)
                if not code and launches:
                    pair = environment.get(wasm_bindgen_tools.DIRECTORY_ENV)
                    if not pair:
                        raise ValueError("browser preflight did not select a wasm-bindgen pair")
                    directory = Path(pair)
                    for launch in launches:
                        crate = launch_crate(launch)
                        invocations[tuple(launch)] = (
                            [str(directory / wasm_bindgen_tools.TEST_RUNNER), str(artifacts[crate]), *launch[2:]],
                            cwd / launch[1], dict(crate_configs[crate], RAYON_NUM_THREADS="1"),
                        )
            except (ValueError, RuntimeError) as error:
                print(f"check: {error}", file=sys.stderr)
                code = code or 1
            if code:
                print("check: command failed: " + shlex.join(command), file=sys.stderr)
                if not keep_going:
                    return total, False
                # No runner may consume incomplete or stale build artifacts.
                index = end
                any_failed = True
                continue
        if code:
            if tooling_workers > 1:
                print("check: command failed: " + shlex.join(command), file=sys.stderr)
                any_failed = True
                index += 1
                continue
            text = serial_outputs[-1][1] if serial_outputs else ""
            if is_crate_launch(command):
                _report_launch_failure(command, text)
            elif keep_going and browser_step:
                print("check: command failed: " + shlex.join(command), file=sys.stderr)
                if is_wasm_pack_test(command) and (summary := format_wasm_failure_summary(text)):
                    print("check: " + summary, file=sys.stderr)
            else:
                if is_wasm_pack_test(command) and (summary := format_wasm_failure_summary(text)):
                    print("check: " + summary, file=sys.stderr)
                return total, False
            # The page `--list` command is the prerequisite for `run-wasm-tests.py --all`.
            if not keep_going or not browser_step or (is_wasm_pack_test(command) and "--list" in command):
                return total, False
            any_failed = True
        index += 1
    return total, not any_failed


def run(names: list[str], update_native_baseline: bool = False,
        *, only: Sequence[str] | None = None, keep_going: bool = False) -> int:
    names = list(names)
    if only is not None:
        if names != ["browser"]:
            raise ValueError("--only applies to `browser` alone")
        only_crates = parse_only_crates(only)
        plan: list[tuple[str, list[Command]]] = []
        if CSS_TEST_CRATES.intersection(only_crates):
            plan.append(("test-css", [test_css_command()]))
        plan.append((f"browser (only: {', '.join(only_crates)})",
                     with_crate_build(select_browser_crate_launches(only_crates))))
        return (run_plan(plan, update_native_baseline, keep_going=True)
                if keep_going else run_plan(plan, update_native_baseline))
    # The real-page smoke needs a current packaged page, including for `check.py browser` alone.
    if _RECEIPT is not None:
        _RECEIPT.plan_commands([(name, STEPS[name][1]) for name in names])
    has_drag_smoke = any("scripts/run-drag-browser.py" in command for command in STEPS["browser"][1])
    smoke_indices = [index for index, step in enumerate(names)
                     if step in ("drag-smoke", "browser-packaged") or (step == "browser" and has_drag_smoke)]
    if smoke_indices:
        # Fail before the page build, which is minutes of work the smoke cannot use without these.
        # The opt-in Rust driver needs only Chrome; the default Node runner also needs its client.
        try:
            browser_tools.validate_drag_driver()
            browser_tools.locate_chrome()
        except browser_tools.MissingBrowserTool as error:
            print(f"check: {error}", file=sys.stderr)
            return 1
        smoke_index = smoke_indices[0]
        built = any(step in ("build", "release") for step in names[:smoke_index])
        if not built:
            later_build = next((index for index in range(smoke_index + 1, len(names))
                                if names[index] in ("build", "release")), None)
            if later_build is None:
                names.insert(smoke_index, "build")
            else:
                names.insert(smoke_index, names.pop(later_build))
    plan = [(name, STEPS[name][1]) for name in names]
    # Hosted packaged checks keep their existing partition from the crate suite.
    if "browser" not in names and any(name in names for name in ("browser-packaged", "drag-smoke")):
        unused = [BROWSER_CRATE_BUILD]
        if "browser-packaged" not in names:
            unused.append(PAGE_TEST_BUILD)
        plan = [(name, [command for command in commands if command not in unused] if name == "build" else commands)
                for name, commands in plan]
    for index, name in enumerate(names):
        if name in ("build", "release"):
            break  # The packaged build generates the stylesheet.
        if name in ("lint", "browser-wasm"):
            if name != "lint" or IN_OVERLAP_ENV not in os.environ:
                plan.insert(index, ("test-css", [test_css_command()]))
            break
    return (run_plan(plan, update_native_baseline, keep_going=True)
            if keep_going else run_plan(plan, update_native_baseline))


def guard_native_tests(commands: list[Command], captured: list[tuple[Command, str]],
                       update: bool) -> tuple[bool, bool]:
    """The test-name guard over a passing `test` step. Returns (ok, baseline_written)."""
    if not captured:
        return True, False
    # Only a step that ran the whole workspace plus footprints can say a baselined binary vanished.
    full = all(command in commands for command in CARGO_TESTS)
    return check_native_tests.verify([text for _, text in captured], full=full, update=update)


def cargo_target() -> Path:
    metadata = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
                              cwd=ROOT, capture_output=True, text=True, check=True)
    target = Path(json.loads(metadata.stdout)["target_directory"])
    if not target.is_absolute():
        raise ValueError("target directory is not absolute")
    return target


def build_preflight(*, cad: bool = False, cad_wasm: bool = False) -> bool:
    try:
        browser_tools._inherited_build_lock()
    except SystemExit as error:
        print(f"check: {error}", file=sys.stderr)
        return False
    def report_target(target: Path) -> None:
        ancestor = target
        while not ancestor.exists() and ancestor != ancestor.parent:
            ancestor = ancestor.parent
        free = shutil.disk_usage(ancestor).free
        print(f"check: build target filesystem free: {free / (1024 ** 3):.1f} GiB ({target}; measured at {ancestor})", flush=True)

    try:
        report_target(cargo_target())
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, TypeError) as error:
        print(f"check: build target unknown: {error}", file=sys.stderr)
    cad_targets = []
    if cad:
        cad_targets.extend(("cad/monstertruck/target", "cad/monstertruck-qualification/target"))
    if cad_wasm:
        cad_targets.append("cad/wasm/target")
    if cad_targets:
        for target in cad_targets:
            try:
                report_target(ROOT / target)
            except OSError as error:
                print(f"check: CAD build target unknown ({target}): {error}", file=sys.stderr)
    temporary = Path(os.environ.get("TMPDIR") or tempfile.gettempdir())
    try:
        free = shutil.disk_usage(temporary).free
        print(f"check: temporary filesystem free: {free / (1024 ** 3):.1f} GiB ({temporary})", flush=True)
    except OSError as error:
        print(f"check: cannot determine temporary filesystem free space ({temporary}): {error}", file=sys.stderr)
    return True


def run_plan(plan: list[tuple[str, list[Command]]], update_native_baseline: bool = False,
             *, keep_going: bool = False, partial: bool = False) -> int:
    if _RECEIPT is not None:
        _RECEIPT.plan_commands(plan)
    cad_build = any("cad/scripts/test-cad.py" in command for _, commands in plan for command in commands)
    cad_wasm = any("scripts/build-web.py" in command and "--skip-providers" not in command
                   for _, commands in plan for command in commands)
    if (cad_build or cad_wasm or any(commands and (name in ("build", "release", "browser", "drag-smoke", "layout-test-css")
                         or name.startswith("browser (only:")
                         or any(command and command[0] in ("cargo", "wasm-pack") for command in commands))
                         for name, commands in plan)) and not build_preflight(cad=cad_build, cad_wasm=cad_wasm):
        return 1
    direct = any(is_wasm_pack_test(command) for _, commands in plan for command in commands)
    page = not os.environ.get("BOARDSTUDIO_WASM_TEST_COMMAND") and any(
        "scripts/run-wasm-tests.py" in command and command != PAGE_TEST_BUILD
        for _, commands in plan for command in commands)
    crate_launches = [command for _, commands in plan for command in commands
                      if command[:1] == [wasm_bindgen_tools.TEST_RUNNER]]
    if direct or page or crate_launches:
        wasm_runner = importlib.import_module("run-wasm-tests")
        try:
            environment = wasm_runner.runner_environment(ROOT)
            with ExitStack() as configs:
                crate_configs = {}
                for crate in browser_crates(crate_launches):
                    configured = configs.enter_context(
                        wasm_runner.desktop_webdriver_config(environment, ROOT / "web/crates" / crate))
                    crate_configs[crate] = {
                        wasm_runner.WEBDRIVER_CONFIG_ENV: configured[wasm_runner.WEBDRIVER_CONFIG_ENV],
                        # wasm-pack 0.15's --headless --chrome sets this and CHROMEDRIVER.
                        "WASM_BINDGEN_TEST_ONLY_WEB": "1",
                    }
                if direct or page:
                    environment = configs.enter_context(wasm_runner.desktop_webdriver_config(environment, ROOT))
                selected = [(name, [command[:4] + ["--chromedriver", environment["CHROMEDRIVER"]] + command[4:]
                                    if is_wasm_pack_test(command) else command for command in commands])
                            for name, commands in plan]
                return _run_plan(selected, update_native_baseline, keep_going=keep_going,
                                 partial=partial, browser_environment=environment, crate_configs=crate_configs)
        except wasm_runner.RunnerError as error:
            print(f"check: {error}", file=sys.stderr)
            return 1
    return _run_plan(plan, update_native_baseline, keep_going=keep_going, partial=partial)


def overlap_span(plan, receipt):
    cpus = os.cpu_count() or 1
    if receipt is not None or os.environ.get("CI") or check_jobs() <= 1 or cpus < OVERLAP_MIN_CPUS:
        return None
    for start in range(len(plan) - 2):
        group = plan[start:start + 3]
        if ({name for name, _ in group} == OVERLAP_STEPS
                and all(commands == STEPS[name][1] for name, commands in group)):
            # The overlapped steps share a stylesheet generated up front. Every plain-page
            # tool must exist: with only Tailwind, `build` would fall back to `dx build`,
            # regenerating the stylesheet WASM Clippy is reading.
            tools = importlib.import_module("build-web").page_tools(quiet=True)
            return (start, start + 3) if tools is not None else None
    return None


@contextmanager
def overlap_jobserver(directory, cpus):
    parent_flags = os.environ.get("MAKEFLAGS", "")
    if "--jobserver" in parent_flags:
        yield parent_flags
        return
    path = directory / "jobserver"
    os.mkfifo(path, 0o600)
    fd = os.open(path, os.O_RDWR)
    try:
        os.write(fd, b"+" * (cpus - 1))
        yield f"{parent_flags} --jobserver-auth=fifo:{path}".strip()
    finally:
        os.close(fd)


def run_overlapped_steps(group, update_native_baseline):
    # The page build and WASM Clippy both read the stylesheet, so it is written once before either starts.
    print("== test-css", flush=True)
    css_elapsed, css_code, _, _ = run_serial(test_css_command(), os.environ.copy(), ROOT / "web")
    if css_code:
        return {"test-css": css_elapsed}, ["test-css"]
    batch: list[Command] = []
    timings: dict[tuple[str, ...], float] = {}
    with tempfile.TemporaryDirectory(prefix="check-compile-") as temporary:
        directory = Path(temporary)
        with overlap_jobserver(directory, os.cpu_count() or OVERLAP_MIN_CPUS) as makeflags:
            invocations = {}
            for name, _ in group:
                command = [sys.executable, "scripts/check.py", name]
                if name == "test" and update_native_baseline:
                    command.append("--update-native-baseline")
                config = {"MAKEFLAGS": makeflags, IN_OVERLAP_ENV: "1"}
                if name == "lint":
                    config["CARGO_TARGET_DIR"] = str(ROOT / "target/lint")
                batch.append(command)
                invocations[tuple(command)] = (command, ROOT, config)
            _, failures = run_parallel(batch, os.environ.copy(), invocations=invocations,
                                       in_order=True, elapsed_by_command=timings)
    return ({"test-css": css_elapsed, **{command[2]: timings[tuple(command)] for command in batch}},
            [command[2] for command in failures])


def _run_plan(plan, update_native_baseline=False, *, keep_going=False, partial=False,
              browser_environment=None, crate_configs=None):
    global _PAGE_TEST_ARTIFACT
    _PAGE_TEST_ARTIFACT = None
    names = [name for name, _ in plan]
    # `--only` and any named-step/quick selection are focused; only the full default plan is not.
    focused = partial or tuple(names) != DEFAULT
    partial = partial or any(name.startswith("browser (only:") for name in names)
    baseline_written = False
    totals: dict[str, float] = {}
    receipt = _RECEIPT
    if receipt is not None:
        receipt.plan_commands(plan)

    def finish(code: int) -> int:
        global _RECEIPT
        if receipt is not None:
            receipt.finish(code, partial=focused, source_after=verification_receipt.source_identity(ROOT))
        _RECEIPT = None
        return code

    def run_step(name, commands):
        nonlocal baseline_written
        print(f"== {name}", flush=True)
        environment = os.environ.copy()
        if "BUILDTIME_LOG" in environment:
            environment["BUILDTIME_STEP"] = name
        if name == "test":
            # An editor's AppImage directory redirects system KiCad's library lookup.
            environment.pop("APPDIR", None)
        if is_browser_step(name):
            if browser_environment is not None:
                environment.update(browser_environment)
            else:
                try:
                    environment = wasm_bindgen_tools.configure(environment, ROOT)
                except ValueError as error:
                    print(f"check: {error}", file=sys.stderr)
                    return False
            # Match the page runner's bounded budget for browser crate batches.
            environment.setdefault("WASM_BINDGEN_TEST_TIMEOUT", "120")
            # Test wasm without DWARF is about a seventh of the size, which is most of a launch.
            environment.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
            # The combined prebuild's page artifact is still fresh: nothing between the build
            # and browser steps touches its fingerprints, so the page suite reuses it directly.
            if _PAGE_TEST_ARTIFACT is not None:
                environment.setdefault(PAGE_TEST_ARTIFACT_ENV, str(_PAGE_TEST_ARTIFACT))
        captured: list[tuple[Command, str]] | None = [] if name == "test" else None
        cwd = ROOT / "web" if name == "test-css" else ROOT
        elapsed, passed = run_commands(
            name, commands, environment, cwd, captured,
            keep_going=keep_going and is_browser_step(name),
            receipt=receipt,
            crate_configs=crate_configs,
        )
        totals[name] = totals.get(name, 0.0) + elapsed
        if captured and receipt is not None:
            receipt.record_native([text for _, text in captured],
                                  full=all(command in commands for command in CARGO_TESTS))
        if passed and captured:
            passed, written = guard_native_tests(commands, captured, update_native_baseline)
            baseline_written = baseline_written or written
        if not passed:
            print(f"check: step '{name}' failed", file=sys.stderr)
        return passed

    span = overlap_span(plan, receipt)
    start_time = time.monotonic() if span is not None else None
    if span is None:
        before, after = plan, []
    else:
        first, end = span
        before, after = plan[:first], plan[end:]
    for name, commands in before:
        if not run_step(name, commands):
            return finish(1)
    if span is not None:
        elapsed, failed = run_overlapped_steps(plan[first:end], update_native_baseline)
        totals.update(elapsed)
        if failed:
            for name in failed:
                print(f"check: step '{name}' failed", file=sys.stderr)
            return finish(1)
        baseline_written |= update_native_baseline
    for name, commands in after:
        if not run_step(name, commands):
            return finish(1)
    if update_native_baseline and not baseline_written:
        print("check: --update-native-baseline: no native `cargo test` ran, so the baseline is unchanged",
              file=sys.stderr)
        return finish(1)
    total_wall = time.monotonic() - start_time if start_time is not None else sum(totals.values())
    print("check: step times: " + ", ".join(f"{name} {seconds:.1f}s" for name, seconds in totals.items())
          + f", total {total_wall:.1f}s")
    if partial:
        print("check: partial steps passed: " + ", ".join(dict.fromkeys(names))
              + " (subset only; full browser gate not run)")
    else:
        print("check: all steps passed: " + ", ".join(dict.fromkeys(names)))
    return finish(0)


def quick_plan(selection: "check_quick.Selection", packages: "dict[str, check_quick.Package]") -> list[tuple[str, list[Command]]]:
    """Run affected checks and required test assets, without the packaged frontend build."""
    plan: list[tuple[str, list[Command]]] = [("repo", STEPS["repo"][1])]
    if selection.tooling:
        plan.append(("tooling", STEPS["tooling"][1]))
    if selection.rust:
        plan += [("test-css", [test_css_command()]), ("lint", LINT), ("typecheck", typecheck())]
        if selection.full_rust:
            native = list(CARGO_TESTS)
        else:
            names = sorted(selection.packages)
            native = [["cargo", "test", "--locked", *[part for name in names for part in ("-p", name)]]] if names else []
        plan.append(("test", native))
        crates = {Path(package.directory).name for name, package in packages.items()
                  if name in selection.packages and package.directory.startswith("web/crates/")}
        if selection.full_rust:
            # Quick deliberately skips checks requiring a packaged site.
            commands = [command for command in browser() if not any(name in command for name in
                        ("scripts/run-drag-browser.py", "cad/scripts/test-cold-responses.py"))]
        else:
            commands = with_crate_build(select_browser_crate_launches(crates, browser()))
            if selection.page_files:
                commands.append([PY, "scripts/run-wasm-tests.py", "--files", *selection.page_files,
                                 "--depth", "1",
                                 "--result-json", "web/target/test-results/browser-quick.json"])
        if commands:
            plan.append(("browser", commands))
    if selection.cad:
        plan.append(("test", [[PY, "cad/scripts/test-cad.py"]]))
    if selection.kicad_parts:
        plan.append(("test", [KICAD_PARTS_TEST]))
    return plan


def quick(files: list[str] | None, update_native_baseline: bool = False,
          *, keep_going: bool = False) -> int:
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
    return (run_plan(plan, update_native_baseline, keep_going=True)
            if keep_going else run_plan(plan, update_native_baseline))


def _main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("steps", nargs="*", metavar="STEP", help="steps to run (default: the full check)")
    parser.add_argument("--list", action="store_true", help="print every step and its commands")
    parser.add_argument("--files", nargs="+", metavar="FILE",
                        help="with `quick`: the changed files, instead of the git diff against main")
    parser.add_argument("--only", nargs="+", action="extend", metavar="CRATE",
                        help="with `browser`: run only the named presentation crate launch(es) "
                        "(partial subset; skips full page build and drag smoke)")
    parser.add_argument("--keep-going", action="store_true",
                        help="in `browser`: continue independent browser launches after a failure "
                        "and exit nonzero if any launch failed")
    parser.add_argument("--update-native-baseline", action="store_true",
                        help="after the native tests pass, record the executed test names in "
                        "scripts/check-native-tests-baseline.json (for tests removed or renamed on purpose)")
    parser.add_argument("--receipt", metavar="PATH",
                        help="write an opt-in JSON verification receipt to PATH (and a reviewer summary "
                        "to PATH.md) from this invocation's captured output")
    parser.add_argument("--base-receipt", metavar="PATH",
                        help="with --receipt: compare against a previously written receipt, marked "
                        "reused with its original run/source identity")
    args = parser.parse_args(argv)
    if args.only is not None:
        if args.list:
            parser.error("--only cannot be combined with --list")
        if args.steps != ["browser"]:
            parser.error("--only applies to `browser` alone (subset runs cannot certify full or other steps)")
        try:
            only_crates = parse_only_crates(args.only)
        except ValueError as error:
            parser.error(str(error))
    else:
        only_crates = None
    if args.keep_going:
        if args.list:
            parser.error("--keep-going cannot be combined with --list")
        effective_steps = args.steps or list(DEFAULT)
        if effective_steps != ["quick"] and "browser" not in effective_steps:
            parser.error("--keep-going applies to the `browser` step (or `quick`, or a full check)")
    if args.update_native_baseline and args.steps and args.steps != ["quick"] and "test" not in args.steps:
        parser.error("--update-native-baseline applies to the `test` step (or `quick`, or a full check)")
    if args.base_receipt and not args.receipt:
        parser.error("--base-receipt requires --receipt")
    if args.list and args.receipt:
        parser.error("--receipt requires execution, not --list")
    if args.files and args.steps != ["quick"]:
        parser.error("--files applies to `quick` only")
    unknown = [name for name in args.steps if name not in STEPS]
    if args.steps != ["quick"] and unknown:
        parser.error(f"unknown step(s): {', '.join(unknown)}; choose from {', '.join(STEPS)}")
    if not args.list:
        browser_tools._inherited_build_lock()
    receipt = None
    if args.receipt:
        receipt = verification_receipt.Collector(
            path=Path(args.receipt),
            source=verification_receipt.source_identity(ROOT),
            baseline_path=check_native_tests.BASELINE,
            baseline_root=ROOT,
            base_path=Path(args.base_receipt) if args.base_receipt else None,
        )
    global _RECEIPT
    _RECEIPT = receipt
    if args.steps == ["quick"]:
        return quick(args.files, args.update_native_baseline, keep_going=args.keep_going)
    if args.list:
        for name, (included, commands) in STEPS.items():
            print(f"{name}{'*' if included else ''}")
            for command in commands:
                print("  " + shlex.join(command))
        print("quick  (changed files only; see `check.py quick`)")
        return 0
    return run(
        args.steps or list(DEFAULT),
        args.update_native_baseline,
        only=only_crates,
        keep_going=args.keep_going,
    )


def main(argv: list[str] | None = None) -> int:
    global _RECEIPT
    code = 1
    try:
        code = _main(argv)
        return code
    except KeyboardInterrupt:
        code = 130
        raise
    except SystemExit as error:
        code = error.code if isinstance(error.code, int) else 1
        raise
    finally:
        receipt, _RECEIPT = _RECEIPT, None
        if receipt is not None:
            names = tuple(dict.fromkeys(step for step, _ in receipt.plan))
            receipt.finish(code, partial=names != DEFAULT, source_after=verification_receipt.source_identity(ROOT))


if __name__ == "__main__":
    raise SystemExit(main())
