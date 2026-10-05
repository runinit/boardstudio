#!/usr/bin/env python3
"""Small local guard for commits and builds in the registered migration checkout.

This is an accidental-misuse guard for cooperating tools running as one OS user.
It does not protect file edits and can be bypassed by a deliberate user.
"""

from contextlib import contextmanager
import argparse
import fcntl
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import threading
from datetime import datetime, timezone
import time
import uuid


MARKER = "# migration-deliver-hook-v1"


class GuardError(RuntimeError):
    pass


def git_output(root, *args):
    return subprocess.check_output(["git", *args], cwd=root, text=True, stderr=subprocess.PIPE).strip()


def canonical_root(root):
    root = Path(root).resolve()
    return Path(git_output(root, "rev-parse", "--show-toplevel")).resolve()


def common_dir(root):
    raw = Path(git_output(root, "rev-parse", "--git-common-dir"))
    return (Path(root).resolve() / raw).resolve() if not raw.is_absolute() else raw.resolve()


def registry_path(root):
    return common_dir(root) / "migration-deliver-registry.json"


def state_lock(root):
    return common_dir(root) / "migration-deliver-state.lock"


def _locked(root):
    lock_path = state_lock(root)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    stream = lock_path.open("a+")
    fcntl.flock(stream.fileno(), fcntl.LOCK_EX)
    return stream


def _read_registry(root):
    path = registry_path(root)
    if not path.exists():
        return {"version": 1, "checkouts": {}}
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise GuardError(f"cannot read local migration registry {path}: {error}") from error
    if value.get("version") != 1 or not isinstance(value.get("checkouts"), dict):
        raise GuardError(f"unsupported or malformed migration registry: {path}")
    return value


def _write_registry(root, value):
    path = registry_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix="migration-deliver-", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as stream:
            json.dump(value, stream, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def _current_branch(root):
    return git_output(root, "branch", "--show-current")


def _entry(root, registry=None):
    root = canonical_root(root)
    registry = registry if registry is not None else _read_registry(root)
    entry = registry["checkouts"].get(str(root))
    if entry is None:
        return root, None
    branch = _current_branch(root)
    if branch != entry.get("branch"):
        raise GuardError(
            f"registered checkout {root} is on branch {branch!r}; expected {entry.get('branch')!r}"
        )
    if entry.get("role") not in ("author", "coordinator"):
        raise GuardError(f"invalid registered role for {root}")
    return root, entry


def validate_checkout(root, *, required=False):
    root, entry = _entry(root)
    if entry is None and required:
        raise GuardError(f"checkout is not registered for migration delivery: {root}")
    return root, entry


def _freeze_path(root):
    return common_dir(root) / "migration-build-freezes" / (str(uuid.uuid5(uuid.NAMESPACE_URL, str(canonical_root(root)))) + ".json")


def _freeze_lock_path(root):
    return _freeze_path(root).with_suffix(".lock")


def _freeze_active(root):
    path = _freeze_path(root)
    if not path.exists():
        return False
    # The context manager holds this advisory kernel lock for the whole build.
    # A free lock proves a leftover marker is stale, including after SIGKILL;
    # this avoids unsafe PID-liveness guesses and PID-reuse ambiguity.
    lock = _freeze_lock_path(root).open("a+")
    try:
        try:
            # Shared admission permits an active delivery commit to run its
            # pre-commit hook while retaining the lock until `git commit` exits.
            fcntl.flock(lock.fileno(), fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError:
            return True
        path.unlink(missing_ok=True)
        return False
    finally:
        try:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        finally:
            lock.close()


def _freeze_active_synchronized(root):
    lock = _locked(root)
    try:
        return _freeze_active(root)
    finally:
        fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        lock.close()


@contextmanager
def build_freeze(root):
    """Block coordinator integration commits for this registered checkout."""
    root, entry = validate_checkout(root)
    if entry is None:
        yield
        return
    if entry["role"] != "coordinator":
        raise GuardError("only a registered coordinator checkout may start a migration build freeze")
    path = _freeze_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    lock = _locked(root)
    freeze_lock = _freeze_lock_path(root).open("a+")
    token = uuid.uuid4().hex
    lease_acquired = False
    try:
        try:
            fcntl.flock(freeze_lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise GuardError(f"a build freeze is already active for {root}") from error
        lease_acquired = True
        # Any prior marker is stale because this process owns the kernel lock.
        path.unlink(missing_ok=True)
        marker = {"root": str(root), "pid": os.getpid(), "started_unix": time.time(), "token": token}
        fd, temporary = tempfile.mkstemp(prefix="freeze-", dir=path.parent)
        try:
            with os.fdopen(fd, "w") as stream:
                json.dump(marker, stream)
                stream.write("\n")
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, path)
        finally:
            if os.path.exists(temporary):
                os.unlink(temporary)
    except BaseException:
        if lease_acquired:
            fcntl.flock(freeze_lock.fileno(), fcntl.LOCK_UN)
        freeze_lock.close()
        raise
    finally:
        fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        lock.close()
    try:
        yield
    finally:
        lock = _locked(root)
        try:
            if path.exists():
                try:
                    marker = json.loads(path.read_text())
                except (OSError, json.JSONDecodeError):
                    marker = {}
                if marker.get("token") == token and marker.get("root") == str(root):
                    path.unlink()
            fcntl.flock(freeze_lock.fileno(), fcntl.LOCK_UN)
        finally:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
            lock.close()
            freeze_lock.close()


def _check_commit(root, intent=None):
    root, entry = validate_checkout(root)
    if entry is None:
        return
    if entry["role"] == "coordinator":
        if intent != "integration":
            raise GuardError("coordinator commits must use migration-deliver.py commit --intent integration")
        if _freeze_active_synchronized(root):
            raise GuardError("coordinator integration commit blocked while this checkout is build-frozen")
    elif intent == "integration":
        raise GuardError("author role cannot create an integration commit")
    elif intent not in (None, "delivery"):
        raise GuardError("author commits must use delivery intent")


def _git_hook(root):
    try:
        _check_commit(root, os.environ.get("MIGRATION_DELIVERY_INTENT"))
    except (GuardError, subprocess.CalledProcessError) as error:
        print(f"migration-deliver: {error}", file=sys.stderr)
        return 1
    return 0


def _install(root):
    root, entry = validate_checkout(root, required=True)
    if entry["role"] != "coordinator":
        raise GuardError("only the registered coordinator checkout may install the migration hook")
    configured_hooks = subprocess.run(
        ["git", "config", "--show-origin", "--get-all", "core.hooksPath"],
        cwd=root, text=True, capture_output=True,
    )
    if configured_hooks.returncode == 0 and configured_hooks.stdout.strip():
        raise GuardError(
            "core.hooksPath is configured; refusing to install into an unused default hooks directory: "
            + configured_hooks.stdout.strip().replace("\n", "; ")
        )
    if configured_hooks.returncode not in (0, 1):
        raise GuardError(f"could not inspect core.hooksPath: {configured_hooks.stderr.strip()}")
    hooks = common_dir(root) / "hooks"
    hooks.mkdir(parents=True, exist_ok=True)
    hook = hooks / "pre-commit"
    chain = hooks / "pre-commit.migration-deliver-chain"
    existing = hook.read_text() if hook.exists() else None
    if existing and MARKER not in existing:
        if chain.exists():
            raise GuardError(f"refusing to overwrite existing hook chain target {chain}")
        os.replace(hook, chain)
    script = Path(__file__).resolve()
    shim = "\n".join((
        "#!/bin/sh",
        MARKER,
        f"CHAIN={_shell_quote(str(chain))}",
        f"{_shell_quote(sys.executable)} {_shell_quote(str(script))} hook || exit $?",
        'if [ -x "$CHAIN" ]; then exec "$CHAIN" "$@"; fi',
        "exit 0",
        "",
    ))
    fd, temporary = tempfile.mkstemp(prefix="pre-commit-", dir=hooks, text=True)
    try:
        with os.fdopen(fd, "w") as stream:
            stream.write(shim)
        os.chmod(temporary, 0o755)
        os.replace(temporary, hook)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    return hook


def _shell_quote(value):
    return "'" + value.replace("'", "'\\''") + "'"


WASM_CHECK_COMMAND = [
    "cargo", "check", "--manifest-path", "web/Cargo.toml", "--locked",
    "--target", "wasm32-unknown-unknown", "--no-default-features",
    "--features", "page", "--bin", "boardstudio-web",
]


def _load_wasm_lint():
    import importlib.util
    spec = importlib.util.spec_from_file_location("check_wasm_tests", Path(__file__).with_name("check-wasm-tests.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _committed_files(root, paths):
    args = ["git", "diff", "--name-only", "-z", "HEAD"] if paths else ["git", "diff", "--cached", "--name-only", "-z"]
    if paths:
        args.extend(["--", *paths])
    result = subprocess.run(args, cwd=root, capture_output=True, text=True, check=True)
    return [name for name in result.stdout.split("\0") if name]


def _check_staged_build_inputs_match_worktree(root, paths):
    """Ensure default staged commits publish the source that gates exercised."""
    if paths:
        # `git commit -- <paths>` takes those paths from the worktree.
        return
    receipts = _load_gate_receipts()
    staged = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "-z"], cwd=root,
        text=True, capture_output=True, check=True,
    )
    maintained = [name for name in staged.stdout.split("\0")
                  if name and receipts.is_maintained_build_input(root, name)]
    for name in maintained:
        difference = subprocess.run(["git", "diff", "--quiet", "--", name], cwd=root)
        if difference.returncode == 1:
            raise GuardError(
                f"staged build input {name} differs from the worktree tested by migration gates; "
                "stage the tested bytes or restore the matching worktree before committing"
            )
        if difference.returncode != 0:
            raise GuardError(f"cannot compare staged build input with worktree: {name}")


def _tail(text, lines=60):
    return "\n".join(text.strip().splitlines()[-lines:])


NATIVE_WEB_TEST_COMMAND = ["cargo", "test", "--manifest-path", "web/Cargo.toml", "--locked", "--lib", "--bin", "boardstudio-web"]
NATIVE_CORE_TEST_COMMAND = ["cargo", "test", "--manifest-path", "core/Cargo.toml", "--locked"]
_FAILED_TEST = re.compile(r"^test (\S+) \.\.\. FAILED", re.M)


def _load_gate_receipts():
    import importlib.util
    path = Path(__file__).with_name("migration_gate_receipts.py")
    spec = importlib.util.spec_from_file_location("migration_gate_receipts", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _utc(unix_time):
    return datetime.fromtimestamp(unix_time, timezone.utc).isoformat(timespec="seconds")


def _format_gate_result(label, receipt, reused):
    counts = receipt.get("counts", {})
    count_text = ""
    if counts:
        count_text = ", ".join(f"{name}={count}" for name, count in sorted(counts.items()))
    status = "reused" if reused else "passed"
    detail = f" ({count_text})" if count_text else ""
    duration = receipt.get("duration_seconds", 0.0)
    started = _utc(receipt.get("started_unix", time.time()))
    ended = _utc(receipt.get("ended_unix", time.time()))
    identity = receipt.get("identity", {})
    receipt_path = (Path("web/target/migration-gates") / identity.get("gate", "unknown")
                    / f"{identity.get('key', 'unknown')}.json")
    print(f"migration-deliver: {status} {label}{detail}; receipt={receipt_path} "
          f"started={started} ended={ended} duration={duration:.2f}s", file=sys.stderr)


def _cached_gate(root, gate, command, label, *, selected_files=(), extra_tools=(),
                 parser=None, positive=False, env=None, tool_cache=None):
    receipts = _load_gate_receipts()
    try:
        receipt, reused, result = receipts.run_cached(
            root, gate, command, env=env, selected_files=selected_files,
            extra_tools=extra_tools, result_parser=parser, require_positive=positive,
            tool_cache=tool_cache,
        )
    except FileNotFoundError as error:
        raise GuardError(f"cannot run {label}: {error.filename} not found on PATH") from error
    if result.returncode:
        return receipt, False, result
    counts = receipt.get("counts", {})
    if (receipt.get("complete") is not True or counts.get("failed", 0) > 0
            or counts.get("excluded_failures", 0) > 0 or counts.get("incomplete", 0) > 0
            or (positive and counts.get("passed", 0) <= 0)):
        return receipt, False, result
    _format_gate_result(label, receipt, reused)
    return receipt, reused, result


def _native_result(output):
    summaries = list(_TEST_SUMMARY.finditer(output))
    counts = {
        "passed": sum(int(match.group("passed")) for match in summaries),
        "failed": sum(int(match.group("failed")) for match in summaries),
        "ignored": sum(int(match.group("ignored")) for match in summaries),
    }
    return {"counts": counts, "complete": bool(summaries)}


def _run_native_tests(root, command, label, tool_cache=None):
    print(f"migration-deliver: ensuring native {label} tests passed before committing", file=sys.stderr)
    receipt, _reused, result = _cached_gate(
        root, f"native-{label}", command, f"native {label} tests",
        parser=_native_result, positive=True, tool_cache=tool_cache,
    )
    if result.returncode:
        output = result.stdout + "\n" + result.stderr
        names = sorted(set(_FAILED_TEST.findall(output)))
        listing = "\n".join(f"  FAILED {name}" for name in names) or "  (no failing test names found; likely a build error)"
        raise GuardError(
            f"native {label} tests failed (`{' '.join(command)}`). Fix and retry.\n{listing}\n"
            f"Output tail:\n{_tail(output)}"
        )
    counts = receipt.get("counts", {})
    if counts.get("failed", 0) > 0:
        raise GuardError(
            f"native {label} test output reported failures despite a successful command. "
            f"Output tail:\n{_tail(result.stdout + result.stderr)}"
        )
    if not receipt.get("complete") or counts.get("passed", 0) <= 0:
        raise GuardError(
            f"native {label} tests did not execute any passed tests (`{' '.join(command)}`). "
            f"at least one passed test is required. Output tail:\n{_tail(result.stdout + result.stderr)}"
        )


def _wasm_result_parser(path):
    def parse(_output):
        try:
            value = json.loads(Path(path).read_text())
        except (OSError, json.JSONDecodeError):
            return {"complete": False, "counts": {}, "reason": "runner result JSON missing or invalid"}
        if not isinstance(value, dict):
            return {"complete": False, "counts": {}, "reason": "runner result JSON must be an object"}
        expected = value.get("expected_tests")
        listed = value.get("listed_tests")
        terminal = value.get("terminal_outcomes")
        problems = value.get("problems")
        known_failures = value.get("failed_known_exclusions")
        valid_lists = all(isinstance(items, list) for items in (expected, listed, terminal, problems, known_failures))
        if valid_lists:
            valid_lists = (all(isinstance(name, str) for name in expected)
                           and all(isinstance(name, str) for name in listed)
                           and all(isinstance(name, str) for name in known_failures)
                           and all(isinstance(problem, str) for problem in problems))
        outcomes = {}
        if valid_lists:
            for item in terminal:
                if not isinstance(item, dict) or not isinstance(item.get("test"), str):
                    valid_lists = False
                    break
                if item["test"] in outcomes:
                    valid_lists = False
                    break
                outcomes[item["test"]] = item.get("status")
        terminal_names = set(outcomes)
        passed_actual = sum(status == "passed" for status in outcomes.values())
        failed_actual = sum(status == "failed" for status in outcomes.values())
        incomplete_actual = sum(status == "incomplete" for status in outcomes.values())
        raw_counts = (value.get("passed_count"), value.get("failed_count"), value.get("incomplete_count"))
        counts_valid = all(type(count) is int and count >= 0 for count in raw_counts)
        counts = {
            "passed": raw_counts[0] if counts_valid else 0,
            "failed": raw_counts[1] if counts_valid else 1,
            "excluded_failures": len(known_failures) if isinstance(known_failures, list) else 1,
            "incomplete": raw_counts[2] if counts_valid else 1,
        }
        complete = (value.get("schema_version") == 1 and value.get("complete") is True
                    and counts_valid
                    and valid_lists and bool(expected) and not problems
                    and set(expected).issubset(set(listed)) and terminal_names == set(expected)
                    and all(status in ("passed", "failed") for status in outcomes.values())
                    and counts["passed"] == passed_actual and counts["failed"] == failed_actual
                    and counts["incomplete"] == incomplete_actual == 0
                    and set(known_failures).issubset(
                        name for name, status in outcomes.items() if status == "failed"))
        return {"complete": complete, "counts": counts,
                "runner_result": value}
    return parse


def _check_wasm_commit(root, paths):
    """Compile-check and test Rust inputs, reusing only exact-input successes."""
    receipts = _load_gate_receipts()
    # Tool versions and executable digests are stable during this synchronous
    # gate sequence; memoize probes only for this invocation.
    tool_cache = {}
    source_before = receipts.source_fingerprint(root)
    files = _committed_files(root, paths)
    rust = [name for name in files if name.startswith("web/src/") and name.endswith(".rs")]
    core = [name for name in files if name.startswith("core/src/") and name.endswith(".rs")]
    if not rust and not core:
        return source_before
    needs_wasm_check = []
    needs_wasm_tests = []
    if rust:
        wasm_only = set(_load_wasm_lint().wasm_only_files(root))
        needs_wasm_check = [name for name in rust if name.startswith("web/src/presentation/") or name in wasm_only]
        needs_wasm_tests = [
            name for name in needs_wasm_check
            if name in wasm_only or (
                (Path(root) / name).is_file()
                and re.search(r"#\s*\[\s*wasm_bindgen_test\b", (Path(root) / name).read_text())
            )
        ]
    if needs_wasm_check:
        print(f"migration-deliver: {len(needs_wasm_check)} wasm-relevant Rust file(s) in this commit "
              f"(e.g. {needs_wasm_check[0]}); ensuring wasm32 cargo check", file=sys.stderr)
        receipt, _reused, result = _cached_gate(
            root, "wasm-page-check", WASM_CHECK_COMMAND, "wasm32 page check",
            tool_cache=tool_cache,
        )
        if result.returncode:
            raise GuardError(
                "wasm32 page check failed; native cargo test does not compile this code. "
                f"Fix and retry. Compiler output tail:\n{_tail(result.stderr or result.stdout)}"
            )
    _run_native_tests(root, NATIVE_WEB_TEST_COMMAND, "web", tool_cache)
    if core:
        _run_native_tests(root, NATIVE_CORE_TEST_COMMAND, "core", tool_cache)
    if not rust:
        if receipts.source_fingerprint(root) != source_before:
            raise GuardError("maintained source inputs changed during migration gates; rerun prepare-gates")
        return source_before
    print("migration-deliver: ensuring scripts/check-wasm-tests.py passes for Rust files in this commit", file=sys.stderr)
    lint_command = [sys.executable, str(Path(__file__).with_name("check-wasm-tests.py")), "--root", str(root)]
    receipt, _reused, result = _cached_gate(
        root, "wasm-test-reachability", lint_command, "wasm test reachability",
        selected_files=rust, extra_tools=(sys.executable,), tool_cache=tool_cache,
    )
    if result.returncode:
        raise GuardError(
            "check-wasm-tests.py found plain #[test]s that native cargo test never runs:\n"
            + _tail(result.stdout + result.stderr)
        )
    if needs_wasm_tests:
        print("migration-deliver: ensuring wasm-bindgen tests pass in headless Chrome for changed modules", file=sys.stderr)
        result_dir = receipts.cache_directory(root) / "results"
        result_dir.mkdir(parents=True, exist_ok=True)
        fd, result_path_text = tempfile.mkstemp(prefix="wasm-result-", suffix=".json", dir=result_dir)
        os.close(fd)
        result_path = Path(result_path_text)
        try:
            runner_command = [sys.executable, str(Path(__file__).with_name("run-wasm-tests.py")),
                              "--root", str(root), "--files", *needs_wasm_tests,
                              "--result-json", str(result_path)]
            receipt, _reused, result = _cached_gate(
                root, "wasm-headless-tests", runner_command, "headless-Chrome wasm tests",
                selected_files=needs_wasm_tests,
                extra_tools=((sys.executable,) if os.environ.get("BOARDSTUDIO_WASM_TEST_COMMAND") else
                             (sys.executable, "wasm-pack", "chromedriver", "google-chrome", "chromium",
                              "google-chrome-stable", "chromium-browser")),
                parser=_wasm_result_parser(result_path), tool_cache=tool_cache,
            )
        finally:
            result_path.unlink(missing_ok=True)
        counts = receipt.get("counts", {})
        if (result.returncode or not receipt.get("complete")
                or counts.get("passed", 0) <= 0 or counts.get("failed", 0) > 0
                or counts.get("excluded_failures", 0) > 0 or counts.get("incomplete", 0) > 0):
            raise GuardError(
                "headless-Chrome wasm tests failed, were incomplete, or reported known exclusions "
                "(see scripts/run-wasm-tests.py; known failures live in scripts/wasm-known-failures.json):\n"
                + _tail(result.stdout + "\n" + result.stderr)
            )
    if receipts.source_fingerprint(root) != source_before:
        raise GuardError("maintained source inputs changed during migration gates; rerun prepare-gates")
    return source_before


def _commit(root, intent, message, paths):
    root, entry = validate_checkout(root, required=True)
    if entry["role"] == "coordinator" and intent != "integration":
        raise GuardError("coordinator commits require --intent integration")
    if entry["role"] == "author" and intent != "delivery":
        raise GuardError("author role can only commit with --intent delivery")
    # Reap a marker whose kernel lease ended after an interrupted build before
    # this commit takes its own shared lease. The check is repeated below after
    # locking, so a build that wins the gap still blocks the commit.
    _check_commit(root, intent)
    _check_staged_build_inputs_match_worktree(root, paths)
    gate_source = _check_wasm_commit(root, paths)
    lock_path = _freeze_lock_path(root)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    commit_lock = lock_path.open("a+")
    try:
        try:
            fcntl.flock(commit_lock.fileno(), fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise GuardError("guarded commit blocked while this checkout is build-frozen or committing") from error
        _check_commit(root, intent)
        if _load_gate_receipts().source_fingerprint(root) != gate_source:
            raise GuardError("maintained source inputs changed after gate success; rerun prepare-gates")
        environment = os.environ.copy()
        environment["MIGRATION_DELIVERY_INTENT"] = intent
        args = ["git", "commit", "-m", message]
        if paths:
            args.extend(["--", *paths])
        subprocess.run(args, cwd=root, env=environment, check=True)
    finally:
        try:
            fcntl.flock(commit_lock.fileno(), fcntl.LOCK_UN)
        finally:
            commit_lock.close()


_TEST_SUMMARY = re.compile(
    r"test result: .*?\b(?P<passed>\d+) passed;\s*"
    r"(?P<failed>\d+) failed;\s*(?P<ignored>\d+) ignored;"
)


def _focused_test(root, command):
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise GuardError("focused-test requires a command after -- (cargo test ... or wasm-pack test ...)")
    executable = Path(command[0]).name
    is_cargo_test = executable == "cargo" and len(command) > 1 and command[1] == "test"
    is_wasm_pack_test = executable == "wasm-pack" and len(command) > 1 and command[1] == "test"
    if not (is_cargo_test or is_wasm_pack_test):
        raise GuardError("focused-test only accepts cargo test ... or wasm-pack test ... commands")

    started = time.time()
    started_mono = time.monotonic()
    # Keep test output live while retaining only a bounded tail for parsing.
    tails = {"stdout": bytearray(), "stderr": bytearray()}
    tail_limit = 1024 * 1024

    def forward(stream, target, tail):
        while chunk := stream.read(8192):
            target.buffer.write(chunk)
            target.flush()
            tail.extend(chunk)
            if len(tail) > tail_limit:
                del tail[:len(tail) - tail_limit]

    process = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    forwarders = [
        threading.Thread(target=forward, args=(process.stdout, sys.stdout, tails["stdout"])),
        threading.Thread(target=forward, args=(process.stderr, sys.stderr, tails["stderr"])),
    ]
    for thread in forwarders:
        thread.start()
    returncode = process.wait()
    for thread in forwarders:
        thread.join()
    if returncode:
        ended = time.time()
        print(f"migration-deliver: focused-test status=failed command_exit={returncode} "
              f"started={_utc(started)} ended={_utc(ended)} "
              f"duration={time.monotonic() - started_mono:.2f}s", file=sys.stderr)
        return returncode

    output_tail = (tails["stdout"] + b"\n" + tails["stderr"]).decode(errors="replace")
    summaries = list(_TEST_SUMMARY.finditer(output_tail))
    passed = sum(int(match.group("passed")) for match in summaries)
    ended = time.time()
    if passed == 0:
        print(f"migration-deliver: focused-test status=empty started={_utc(started)} "
              f"ended={_utc(ended)} duration={time.monotonic() - started_mono:.2f}s", file=sys.stderr)
        raise GuardError(
            "focused test command succeeded but reported zero executed tests; "
            "expected a Rust test result summary with at least one passed test"
        )
    print(f"migration-deliver: focused-test status=passed passed={passed} "
          f"started={_utc(started)} ended={_utc(ended)} "
          f"duration={time.monotonic() - started_mono:.2f}s", file=sys.stderr)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    register = commands.add_parser("register", help="register this checkout path, branch, and role locally")
    register.add_argument("--role", choices=("author", "coordinator"), required=True)
    register.add_argument("--branch", help="expected branch (defaults to current branch)")
    register.add_argument("--replace-registration", action="store_true", help="explicitly replace this path's existing role/branch")
    commands.add_parser("install", help="install the repo-local pre-commit guard, preserving an existing hook")
    commands.add_parser("check", help="validate this checkout's registration and active freeze state")
    commit = commands.add_parser("commit", help="make a guarded migration delivery/integration commit")
    commit.add_argument("--intent", choices=("delivery", "integration"), required=True)
    commit.add_argument("-m", "--message", required=True)
    commit.add_argument("paths", nargs="*", help="optional pathspecs; otherwise commit the current index")
    prepare = commands.add_parser("prepare-gates", help="run or reuse exact-input gates for the pending commit")
    prepare.add_argument("paths", nargs="*", help="optional pathspecs; otherwise use the current index")
    candidate = commands.add_parser(
        "publish-candidate",
        help="derive and publish a candidate proof from a completed build",
    )
    candidate.add_argument("build_id")
    candidate.add_argument("--root-url", required=True)
    candidate.add_argument("--subpath-url", required=True)
    candidate.add_argument(
        "--extra-candidate", metavar="REASON",
        help="required reason to publish a third or later candidate on the same UTC day",
    )
    candidate.add_argument("--proof", help="output proof path (defaults under candidate evidence)")
    focused_test = commands.add_parser(
        "focused-test",
        help="run cargo/wasm-pack tests and fail if no Rust tests executed",
    )
    focused_test.add_argument("test_command", nargs=argparse.REMAINDER, help="command to run after --")
    commands.add_parser("hook", help="internal entry point used by the installed pre-commit hook")
    args = parser.parse_args(argv)
    try:
        root = canonical_root(Path.cwd())
        if args.command == "hook":
            return _git_hook(root)
        if args.command == "focused-test":
            return _focused_test(root, args.test_command)
        if args.command == "publish-candidate":
            _, entry = validate_checkout(root, required=True)
            if entry["role"] != "coordinator":
                raise GuardError("only the registered coordinator checkout may publish a migration candidate")
            # Imported lazily so the existing local guard remains independent of
            # the progress record and candidate build tooling.
            import importlib.util
            candidate_path = Path(__file__).with_name("migration_candidate.py")
            spec = importlib.util.spec_from_file_location("migration_candidate", candidate_path)
            candidate_module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(candidate_module)
            changed = candidate_module.publish(
                root, args.build_id, args.root_url, args.subpath_url, args.proof,
                extra_reason=args.extra_candidate,
            )
            print("Candidate proof and current run updated" if changed else "Candidate already current")
            return 0
        if args.command == "register":
            branch = args.branch or _current_branch(root)
            if not branch:
                raise GuardError("cannot register a detached checkout; check out its intended branch first")
            current_branch = _current_branch(root)
            if branch != current_branch:
                raise GuardError(f"--branch {branch!r} does not match current branch {current_branch!r}")
            lock = _locked(root)
            try:
                registry = _read_registry(root)
                existing = registry["checkouts"].get(str(root))
                requested = {"branch": branch, "role": args.role}
                if existing and existing != requested and not args.replace_registration:
                    raise GuardError(
                        f"checkout already registered as {existing}; pass --replace-registration to change it"
                    )
                if existing != requested:
                    registry["checkouts"][str(root)] = requested
                    _write_registry(root, registry)
            finally:
                fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
                lock.close()
            print(f"registered {root} on {branch} as {args.role}")
        elif args.command == "install":
            print(f"installed repo-local hook at {_install(root)}")
        elif args.command == "check":
            _, entry = validate_checkout(root, required=True)
            print(f"registered {root} on {entry['branch']} as {entry['role']}; build_frozen={_freeze_active(root)}")
        elif args.command == "prepare-gates":
            _, entry = validate_checkout(root, required=True)
            if entry["role"] != "coordinator":
                raise GuardError("only the registered coordinator checkout may prepare integration gates")
            fingerprint = _check_wasm_commit(root, args.paths)
            print(f"migration-deliver: gates complete for source {fingerprint[:16]}")
        elif args.command == "commit":
            _commit(root, args.intent, args.message, args.paths)
        return 0
    except (GuardError, subprocess.CalledProcessError, OSError, ValueError) as error:
        print(f"migration-deliver: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
