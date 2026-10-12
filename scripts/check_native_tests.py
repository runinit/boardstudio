"""Native test-name guard for `python3 scripts/check.py test`.

AGENTS.md says executed test counts must not drop. The WASM side is guarded by
scripts/check-wasm-tests.py; this guards the native `cargo test` step the same way, by test
NAME rather than by count: a count stays level when one test is deleted and another added.

The names come from the output the `test` step's `cargo test` commands already produce
(`Running ... (target/debug/deps/<binary>-<hash>)` followed by `test <name> ... ok`), so the
suite is never run a second time. scripts/check-native-tests-baseline.json maps each test
binary to the sorted names it executed:

  {"boardstudio_core": ["mirrored_half::reconcile::tests::foo", ...], ...}

A baselined test that is absent from a run, or only `ignored`, fails the step. New tests never
fail it. A full `test` step (the whole workspace and footprints) must also execute every
baselined binary; a partial run (`check.py quick`) checks only the binaries it ran. Doc-tests
are not baselined: their names carry line numbers.

Record intentional removals, renames and additions with
`python3 scripts/check.py test --update-native-baseline` (after the tests pass); review the
JSON diff in the commit. Do not hand-edit the file.
"""

from __future__ import annotations

from dataclasses import dataclass, field
import json
from pathlib import Path
import re
import sys

BASELINE = Path(__file__).with_name("check-native-tests-baseline.json")
RUNNING = re.compile(r"^\s*Running\s.*\(([^()]*)\)\s*$")
DOC_TESTS = re.compile(r"^\s*Doc-tests\s")
# Not anchored to the line start: a test's own uncaptured stderr (a Drop impl printing, say) can
# land on the same line just before libtest's `test <name> ... ok`.
RESULT = re.compile(r"(?:^|\W)test (\S+)(?: - should panic)? \.\.\. (ok|FAILED|ignored)\b")
# A test's stderr can also land between `...` and the verdict (`... Customok {`), which hides it
# from RESULT. A test that wrote output ran, so a name followed by `...` still counts as executed.
STARTED = re.compile(r"(?:^|\W)test (\S+)(?: - should panic)? \.\.\. ")
HASH = re.compile(r"-[0-9a-f]{16}$")
SHOWN = 20  # names listed per binary before "... and N more"


@dataclass
class Run:
    executed: dict[str, set[str]] = field(default_factory=dict)
    ignored: dict[str, set[str]] = field(default_factory=dict)
    binaries: set[str] = field(default_factory=set)

    def count(self) -> int:
        return sum(len(names) for names in self.executed.values())


def binary_name(path: str) -> str:
    """`target/debug/deps/boardstudio_core-0123456789abcdef` -> `boardstudio_core`."""
    name = path.rstrip("/").rsplit("/", 1)[-1]
    if name.endswith(".exe"):
        name = name[:-4]
    return HASH.sub("", name)


def collect(outputs: list[str]) -> Run:
    """Executed (ok or failed) and ignored test names of each test binary, over cargo outputs."""
    run = Run()
    for output in outputs:
        current: str | None = None
        for line in output.splitlines():
            running = RUNNING.match(line)
            if running:
                current = binary_name(running.group(1))
                run.binaries.add(current)
            elif DOC_TESTS.match(line):
                current = None
            elif current is not None:
                result = RESULT.search(line)
                if result:
                    bucket = run.ignored if result.group(2) == "ignored" else run.executed
                    bucket.setdefault(current, set()).add(result.group(1))
                elif started := STARTED.search(line):
                    run.executed.setdefault(current, set()).add(started.group(1))
    return run


def load_baseline(path: Path) -> dict[str, list[str]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or not all(
            isinstance(names, list) and all(isinstance(name, str) for name in names)
            for names in data.values()):
        raise ValueError(f"{path}: expected an object of test-name lists")
    return data


def missing(baseline: dict[str, list[str]], run: Run, full: bool) -> dict[str, list[str]]:
    """Baselined tests the run did not execute. A partial run checks only the binaries it ran."""
    gone: dict[str, list[str]] = {}
    for binary, names in baseline.items():
        if not full and binary not in run.binaries:
            continue
        absent = sorted(set(names) - run.executed.get(binary, set()))
        if absent:
            gone[binary] = absent
    return gone


def updated(baseline: dict[str, list[str]], run: Run, full: bool) -> dict[str, list[str]]:
    """The baseline after recording this run: replaced by a full run, merged by a partial one."""
    merged = {} if full else {name: list(names) for name, names in baseline.items()
                              if name not in run.binaries}
    merged.update({binary: sorted(names) for binary, names in run.executed.items() if names})
    return dict(sorted(merged.items()))


def write_baseline(baseline: dict[str, list[str]], path: Path) -> None:
    path.write_text(json.dumps(baseline, indent=1) + "\n", encoding="utf-8")


def report(gone: dict[str, list[str]], run: Run) -> list[str]:
    lines = []
    for binary, names in gone.items():
        lines.append(f"check: native tests: {binary}: {len(names)} baselined test(s) not executed")
        for name in names[:SHOWN]:
            why = "ignored" if name in run.ignored.get(binary, set()) else "absent"
            lines.append(f"    {name} ({why})")
        if len(names) > SHOWN:
            lines.append(f"    ... and {len(names) - SHOWN} more")
    return lines


def verify(outputs: list[str], *, full: bool, update: bool = False,
           baseline_path: Path | None = None) -> tuple[bool, bool]:
    """Compares a passing run with the baseline. Returns (ok, baseline_written)."""
    baseline_path = baseline_path or BASELINE
    run = collect(outputs)
    if not run.binaries:
        print("check: native tests: no test binary output to inspect; guard skipped", file=sys.stderr)
        return not update, False
    try:
        baseline = load_baseline(baseline_path)
    except FileNotFoundError:
        if not update:
            print(f"check: native tests: {baseline_path} is missing; create it with "
                  "`python3 scripts/check.py test --update-native-baseline`", file=sys.stderr)
            return False, False
        baseline = {}
    except ValueError as error:
        print(f"check: native tests: unreadable baseline: {error}", file=sys.stderr)
        return False, False
    if update:
        write_baseline(updated(baseline, run, full), baseline_path)
        gone = sum(len(names) for names in missing(baseline, run, full).values())
        print(f"check: native tests: baseline recorded, {run.count()} executed test(s)"
              + (f"; {gone} previously baselined test(s) dropped" if gone else "")
              + f" ({baseline_path.name})")
        return True, True
    gone = missing(baseline, run, full)
    if gone:
        for line in report(gone, run):
            print(line, file=sys.stderr)
        print(f"check: native tests: {sum(len(names) for names in gone.values())} baselined test(s) "
              "no longer execute. Restore them, or record a deliberate removal with "
              "`python3 scripts/check.py test --update-native-baseline`", file=sys.stderr)
        return False, False
    known = {name for names in baseline.values() for name in names}
    new = sum(1 for names in run.executed.values() for name in names if name not in known)
    print(f"check: native tests: all {sum(len(names) for names in baseline.values())} baselined "
          f"test(s) executed" + (f"; {new} new, record with --update-native-baseline" if new else ""))
    return True, False
