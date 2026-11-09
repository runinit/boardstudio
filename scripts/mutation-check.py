#!/usr/bin/env python3
"""Check that the tests notice deliberate breakage: apply each mutation, run the tests, restore.

A mutation is a named set of exact-text edits. For each one this script asserts every `old`
occurs exactly `count` times, applies the edits, runs the test command, saves its output as
evidence, restores the original bytes and verifies the restoration by SHA-256. The result is
one of:

  killed                           the tests failed: they noticed the mutation
  survived                         the tests passed: nothing noticed the mutation
  compile-failure (does not count) the mutant did not build, so it says nothing about the tests

Spec file (JSON): a list of
  {"name": "m1-...", "description": "what the mutation does",
   "edits": [{"file": "core/src/x.rs", "old": "exact text", "new": "replacement", "count": 1}]}
`file` is relative to --root; `count` defaults to 1. Edits of one mutation apply in order.

Exit status: 0 when every file was restored (survivors are reported, not failed), 1 when a
file did not restore byte for byte, 2 for a bad spec, a spec that does not match the files, or
a red unmutated run. Files are restored on Ctrl-C and on any error, so a stopped run does not
leave a mutant behind.

Usage: mutation-check.py SPEC.json [--root DIR] [--command "cargo test ..."]
           [--evidence-dir DIR] [--only NAME ...] [--timeout SECONDS]
           [--skip-baseline] [--validate]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
import time

REPO = Path(__file__).resolve().parents[1]
DEFAULT_COMMAND = "cargo test -p boardstudio_core --locked --lib"
KILLED = "killed"
SURVIVED = "survived"
COMPILE_FAILURE = "compile-failure (does not count)"
COMPILE_MARKER = "could not compile"
FAILED_TEST = re.compile(r"^test (\S+) \.\.\. FAILED$")


class SpecError(Exception):
    """The spec is malformed or does not match the files on disk."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_bytes(path: Path) -> bytes:
    return path.read_bytes()


def write_bytes(path: Path, data: bytes) -> None:
    path.write_bytes(data)


def load_spec(path: Path) -> list[dict]:
    try:
        spec = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SpecError(f"{path}: cannot read spec: {error}") from error
    if not isinstance(spec, list) or not spec:
        raise SpecError(f"{path}: the spec must be a non-empty JSON list of mutations")
    seen: set[str] = set()
    for index, mutation in enumerate(spec):
        label = f"{path}: mutation #{index + 1}"
        if not isinstance(mutation, dict):
            raise SpecError(f"{label}: must be an object")
        name = mutation.get("name")
        if not isinstance(name, str) or not re.fullmatch(r"[A-Za-z0-9._-]+", name):
            raise SpecError(f"{label}: `name` must be a file-name-safe string (letters, digits, . _ -)")
        if name in seen:
            raise SpecError(f"{label}: duplicate name {name!r}")
        seen.add(name)
        edits = mutation.get("edits")
        if not isinstance(edits, list) or not edits:
            raise SpecError(f"{name}: `edits` must be a non-empty list")
        for edit in edits:
            if not isinstance(edit, dict):
                raise SpecError(f"{name}: each edit must be an object")
            file = edit.get("file")
            if not isinstance(file, str) or not file or Path(file).is_absolute() or ".." in Path(file).parts:
                raise SpecError(f"{name}: `file` must be a path relative to the root, got {file!r}")
            for key in ("old", "new"):
                if not isinstance(edit.get(key), str):
                    raise SpecError(f"{name}: {file}: `{key}` must be a string")
            if not edit["old"]:
                raise SpecError(f"{name}: {file}: `old` must not be empty")
            if edit["old"] == edit["new"]:
                raise SpecError(f"{name}: {file}: `old` and `new` are identical, so nothing would change")
            count = edit.get("count", 1)
            if not isinstance(count, int) or isinstance(count, bool) or count < 1:
                raise SpecError(f"{name}: {file}: `count` must be a positive integer")
    return spec


def planned_contents(root: Path, mutation: dict, originals: dict[str, bytes] | None = None) -> dict[str, bytes]:
    """The mutated bytes of each file of one mutation; raises when an `old` text does not match
    its expected count. Nothing is written."""
    contents: dict[str, bytes] = {}
    for edit in mutation["edits"]:
        file = edit["file"]
        if file not in contents:
            if originals is not None and file in originals:
                contents[file] = originals[file]
            else:
                try:
                    contents[file] = read_bytes(root / file)
                except OSError as error:
                    raise SpecError(f"{mutation['name']}: cannot read {file}: {error}") from error
        old, new, expected = edit["old"].encode(), edit["new"].encode(), edit.get("count", 1)
        found = contents[file].count(old)
        if found != expected:
            preview = edit["old"].strip().splitlines()[0][:70] if edit["old"].strip() else repr(edit["old"][:40])
            raise SpecError(f"{mutation['name']}: {file}: expected {expected} occurrence(s) of "
                            f"`{preview}`, found {found}")
        contents[file] = contents[file].replace(old, new)
    return contents


def validate(root: Path, spec: list[dict]) -> list[str]:
    problems = []
    for mutation in spec:
        try:
            planned_contents(root, mutation)
        except SpecError as error:
            problems.append(str(error))
    return problems


def run_command(command: list[str], root: Path, timeout: float | None) -> tuple[int, str, bool]:
    """Runs the test command; returns (exit code, combined output, timed out)."""
    process = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                               text=True, errors="replace", start_new_session=True)
    try:
        output, _ = process.communicate(timeout=timeout)
        return process.returncode, output or "", False
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        output, _ = process.communicate()
        return -signal.SIGKILL, output or "", True
    except BaseException:
        # Ctrl-C or an error in this script: do not leave the test run going on a restored tree.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
        raise


def classify(code: int, output: str, timed_out: bool) -> tuple[str, list[str]]:
    failed = [match.group(1) for line in output.splitlines() if (match := FAILED_TEST.match(line.strip()))]
    if timed_out:
        return KILLED, failed  # a mutant that hangs the tests is noticed by them
    if COMPILE_MARKER in output:
        return COMPILE_FAILURE, failed
    if failed or code != 0:
        return KILLED, failed
    return SURVIVED, failed


def restore(root: Path, originals: dict[str, bytes]) -> dict[str, bool]:
    """Writes the original bytes back and verifies each file by hash."""
    restored: dict[str, bool] = {}
    for file, data in originals.items():
        try:
            write_bytes(root / file, data)
        except OSError as error:
            print(f"mutation-check: ERROR: could not write {file} back: {error}", file=sys.stderr)
        try:
            restored[file] = sha256(read_bytes(root / file)) == sha256(data)
        except OSError:
            restored[file] = False
    return restored


def run_mutation(root: Path, mutation: dict, command: list[str], evidence: Path,
                 timeout: float | None) -> dict:
    name = mutation["name"]
    files = sorted({edit["file"] for edit in mutation["edits"]})
    originals = {file: read_bytes(root / file) for file in files}
    before = {file: sha256(data) for file, data in originals.items()}
    started = time.monotonic()
    code, output, timed_out = -1, "", False
    try:
        mutated = planned_contents(root, mutation, originals)
        for file, data in mutated.items():
            write_bytes(root / file, data)
        code, output, timed_out = run_command(command, root, timeout)
    finally:
        # Runs on success, on error and on Ctrl-C alike.
        restored = restore(root, originals)
    status, failed = classify(code, output, timed_out)
    (evidence / f"{name}.log").write_text(output, encoding="utf-8")
    after = {file: sha256(read_bytes(root / file)) for file in files if (root / file).exists()}
    return {
        "name": name,
        "description": mutation.get("description", ""),
        "status": status,
        "exit_code": code,
        "timed_out": timed_out,
        "failed_tests": failed,
        "restored_exact": all(restored.values()) and before == after,
        "files": {file: {"sha256_before": before[file], "sha256_after": after.get(file)} for file in files},
        "seconds": round(time.monotonic() - started, 1),
        "log": f"{name}.log",
    }


def check(root: Path, spec: list[dict], command: list[str], evidence: Path, *, only: list[str] | None = None,
          timeout: float | None = None, skip_baseline: bool = False) -> tuple[list[dict], int]:
    """Runs the mutations. Returns (results, exit status)."""
    selected = [mutation for mutation in spec if not only or mutation["name"] in only]
    unknown = sorted(set(only or ()) - {mutation["name"] for mutation in spec})
    if unknown:
        print(f"mutation-check: no such mutation(s): {', '.join(unknown)}", file=sys.stderr)
        return [], 2
    problems = validate(root, selected)
    if problems:
        for problem in problems:
            print(f"mutation-check: spec mismatch: {problem}", file=sys.stderr)
        return [], 2
    evidence.mkdir(parents=True, exist_ok=True)
    if not skip_baseline:
        print("mutation-check: unmutated baseline run ...", flush=True)
        code, output, timed_out = run_command(command, root, timeout)
        (evidence / "_baseline.log").write_text(output, encoding="utf-8")
        if code != 0 or timed_out:
            print("mutation-check: the unmutated tests do not pass, so no mutation result would mean "
                  f"anything (see {evidence / '_baseline.log'}); fix them or use --skip-baseline",
                  file=sys.stderr)
            return [], 2
    results: list[dict] = []
    try:
        for mutation in selected:
            result = run_mutation(root, mutation, command, evidence, timeout)
            results.append(result)
            note = f", {len(result['failed_tests'])} failing test(s)" if result["failed_tests"] else ""
            print(f"mutation-check: {result['name']}: {result['status']}{note}"
                  f"{'' if result['restored_exact'] else ' -- FILE NOT RESTORED'}", flush=True)
    finally:
        # A stopped run still leaves its evidence and a results file for what completed.
        suffix = "" if not only else "-" + "-".join(only)
        (evidence / f"results{suffix}.json").write_text(
            json.dumps({"command": command, "results": results}, indent=2) + "\n", encoding="utf-8")
    status = 0
    for result in results:
        if not result["restored_exact"]:
            status = 1
            print(f"mutation-check: ERROR: {result['name']}: a file did not restore to its original "
                  "bytes; check `git diff` before doing anything else", file=sys.stderr)
    return results, status


def summary(results: list[dict]) -> str:
    counts = {status: sum(result["status"] == status for result in results)
              for status in (KILLED, SURVIVED, COMPILE_FAILURE)}
    survivors = [result["name"] for result in results if result["status"] == SURVIVED]
    text = (f"mutation-check: {counts[KILLED]} killed, {counts[SURVIVED]} survived, "
            f"{counts[COMPILE_FAILURE]} compile-failure (not counted)")
    return text + (f"; survivors: {', '.join(survivors)}" if survivors else "")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("spec", type=Path, help="JSON list of mutations")
    parser.add_argument("--root", type=Path, default=REPO, help="directory the spec's files are relative to "
                        "and the command runs in (default: the repository root)")
    parser.add_argument("--command", default=DEFAULT_COMMAND, help=f"test command (default: {DEFAULT_COMMAND})")
    parser.add_argument("--evidence-dir", type=Path, default=None,
                        help="where logs and results.json go (default: .evidence/mutations under --root, "
                        "which is gitignored)")
    parser.add_argument("--only", nargs="+", metavar="NAME", help="run just these mutations")
    parser.add_argument("--timeout", type=float, default=1800.0, metavar="SECONDS",
                        help="per run; a mutant that hangs the tests counts as killed (default 1800)")
    parser.add_argument("--skip-baseline", action="store_true", help="do not run the unmutated tests first")
    parser.add_argument("--validate", action="store_true",
                        help="only check the spec against the files (counts of `old`); run nothing")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        spec = load_spec(args.spec)
    except SpecError as error:
        print(f"mutation-check: {error}", file=sys.stderr)
        return 2
    if args.validate:
        problems = validate(root, spec)
        for problem in problems:
            print(f"mutation-check: spec mismatch: {problem}", file=sys.stderr)
        if not problems:
            print(f"mutation-check: spec matches the files ({len(spec)} mutation(s))")
        return 2 if problems else 0
    evidence = (args.evidence_dir or root / ".evidence" / "mutations").resolve()

    def terminate(*_):
        raise KeyboardInterrupt  # SIGTERM unwinds through the `finally` blocks like Ctrl-C does

    signal.signal(signal.SIGTERM, terminate)
    results, status = check(root, spec, shlex.split(args.command), evidence, only=args.only,
                            timeout=args.timeout, skip_baseline=args.skip_baseline)
    if results:
        print(summary(results))
        print(f"mutation-check: evidence in {evidence}")
    return status


if __name__ == "__main__":
    raise SystemExit(main())
