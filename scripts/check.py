#!/usr/bin/env python3
"""Run BoardStudio's named check steps in order.

  check.py                run the full check (the steps marked with * below)
  check.py STEP...        run only the named steps
  check.py browser --only CRATE...   run only the named presentation crate browser launch(es)
                          (partial subset; skips full page build and drag smoke)
  check.py browser --keep-going      continue independent browser launches after a failure
  check.py quick          the routine loop: only the checks that cover your changed files
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
              run up to BOARDSTUDIO_CHECK_JOBS (default 4) at once, without debug info;
              the independent native test commands also run together
  security    dependency audit of every Rust lockfile (separate CI workflow)
  deny        cargo-deny license, ban and source policy from deny.toml (separate CI workflow)
  precommit   typecheck the page
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
import argparse
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import browser_tools  # noqa: E402  (needs the scripts directory on sys.path)
import check_native_tests  # noqa: E402  (needs the scripts directory on sys.path)
import verification_receipt  # noqa: E402  (needs the scripts directory on sys.path)

PY = "python3"
# One workspace invocation compiles Core once for every crate's tests. Footprints runs
# separately: its serde_json `preserve_order` dev-feature must not leak into Core's tests.
CARGO_TESTS: list[list[str]] = [
    ["cargo", "test", "--workspace", "--locked", "--exclude", "boardstudio-footprints"],
    ["cargo", "test", "-p", "boardstudio-footprints", "--locked"],
]
TOOLING_TESTS = (
    "cad/scripts/test-cad-build.py", "cad/scripts/test-measure-cad.py",
    "cad/scripts/test-measure-cad-worker.cjs",
    "scripts/test-build-web.py", "scripts/test-serve-web.py",
    "scripts/test-dev-web.py", "scripts/test-run-wasm-tests.py", "scripts/test-check-wasm-tests.py",
    "scripts/test-stage-ergogen-models.py", "scripts/test-check.py", "scripts/test-check-quick.py", "scripts/test-catalogue-tools.py", "scripts/test-gitnexus-refresh.py",
    "scripts/test-import-kicad-parts.py", "scripts/test-gitnexus-stop-hook.py",
    "scripts/test-model-lookup.cjs", "scripts/test-ergogen-parity.cjs", "scripts/test-verification-receipt.py",
    "scripts/test-check-native-tests.py", "scripts/test-mutation-check.py", "scripts/test-browser-tools.py",
<<<<<<< HEAD
    "scripts/test-chrome-for-testing.py", "scripts/test-bump-chrome-for-testing.py",
=======
    "scripts/test-install-drag-chrome.py", "scripts/test-graphite-preflight.py", "tools/browser-harness/test-response-probe-cleanup.py",
>>>>>>> c1dbd3019 (tooling: default the drag smoke to the Rust harness, retire the Node path)
)
WASM_PAGE = ["--no-default-features", "--features", "page"]
ISOLATED = ("presentation::case_workspace::",)
DRAG_SMOKE = [PY, "scripts/run-drag-browser.py"]
LAYOUT_TEST_CSS: list[str] = [
    "dx", "build", "--web", "--no-default-features", "--features", "page", "--cargo-args=--locked",
]

Command = list[str]
# The active opt-in receipt collector for `--receipt`; set by `main` and read by `run_plan`.
_RECEIPT: verification_receipt.Collector | None = None


def typecheck() -> list[Command]:
    return [["cargo", "check", "-p", "boardstudio-web", "--locked", "--target", "wasm32-unknown-unknown",
             *WASM_PAGE, "--bin", "boardstudio-web"]]


def browser() -> list[Command]:
    wasm_pack = ["wasm-pack", "test", "--headless", "--chrome", "web", "--locked", *WASM_PAGE]
    return [
        DRAG_SMOKE,
        [PY, "cad/scripts/test-cad-capture-controls.py"],
        [PY, "cad/scripts/test-cold-responses.py"],
        [PY, "scripts/check-wasm-tests.py"],
        [PY, "cad/scripts/test-cad-provider.py"],
        [PY, "cad/scripts/test-cad-browser.py"],
        ["wasm-pack", "test", "--headless", "--chrome", "web/crates/host", "--locked", "--features", "page",
         "--lib"],
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


def launch_crate(command: Command) -> str | None:
    """The `web/crates/<name>` target of a presentation-crate `wasm-pack test` launch."""
    if command[:2] != ["wasm-pack", "test"]:
        return None
    return next((part.removeprefix("web/crates/") for part in command if part.startswith("web/crates/")), None)


WEB_WASM_PACKAGES: tuple[str, ...] = (
    "boardstudio-web",
    *dict.fromkeys(f"boardstudio-web-{crate}" for cmd in browser() if (crate := launch_crate(cmd))),
)

# `--no-deps` keeps Clippy off the vendored cgmath fork; its lints are audited separately.
# The browser harness lives outside the workspace with its own lockfile, so it
# gets its own manifest-scoped format and lint commands.
HARNESS_MANIFEST = "tools/browser-harness/Cargo.toml"
LINT: list[list[str]] = [
    ["cargo", "fmt", "--all", "--check"],
    ["cargo", "clippy", "--workspace", "--locked", "--all-targets", "--no-deps", "--", "-D", "warnings"],
    # Native Clippy compiles non-WASM stubs for browser-gated modules and tests, so all
    # web workspace crates are also linted under an explicit WASM target.
    ["cargo", "clippy", *[arg for pkg in WEB_WASM_PACKAGES for arg in ("--package", pkg)],
     "--target", "wasm32-unknown-unknown", "--all-targets", "--locked", "--no-deps", "--", "-D", "warnings"],
    # The browser harness lives outside the workspace with its own lockfile, so
    # it gets its own manifest-scoped format and lint commands.
    ["cargo", "fmt", "--manifest-path", HARNESS_MANIFEST, "--", "--check"],
<<<<<<< HEAD
    ["cargo", "clippy", "--manifest-path", HARNESS_MANIFEST, "--all-targets", "--locked", "--", "-D", "warnings"],
=======
    ["cargo", "clippy", "--manifest-path", HARNESS_MANIFEST, "--all-targets", "--locked", "--no-deps", "--", "-D", "warnings"],
>>>>>>> c1dbd3019 (tooling: default the drag smoke to the Rust harness, retire the Node path)
]

STEPS: dict[str, tuple[bool, list[Command]]] = {
    "repo": (True, [
        [PY, "-B", "scripts/test-check-doc-links.py"],
        [PY, "scripts/check-doc-links.py"],
        [PY, "-B", "scripts/test-check-core-tests.py"],
        [PY, "scripts/check-core-tests.py"],
        [PY, "-B", "scripts/test-check-adr-index.py"],
        [PY, "scripts/check-adr-index.py"],
    ]),
    "tooling": (True, [["node", "--version"], *[
        ["node", script] if script.endswith(".cjs") else [PY, "-B", script]
        for script in TOOLING_TESTS]]),
    "lint": (True, LINT),
    # The routine build skips wasm-opt and the optimized Dioxus build; `release` keeps both.
    "build": (True, [[PY, "scripts/build-web.py", "--fast"]]),
    "release": (False, [[PY, "scripts/build-web.py"]]),
    "test": (True, [*CARGO_TESTS, [PY, "cad/scripts/test-cad.py"]]),
    "typecheck": (False, typecheck()),
    "browser": (True, browser()),
    "drag-smoke": (False, [DRAG_SMOKE]),
    "security": (False, [[PY, "-B", "scripts/security-audit.test.py"], [PY, "scripts/security-audit.py"]]),
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
    """One `wasm-pack test` launch of a presentation crate: independent of every other launch."""
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
    return name == "browser" or name.startswith("browser (")


def check_jobs() -> int:
    try:
        return max(1, int(os.environ.get("BOARDSTUDIO_CHECK_JOBS", "4")))
    except ValueError:
        return 4


def launch_order(command: Command) -> int:
    return 0 if launch_crate(command) in HEAVY_CRATES else 1


def is_cargo_test(command: Command) -> bool:
    return command[:2] == ["cargo", "test"]


def is_wasm_pack_test(command: Command) -> bool:
    return command[:2] == ["wasm-pack", "test"]


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


def run_serial(command: Command, environment: dict[str, str], cwd: Path = ROOT,
               captured: list[tuple[Command, str]] | None = None,
               outputs: list[tuple[Command, str]] | None = None) -> tuple[float, int, str, str]:
    print("$ " + shlex.join(command), flush=True)
    lock_fd = browser_tools._inherited_build_lock()
    if not any(part in command for part in ("scripts/run-drag-browser.py", "cad/scripts/test-cad-capture-controls.py", "cad/scripts/test-cold-responses.py", "scripts/build-web.py",
               "scripts/run-wasm-tests.py", "cad/scripts/test-cad.py", "scripts/test-import-kicad-parts.py")):
        environment = environment.copy()
        environment.pop("BOARDSTUDIO_BUILD_LOCK_FD", None)
    lock_args = {"pass_fds": (lock_fd,)} if lock_fd is not None else {}
    started = time.monotonic()
    started_at = datetime.now(timezone.utc).isoformat()
    print(f"check: start {started_at} $ {shlex.join(command)}", flush=True)
    def run(*args, **kwargs):
        try:
            return subprocess.run(*args, **kwargs)
        except OSError as error:
            message = f"startup failed: {error}\n"
            print(message, end="", file=sys.stderr, flush=True)
            return subprocess.CompletedProcess(command, 127, stdout=message)
    if captured is not None and is_cargo_test(command):
        # The native test-name guard reads this output; it is printed whole once the command ends.
        done = run(command, cwd=cwd, env=environment, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, **lock_args)
        text = output_text(done.stdout)
        print(text, end="" if text.endswith("\n") or not text else "\n", flush=True)
        captured.append((command, text))
        if outputs is not None:
            outputs.append((command, text))
        code = done.returncode
    elif outputs is not None and is_wasm_pack_test(command):
        done = run(command, cwd=cwd, env=environment, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, **lock_args)
        text = output_text(done.stdout)
        print(text, end="" if text.endswith("\n") or not text else "\n", flush=True)
        outputs.append((command, text))
        code = done.returncode
    else:
        code = run(command, cwd=cwd, env=environment, **lock_args).returncode
    elapsed = time.monotonic() - started
    ended_at = datetime.now(timezone.utc).isoformat()
    print(f"check: end {ended_at} exit {code} ({elapsed:.1f}s)", flush=True)
    return elapsed, code, started_at, ended_at


def run_parallel(batch: list[Command], environment: dict[str, str],
                 captured: list[tuple[Command, str]] | None = None,
                 outputs: list[tuple[Command, str]] | None = None,
                 receipt: verification_receipt.Collector | None = None,
                 step: str = "") -> tuple[float, list[Command]]:
    """Runs independent launches together, printing each one's output whole and in order."""
    lock_fd = browser_tools._inherited_build_lock()

    def launch(command: Command) -> tuple[int, str, float, str, str]:
        started = time.monotonic()
        started_at = datetime.now(timezone.utc).isoformat()
        print(f"check: start {started_at} $ {shlex.join(command)}", flush=True)
        child_env = environment.copy()
        if "cad/scripts/test-cad.py" not in command:
            child_env.pop("BOARDSTUDIO_BUILD_LOCK_FD", None)
        try:
            done = subprocess.run(command, cwd=ROOT, env=child_env, stdout=subprocess.PIPE,
                                  stderr=subprocess.STDOUT, text=True,
                                  **({"pass_fds": (lock_fd,)} if lock_fd is not None else {}))
            code, output = done.returncode, done.stdout
        except OSError as error:
            code, output = 127, f"startup failed: {error}\n"
        ended_at = datetime.now(timezone.utc).isoformat()
        print(f"check: end {ended_at} exit {code} $ {shlex.join(command)}", flush=True)
        return code, output, time.monotonic() - started, started_at, ended_at

    ordered = sorted(batch, key=launch_order)
    started = time.monotonic()
    failed: list[Command] = []
    with ThreadPoolExecutor(max_workers=check_jobs()) as pool:
        futures = [(command, pool.submit(launch, command)) for command in ordered]
        for command, future in futures:
            code, output, elapsed, started_at, ended_at = future.result()
            text = output_text(output)
            print("$ " + shlex.join(command), flush=True)
            print(text, end="" if text.endswith("\n") or not text else "\n", flush=True)
            print(f"  ({elapsed:.1f}s)", flush=True)
            if captured is not None and is_cargo_test(command):
                captured.append((command, text))
            if outputs is not None:
                outputs.append((command, text))
            if receipt is not None:
                receipt.observe(step, command, code, elapsed, text, started_at=started_at, ended_at=ended_at)
            if code:
                failed.append(command)
    return time.monotonic() - started, failed


def _report_launch_failure(command: Command, text: str) -> None:
    print("check: launch failed: " + shlex.join(command), file=sys.stderr)
    if summary := format_wasm_failure_summary(text):
        print("check: " + summary, file=sys.stderr)


def run_commands(name: str, commands: list[Command], environment: dict[str, str], cwd: Path = ROOT,
                 captured: list[tuple[Command, str]] | None = None,
                 *, keep_going: bool = False,
                 receipt: verification_receipt.Collector | None = None) -> tuple[float, bool]:
    """Runs a step's commands; consecutive crate launches in `browser`, and the independent
    workspaces of `test`, run together. `captured` collects each `cargo test` command's output."""
    if name == "test" and check_jobs() > 1 and len(commands) > 1:
        elapsed, failed = run_parallel(commands, environment, captured, receipt=receipt, step=name)
        for command in failed:
            print("check: command failed: " + shlex.join(command), file=sys.stderr)
        return elapsed, not failed
    browser_step = is_browser_step(name)
    total = 0.0
    any_failed = False
    index = 0
    while index < len(commands):
        command = commands[index]
        if browser_step and check_jobs() > 1 and is_crate_launch(command):
            end = index
            while end < len(commands) and is_crate_launch(commands[end]):
                end += 1
            batch_outputs: list[tuple[Command, str]] = []
            elapsed, failed = run_parallel(commands[index:end], environment, outputs=batch_outputs,
                                           receipt=receipt, step=name)
            total += elapsed
            if failed:
                output_by_cmd = {tuple(cmd): text for cmd, text in batch_outputs}
                for failed_cmd in failed:
                    _report_launch_failure(failed_cmd, output_by_cmd.get(tuple(failed_cmd), ""))
                if not keep_going:
                    return total, False
                any_failed = True
            index = end
            continue
        serial_outputs: list[tuple[Command, str]] = []
        if receipt is not None:
            receipt.prepare(command)
        elapsed, code, started_at, ended_at = run_serial(command, environment, cwd, captured, serial_outputs)
        total += elapsed
        if receipt is not None:
            receipt.observe(name, command, code, elapsed, serial_outputs[-1][1] if serial_outputs else "",
                            started_at=started_at, ended_at=ended_at)
        if code:
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
        if "layout" in only_crates:
            plan.append(("layout-test-css", [LAYOUT_TEST_CSS]))
        plan.append((f"browser (only: {', '.join(only_crates)})",
                     select_browser_crate_launches(only_crates)))
        return (run_plan(plan, update_native_baseline, keep_going=True)
                if keep_going else run_plan(plan, update_native_baseline))
    # The real-page smoke needs a current packaged page, including for `check.py browser` alone.
    if _RECEIPT is not None:
        _RECEIPT.plan_commands([(name, STEPS[name][1]) for name in names])
    has_drag_smoke = any("scripts/run-drag-browser.py" in command for command in STEPS["browser"][1])
    smoke_indices = [index for index, step in enumerate(names)
                     if step == "drag-smoke" or (step == "browser" and has_drag_smoke)]
    if smoke_indices:
        # Fail before the page build, which is minutes of work the smoke cannot use without these.
        # The opt-in Rust driver needs only Chrome; the default Node runner also needs its client.
        try:
<<<<<<< HEAD
            if os.environ.get("BOARDSTUDIO_DRAG_DRIVER", "node") == "rust":
                browser_tools.locate_chrome()
            else:
                browser_tools.locate()
=======
            browser_tools.validate_drag_driver()
            browser_tools.locate_chrome()
>>>>>>> c1dbd3019 (tooling: default the drag smoke to the Rust harness, retire the Node path)
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


def run_plan(plan: list[tuple[str, list[Command]]], update_native_baseline: bool = False,
             *, keep_going: bool = False, partial: bool = False) -> int:
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
            receipt.finish(code, partial=focused)
        _RECEIPT = None
        return code

    for name, commands in plan:
        print(f"== {name}", flush=True)
        environment = os.environ.copy()
        if name == "test":
            # An editor's AppImage directory redirects system KiCad's library lookup.
            environment.pop("APPDIR", None)
        if is_browser_step(name):
            # Match the page runner's bounded budget for direct wasm-pack crate batches.
            environment.setdefault("WASM_BINDGEN_TEST_TIMEOUT", "120")
            # Test wasm without DWARF is about a seventh of the size, which is most of a launch.
            environment.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
        captured: list[tuple[Command, str]] | None = [] if name == "test" else None
        cwd = ROOT / "web" if name == "layout-test-css" else ROOT
        elapsed, passed = run_commands(
            name, commands, environment, cwd, captured,
            keep_going=keep_going and is_browser_step(name),
            receipt=receipt,
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
            return finish(1)
    if update_native_baseline and not baseline_written:
        print("check: --update-native-baseline: no native `cargo test` ran, so the baseline is unchanged",
              file=sys.stderr)
        return finish(1)
    print("check: step times: " + ", ".join(f"{name} {seconds:.1f}s" for name, seconds in totals.items())
          + f", total {sum(totals.values()):.1f}s")
    if partial:
        print("check: partial steps passed: " + ", ".join(dict.fromkeys(names))
              + " (subset only; full browser gate not run)")
    else:
        print("check: all steps passed: " + ", ".join(dict.fromkeys(names)))
    return finish(0)


def quick_plan(selection: "check_quick.Selection", packages: "dict[str, check_quick.Package]") -> list[tuple[str, list[Command]]]:
    """Run affected checks and required test assets, without the full frontend build."""
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
            # Quick deliberately skips checks requiring a packaged site.
            commands = [command for command in browser() if not any(name in command for name in
                        ("scripts/run-drag-browser.py", "cad/scripts/test-cold-responses.py"))]
        else:
            commands = select_browser_crate_launches(crates, browser())
            if selection.page_files:
                commands.append([PY, "scripts/run-wasm-tests.py", "--files", *selection.page_files,
                                 "--depth", "1", *[f"--isolate={name}" for name in ISOLATED],
                                 "--result-json", "web/target/test-results/browser-quick.json"])
        if commands:
            # No layout-test-css: quick excludes the packaged-site drag smoke, and no quick
            # browser test reads the dx bundle (dx build touches no tracked files).
            plan.append(("browser", commands))
    if selection.cad:
        plan.append(("test", [[PY, "cad/scripts/test-cad.py"]]))
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
            receipt.finish(code, partial=names != DEFAULT)


if __name__ == "__main__":
    raise SystemExit(main())
