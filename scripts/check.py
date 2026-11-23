#!/usr/bin/env python3
"""Run BoardStudio checks. Use --list to see the default and optional steps."""

from __future__ import annotations

import argparse
from collections.abc import Iterable, Sequence
from concurrent.futures import ThreadPoolExecutor, wait
from contextlib import ExitStack, contextmanager
import importlib
import json
import os
from pathlib import Path
import shlex
import signal
import subprocess
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import browser_tools
import cargo_json
import check_native_tests
from tailwind_tools import IN_OVERLAP_ENV, TEST_CSS_READY, check_jobs, test_css_command
import wasm_bindgen_tools


PY = "python3"
OVERLAP_MIN_CPUS = 8
OVERLAP_STEPS = frozenset({"build", "lint", "test"})
OVERLAP_OPTIONAL_STEPS = frozenset({"repo", "tooling"})
# One workspace invocation compiles Core once for every crate's tests, including footprints.
CARGO_TESTS: list[list[str]] = [
    ["cargo", "test", "--workspace", "--locked", "--lib", "--bins", "--tests"],
]
# Longest first, so the parallel batch ends with short scripts; the serial scripts run after it.
TOOLING_TESTS = (
    "cad/scripts/test-cad-build.py", "scripts/test-ci-workflow.py",
    "scripts/test-check-quick.py", "scripts/test-check.py", "scripts/test-browser-tools.py",
    "scripts/test-mutation-check.py", "scripts/test-run-wasm-tests.py",
    "scripts/test-stage-ergogen-models.py", "scripts/test-dev-web.py",
    "scripts/test-build-web.py", "scripts/test-build-patched-wasm-bindgen.py",
    "scripts/test-bump-chrome-for-testing.py", "scripts/test-chrome-for-testing.py",
    "scripts/test-check-native-tests.py", "scripts/test-verify-no-node.py",
    "scripts/test-catalogue-tools.py", "scripts/test-run-drag-browser.py",
    "scripts/test-serve-web.py",
)
KICAD_PARTS_TEST = [PY, "scripts/test-import-kicad-parts.py"]
CAD_TEST = [PY, "cad/scripts/test-cad.py"]
WASM_PAGE = ["--no-default-features", "--features", "page"]
# Keep browser test builds on the same profile as the checked page.
WASM_CRATE_BUILD = ["cargo", "test", "--no-run", "--lib", "--target", "wasm32-unknown-unknown",
                    "--profile", "wasm-check", "--locked", "--message-format=json"]
DRAG_SMOKE = [PY, "scripts/run-drag-browser.py"]
CAD_BROWSER = [PY, "cad/scripts/test-cad-browser.py"]
PROVIDER_CHECKS = [
    [PY, "cad/scripts/test-cad-provider.py"],
    [PY, "-B", "scripts/ergogen-parity/test_generate.py", "--source"],
    [PY, "scripts/model-lookup-pilot.py", "--gate"],
]
# Two depth-one pages. Each further page costs ~15s of loading beside the crate launches, more than
# splitting the larger group with `--max-group-tests` saves (measured 2026-11-19).
PAGE_SUITE = [PY, "scripts/run-wasm-tests.py", "--all", "--depth", "1",
              "--result-json", "web/target/test-results/browser.json"]
PAGE_TEST_BUILD = [PY, "scripts/run-wasm-tests.py", "--build-only"]
PAGE_PACKAGE = "boardstudio-web"
BUILD_LOCK_SCRIPTS = (
    "scripts/build-web.py", "scripts/run-wasm-tests.py", CAD_TEST[1], KICAD_PARTS_TEST[1],
    "scripts/check.py",
)


Command = list[str]


def typecheck() -> list[Command]:
    return [["cargo", "check", "-p", "boardstudio-web", "--locked", "--target", "wasm32-unknown-unknown",
             *WASM_PAGE, "--bin", "boardstudio-web"]]


def browser() -> list[Command]:
    return [
        DRAG_SMOKE,
        CAD_BROWSER,
        *PROVIDER_CHECKS,
        *with_crate_build([
            crate_launch("host"),
            crate_launch("worker-host"),
        ]),
        PAGE_SUITE,
    ]


def crate_launch(crate: str, *filters: str) -> Command:
    # The plan uses the crate directory as a stable identity. Cargo supplies the binary at runtime.
    return [wasm_bindgen_tools.TEST_RUNNER, f"web/crates/{crate}", *filters]


def launch_crate(command: Command) -> str | None:
    """The presentation crate targeted by a browser launch."""
    if len(command) > 1 and command[0] == wasm_bindgen_tools.TEST_RUNNER:
        return Path(command[1]).name
    return None


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


WEB_WASM_LINT_PACKAGES = (
    "boardstudio-web", "boardstudio-web-host", "boardstudio-web-worker-host",
    "boardstudio-web-runtime", "boardstudio-web-ui-shared", "boardstudio-web-keycaps",
    "boardstudio-web-library", "boardstudio-web-keymap", "boardstudio-web-case",
    "boardstudio-web-parts", "boardstudio-web-pcb", "boardstudio-web-layout",
    "boardstudio-web-layout-objects",
)

BROWSER_CRATE_BUILD = next(command for command in browser() if is_crate_build(command))
BROWSER_PREBUILDS = [BROWSER_CRATE_BUILD, PAGE_TEST_BUILD]


# `--no-deps` keeps Clippy off the vendored cgmath fork; its lints are audited separately.
LINT: list[list[str]] = [
    ["cargo", "fmt", "--all", "--check"],
    ["cargo", "clippy", "--profile", "lint", "--workspace", "--locked", "--all-targets", "--no-deps", "--", "-D", "warnings"],
    # Native Clippy compiles non-WASM stubs for browser-gated modules and tests, so all
    # web workspace crates are also linted under an explicit WASM target.
    ["cargo", "clippy", "--profile", "lint", *[arg for pkg in WEB_WASM_LINT_PACKAGES for arg in ("--package", pkg)],
     "--target", "wasm32-unknown-unknown", "--all-targets", "--locked", "--no-deps", "--", "-D", "warnings"],
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
                       [PY, "-B", "cad/scripts/test-provider-pages.py", "--hermetic"],
                       [PY, "-B", "cad/scripts/test-cad-provider-gate.py"], *[
        [PY, "-B", script]
        for script in TOOLING_TESTS]]),
    # The routine build skips wasm-opt and the optimized Dioxus build; `release` keeps both.
    "build": (True, [[PY, "scripts/build-web.py", "--fast"], *BROWSER_PREBUILDS]),
    "lint": (True, LINT),
    "release": (False, [[PY, "scripts/build-web.py"]]),
    "test": (True, [*CARGO_TESTS, CAD_TEST, KICAD_PARTS_TEST]),
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


def browser_crates() -> tuple[str, ...]:
    return tuple(crate for command in browser() if (crate := launch_crate(command)))


def select_browser_crate_launches(crates: Iterable[str]) -> list[Command]:
    chosen = set(crates)
    return [command for command in browser() if launch_crate(command) in chosen]


def parse_only_crates(raw: Sequence[str]) -> list[str]:
    known = browser_crates()
    chosen: list[str] = []
    for item in raw:
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


def crate_test_artifacts(output: str, cwd: Path, crates: Iterable[str]) -> dict[str, Path]:
    manifests = {cwd / "web/crates" / crate / "Cargo.toml": crate for crate in crates}
    artifacts: dict[str, Path] = {}
    for message in cargo_json.messages(output, sys.stdout, flush=True):
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



def child_options(command, environment):
    fd = browser_tools._inherited_build_lock()
    env = environment.copy()
    if not any(script in command for script in BUILD_LOCK_SCRIPTS):
        env.pop("BOARDSTUDIO_BUILD_LOCK_FD", None)
    return {"env": env, "pass_fds": () if fd is None else (fd,), "text": True}


def run_serial(command, environment, cwd=ROOT, captured=None, outputs=None):
    print("$ " + shlex.join(command), flush=True)
    started = time.monotonic()
    capture = outputs is not None or captured is not None
    try:
        result = subprocess.run(command, cwd=cwd, **child_options(command, environment),
                                stdout=subprocess.PIPE if capture else None,
                                stderr=subprocess.STDOUT if captured is not None else None)
    except OSError as error:
        print(f"check: {error}", file=sys.stderr)
        return time.monotonic() - started, 127
    if captured is not None:
        captured.append((command, result.stdout))
        print(result.stdout, end="", flush=True)
    if outputs is not None:
        outputs.append((command, result.stdout))
    return time.monotonic() - started, result.returncode


def run_parallel(batch, environment, *, captured=None):
    """Run independent command lanes, each in order, and clean up on interruption."""
    started = time.monotonic()
    active = set()
    lock = threading.Lock()
    cancelled = threading.Event()
    own_groups = IN_OVERLAP_ENV not in environment

    def run_lane(lane):
        for command, cwd, env in lane:
            print("$ " + shlex.join(command), flush=True)
            command_started = time.monotonic()
            with lock:
                if cancelled.is_set():
                    return False
                try:
                    process = subprocess.Popen(command, cwd=cwd, **child_options(command, env),
                                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                               start_new_session=own_groups)
                except OSError as error:
                    print(f"check: {error}", file=sys.stderr)
                    return False
                active.add(process)
            with process.stdout:
                output = process.stdout.read()
            # Keep the process group ID reserved until both the process and output are done.
            os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOWAIT)
            with lock:
                process.wait()
                active.discard(process)
                if captured is not None:
                    captured.append((command, output))
            elapsed = time.monotonic() - command_started
            if output:
                print(output, end="" if output.endswith("\n") else "\n", flush=True)
            print(f"check: {elapsed:.2f}s {shlex.join(command)}", flush=True)
            if process.returncode:
                print("check: command failed: " + shlex.join(command), file=sys.stderr)
                return False
        return True

    def signal_children(signum):
        with lock:
            for process in active:
                try:
                    if own_groups:
                        os.killpg(process.pid, signum)
                    else:
                        process.send_signal(signum)
                except ProcessLookupError:
                    pass

    with ThreadPoolExecutor(max_workers=check_jobs()) as pool:
        futures = [pool.submit(run_lane, lane) for lane in batch]
        try:
            results = [future.result() for future in futures]
        except BaseException:
            cancelled.set()
            signal_children(signal.SIGTERM)
            if wait(futures, timeout=2).not_done:
                signal_children(signal.SIGKILL)
            raise
    return time.monotonic() - started, all(results)


def run_native_tests(environment, cwd, state):
    captured = []
    with tempfile.TemporaryDirectory(prefix="check-native-") as temporary:
        with overlap_jobserver(Path(temporary), os.cpu_count() or 1) as flags:
            env = dict(environment, MAKEFLAGS=flags)
            kicad_env = env.copy()
            kicad_env.pop("APPDIR", None)  # KiCad must use its system libraries.
            batch = [
                [(command, cwd, env) for command in CARGO_TESTS],
                [(CAD_TEST, cwd, env), (KICAD_PARTS_TEST, cwd, kicad_env)],
            ]
            elapsed, passed = run_parallel(batch, env, captured=captured)
    if passed:
        state["native"] = [output for command, output in captured if command in CARGO_TESTS]
    return elapsed, passed


def run_commands(name, commands, environment, cwd=ROOT, *, state=None, keep_going=False):
    started = time.monotonic()
    if state is None:
        state = {}
    if name == "tooling" and check_jobs() > 1:
        serial = [cmd for cmd in commands if "scripts/test-serve-web.py" in cmd]
        batch = [[(cmd, cwd, environment)] for cmd in commands if cmd not in serial]
        _, passed = run_parallel(batch, environment)
        serial_passed = [run_serial(cmd, environment, cwd)[1] == 0 for cmd in serial]
        return time.monotonic() - started, passed and all(serial_passed)
    if (name == "build" and check_jobs() > 1 and commands == STEPS["build"][1]):
        if not state.get("css") and environment.get(TEST_CSS_READY) != "1":
            css = test_css_command()
            _, code = run_serial(css, environment, cwd / "web")
            if code:
                print("check: command failed: " + shlex.join(css), file=sys.stderr)
                return time.monotonic() - started, False
            state["css"] = True
            environment[TEST_CSS_READY] = "1"
        web_env = dict(environment, **{TEST_CSS_READY: "1", IN_OVERLAP_ENV: "1"})
        browser_env = dict(environment, **{TEST_CSS_READY: "1"})
        # The provider/page build and browser prebuilds use disjoint Cargo targets.
        with ThreadPoolExecutor(max_workers=2) as pool:
            web = pool.submit(run_commands, name, commands[:1], web_env, cwd, state=state)
            browser = pool.submit(run_commands, name, commands[1:], browser_env, cwd, state=state)
            web_passed, browser_passed = web.result()[1], browser.result()[1]
            passed = web_passed and browser_passed
        if passed:
            environment[TEST_CSS_READY] = "1"
        return time.monotonic() - started, passed
    if is_browser_step(name):
        return run_browser(commands, environment, cwd, state, keep_going)
    if name == "test" and commands == STEPS["test"][1] and check_jobs() > 1:
        return run_native_tests(environment, cwd, state)

    passed = True
    for command in commands:
        output = []
        capture = None
        if is_crate_build(command):
            code = int(not build_crate_tests(command, environment, cwd, state))
        elif command == PAGE_TEST_BUILD:
            _, code = run_serial(command, environment, cwd, outputs=output)
            if not code:
                state["page"] = output[0][1].strip()
        else:
            if command[:2] == ["cargo", "test"] and name == "test":
                capture = []
            _, code = run_serial(command, environment, cwd, captured=capture)
            if capture and not code:
                state.setdefault("native", []).extend(text for _, text in capture)
            if not code and ("scripts/build-web.py" in command or name == "test-css"):
                state["css"] = True
                environment[TEST_CSS_READY] = "1"
        if code:
            passed = False
            print("check: command failed: " + shlex.join(command), file=sys.stderr)
            if not keep_going:
                break
    return time.monotonic() - started, passed


def build_crate_tests(command, environment, cwd, state):
    if command == state.get("crate_build") and all(path.is_file() for path in state["crates"].values()):
        return True
    output = []
    env = wasm_bindgen_tools.test_build_env(environment, cwd)
    _, code = run_serial(command, env, cwd, outputs=output)
    crates = [value.removeprefix("boardstudio-web-") for flag, value in zip(command, command[1:])
              if flag == "-p"]
    if output:
        try:
            artifacts = crate_test_artifacts(output[0][1], cwd, [] if code else crates)
        except (ValueError, RuntimeError) as error:
            print(f"check: {error}", file=sys.stderr)
            return False
        if not code:
            state.update(crate_build=command, crates=artifacts)
            return True
    return False


def run_browser(commands, environment, cwd, state, keep_going):
    started = time.monotonic()
    failed = False
    available = False
    lanes, support_checks = [], []
    with ExitStack() as configs:
        for command in commands:
            if is_crate_build(command):
                available = build_crate_tests(command, environment, cwd, state)
                failed |= not available
                if failed and not keep_going:
                    return time.monotonic() - started, False
                continue
            env = environment
            directory = cwd
            if crate := launch_crate(command):
                if not available:
                    continue
                directory = cwd / command[1]
                runner = importlib.import_module("run-wasm-tests")
                env = configs.enter_context(runner.desktop_webdriver_config(env, directory))
                env.update(WASM_BINDGEN_TEST_ONLY_WEB="1", RAYON_NUM_THREADS="1")
                command = [wasm_bindgen_tools.TEST_RUNNER, str(state["crates"][crate]), *command[2:]]
            elif "scripts/run-wasm-tests.py" in command:
                if command == PAGE_SUITE and state.get("page"):
                    command = [*command, "--test-binary", state["page"]]
                spare = check_jobs() - len(lanes)
                env = dict(env, BOARDSTUDIO_CHECK_JOBS=str(max(1, min(2, spare))))
            if command == DRAG_SMOKE and not keep_going and check_jobs() > 2:
                lanes += [[([*command, "--suite", suite], directory, env)] for suite in ("drag", "collision")]
            elif not keep_going and (command in (CAD_BROWSER, *PROVIDER_CHECKS) or (crate and support_checks)):
                if not support_checks:
                    lanes.append(support_checks)
                support_checks.append((command, directory, env))
            else:
                lanes.append([(command, directory, env)])
        if check_jobs() > 1:
            _, passed = run_parallel(lanes, environment)
            failed |= not passed
        else:
            for command, directory, env in (invocation for lane in lanes for invocation in lane):
                _, code = run_serial(command, env, directory)
                if code and not keep_going:
                    return time.monotonic() - started, False
                failed |= bool(code)
    return time.monotonic() - started, not failed


def overlap_span(plan):
    if os.environ.get("CI") or check_jobs() <= 1 or (os.cpu_count() or 1) < OVERLAP_MIN_CPUS:
        return None
    allowed = OVERLAP_STEPS | OVERLAP_OPTIONAL_STEPS
    for start in range(len(plan) - len(OVERLAP_STEPS) + 1):
        names = set()
        end = start
        for name, commands in plan[start:start + len(allowed)]:
            if name in names or name not in allowed or commands != STEPS[name][1]:
                break
            names.add(name)
            end += 1
        if OVERLAP_STEPS <= names:
            # Only the plain Cargo page can reuse CSS while WASM Clippy reads it.
            if importlib.import_module("build-web").page_tools(quiet=True):
                if end < len(plan) and plan[end] == ("browser", STEPS["browser"][1]):
                    end += 1
                return start, end
            return None
    return None


def run_overlapped_steps(group, update_native_baseline, *, keep_going=False):
    env = os.environ.copy()
    include_browser = any(name == "browser" for name, _ in group)
    with tempfile.TemporaryDirectory(prefix="check-compile-") as temporary:
        with overlap_jobserver(Path(temporary), os.cpu_count() or OVERLAP_MIN_CPUS) as flags:
            batch = []
            for name, _ in group:
                if name == "browser":
                    continue
                if name == "build" and include_browser:
                    command = [sys.executable, "scripts/check.py", "build", "browser",
                               *(["--keep-going"] if keep_going else [])]
                else:
                    command = [sys.executable, "scripts/check.py", name]
                if name == "test" and update_native_baseline:
                    command.append("--update-native-baseline")
                child_env = dict(env, MAKEFLAGS=flags, **{IN_OVERLAP_ENV: "1"})
                if name == "lint":
                    child_env["CARGO_TARGET_DIR"] = str(ROOT / "target/lint")
                elif name == "test":
                    child_env.setdefault("RUST_TEST_THREADS", str(max(2, (os.cpu_count() or 8) // 4)))
                batch.append([(command, ROOT, child_env)])
            return run_parallel(batch, env)


def run_plan(plan, update_native_baseline=False, *, keep_going=False, partial=False):
    started = time.monotonic()
    totals = {}
    result = 1
    try:
        result = _run_plan(plan, update_native_baseline, keep_going=keep_going, totals=totals)
        return result
    except KeyboardInterrupt:
        result = 130
        return result
    except SystemExit as error:
        if isinstance(error.code, int):
            result = error.code
        raise
    except (OSError, RuntimeError, ValueError) as error:
        print(f"check: {error}", file=sys.stderr)
        return result
    finally:
        if IN_OVERLAP_ENV not in os.environ:
            print(json.dumps({
                "steps": [name for name, _ in plan],
                "scope": "full" if not partial and tuple(name for name, _ in plan
                                                          if name != "test-css") == DEFAULT else "selected",
                "exit_code": result,
                "seconds": round(time.monotonic() - started, 3),
                "step_seconds": {name: round(elapsed, 3) for name, elapsed in totals.items()},
            }), flush=True)


def _run_plan(plan, update_native_baseline=False, *, keep_going=False, totals=None):
    if totals is None:
        totals = {}
    browser_tools._inherited_build_lock()
    span = overlap_span(plan)
    state = {"css": IN_OVERLAP_ENV in os.environ}
    baseline_checked = False
    index = 0
    while index < len(plan):
        name, commands = plan[index]
        env = os.environ.copy()
        if state["css"]:
            env[TEST_CSS_READY] = "1"
        if span and index == span[0]:
            print("== test-css", flush=True)
            totals["test-css"], code = run_serial(test_css_command(), env, ROOT / "web")
            if code:
                return 1
            group = plan[index:span[1]]
            group_name = "/".join(name for name, _ in group)
            totals[group_name], passed = run_overlapped_steps(
                group, update_native_baseline, keep_going=keep_going
            )
            if not passed:
                return 1
            state["css"] = True
            baseline_checked |= update_native_baseline
            index = span[1]
            continue
        print(f"== {name}", flush=True)
        if name == "test":
            env.pop("APPDIR", None)  # KiCad must use its system libraries.
            state.pop("native", None)
        if is_browser_step(name):
            if any(launch_crate(command) for command in commands):
                runner = importlib.import_module("run-wasm-tests")
                env.update(runner.runner_environment(ROOT))
            elif wasm_bindgen_tools.DIRECTORY_ENV in env:
                env = wasm_bindgen_tools.configure(env, ROOT)
        elapsed, passed = run_commands(name, commands, env, ROOT / "web" if name == "test-css" else ROOT,
                                       state=state, keep_going=keep_going and is_browser_step(name))
        totals[name] = totals.get(name, 0) + elapsed
        if passed and (native := state.pop("native", None)):
            passed, written = check_native_tests.verify(
                native, full=all(cmd in commands for cmd in CARGO_TESTS), update=update_native_baseline,
                baseline_path=check_native_tests.BASELINE)
            baseline_checked |= written
        if not passed:
            print(f"check: step '{name}' failed", file=sys.stderr)
            return 1
        index += 1
    if update_native_baseline and not baseline_checked:
        print("check: no native cargo test ran; the baseline is unchanged", file=sys.stderr)
        return 1
    return 0


def run(names, update_native_baseline=False, *, only=None, keep_going=False):
    if only is not None:
        launches = select_browser_crate_launches(only)
        plan = [(f"browser (only: {', '.join(only)})", with_crate_build(launches))]
        return run_plan(plan, update_native_baseline, keep_going=keep_going, partial=True)
    names = list(names)
    smoke = next((index for index, name in enumerate(names)
                  if name in ("browser", "browser-packaged", "drag-smoke")), None)
    if smoke is not None:
        try:
            browser_tools.locate_chrome()
            browser_tools.locate_chromedriver()
        except browser_tools.MissingBrowserTool as error:
            print(f"check: {error}", file=sys.stderr)
            return 1
        if not set(names[:smoke]).intersection(("build", "release")):
            build = next((name for name in names[smoke:] if name in ("build", "release")), None)
            if build:
                names.remove(build)
            names.insert(smoke, build or "build")
    plan = []
    css_ready = IN_OVERLAP_ENV in os.environ
    for name in names:
        commands = STEPS[name][1]
        if name == "build" and "browser" not in names and set(names).intersection(("browser-packaged", "drag-smoke")):
            unwanted = [BROWSER_CRATE_BUILD]
            if "browser-packaged" not in names:
                unwanted.append(PAGE_TEST_BUILD)
            commands = [command for command in commands if command not in unwanted]
        if name == "lint" and not css_ready:
            plan.append(("test-css", [test_css_command()]))
            css_ready = True
        if name in ("build", "release"):
            css_ready = True
        plan.append((name, commands))
    return run_plan(plan, update_native_baseline, keep_going=keep_going)
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
            commands = [command for command in browser() if command != DRAG_SMOKE]
        else:
            commands = with_crate_build(select_browser_crate_launches(crates))
            if selection.page_files:
                commands.append([PY, "scripts/run-wasm-tests.py", "--files", *selection.page_files,
                                 "--depth", "1",
                                 "--result-json", "web/target/test-results/browser-quick.json"])
        if commands:
            plan.append(("browser", commands))
    if selection.cad:
        plan.append(("test", [CAD_TEST]))
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
    return run_plan(plan, update_native_baseline, keep_going=keep_going)


def main(argv: list[str] | None = None) -> int:
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
    if args.files and args.steps != ["quick"]:
        parser.error("--files applies to `quick` only")
    unknown = [name for name in args.steps if name not in STEPS]
    if args.steps != ["quick"] and unknown:
        parser.error(f"unknown step(s): {', '.join(unknown)}; choose from {', '.join(STEPS)}")
    if not args.list:
        browser_tools._inherited_build_lock()
    if args.steps == ["quick"]:
        return quick(args.files, args.update_native_baseline, keep_going=args.keep_going)
    if args.list:
        for name, (included, commands) in STEPS.items():
            print(f"{name}{'*' if included else ''}")
            for command in commands:
                print("  " + shlex.join(command))
        print("quick  (changed files only; see `check.py quick`)")
        return 0
    return run(args.steps or list(DEFAULT), args.update_native_baseline,
               only=only_crates, keep_going=args.keep_going)

def stop_on_sigterm(signum, _frame):
    raise SystemExit(128 + signum)


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, stop_on_sigterm)
    raise SystemExit(main())
