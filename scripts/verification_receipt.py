"""Opt-in verification receipt for `python3 scripts/check.py --receipt PATH`.

The receipt is written from the output the check invocation already captures, so it never
runs a suite twice. It records the invocation's planned and run commands, each command's
scope, exit status and observed results, and keeps scopes separate:

  * `native`     `cargo test` binaries, with every observed test name and outcome and the
                 native name guard's matched/missing/new semantics against the retention list.
  * `wasm-crate` each direct `wasm-pack test web/crates/<crate>` launch, with full names.
  * `page-suite` the `web/src` page runner JSON (a scoped part of the browser gate, not all of it).
  * `smoke`      the real-browser drag smoke and CAD provider checks: a separate evidence
                 scope, never added to Rust test counts.

Focused module groupings are recomputed from each launch's observed names. A before/after
group delta is only reported when a caller explicitly supplies a previously written base
receipt; on its own, the retention list is not a before-count and the comparison is unknown.

The receipt is invocation-scoped: a command that ran is fresh evidence; a planned command that
did not run is `not_run`; malformed or interrupted output is incomplete, never zero failures.

`check.py --receipt PATH` also writes a reviewer summary derived from the receipt to `PATH.md`.
That summary is the evidence to provide at review handoff.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
import json
import hashlib
from pathlib import Path
import re
import subprocess
import threading
import uuid

import check_native_tests

SCHEMA_VERSION = 1

NATIVE = "native"
WASM_CRATE = "wasm-crate"
PAGE_SUITE = "page-suite"
SMOKE = "smoke"
OTHER = "other"

SMOKE_SCRIPTS = {"run-drag-browser.py", "test-cad-provider.py", "test-cad-browser.py", "face-completeness-gate.py"}

# Terminal lines of a direct `wasm-pack test` launch.
WASM_STATUS = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|FAIL|ignored)\s*$", re.M)
WASM_INVOKED = re.compile(r"^\s*Invoking test: (\S+)\s*$", re.M)
WASM_SUMMARY = re.compile(
    r"^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) filtered out", re.M)

_OUTCOMES = {"ok": "passed", "FAILED": "failed", "FAIL": "failed", "ignored": "ignored"}


@dataclass
class Observation:
    """One command this invocation ran, with the output it captured (empty when not captured)."""
    step: str
    command: list[str]
    status: str = "ran"
    exit_code: int | None = None
    elapsed: float | None = None
    output: str = ""
    started_at: str | None = None
    ended_at: str | None = None
    attempt_id: str | None = None
    retry_of: str | None = None
    page_report: dict | None = None
    page_captured: bool = False


def script_name(command: list[str]) -> str:
    for part in command[1:]:
        if part.endswith(".py") or part.endswith(".cjs"):
            return Path(part).name
    return ""


def scope_of(command: list[str]) -> str:
    if command[:2] == ["cargo", "test"]:
        return NATIVE
    if command[:2] == ["wasm-pack", "test"]:
        return WASM_CRATE if crate_of(command) else OTHER
    name = script_name(command)
    if name == "run-wasm-tests.py":
        return PAGE_SUITE
    if name in SMOKE_SCRIPTS:
        return SMOKE
    return OTHER


def crate_of(command: list[str]) -> str | None:
    return next((part.removeprefix("web/crates/") for part in command
                 if part.startswith("web/crates/")), None)


def group_counts(names: list[str]) -> dict[str, int]:
    """Recommended focused groups: the module prefix (name minus its final path segment)."""
    counts: dict[str, int] = {}
    for name in names:
        prefix = name.rsplit("::", 1)[0] if "::" in name else ""
        counts[prefix] = counts.get(prefix, 0) + 1
    return dict(sorted(counts.items()))


def _result_json_path(command: list[str]) -> str | None:
    if "--result-json" in command:
        index = command.index("--result-json")
        if index + 1 < len(command):
            return command[index + 1]
    return None


def load_json(path: str) -> dict | None:
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else None
    except (OSError, ValueError):
        return None


def crate_record(observation: Observation) -> dict:
    output = observation.output
    outcomes = {name: _OUTCOMES[status] for name, status in WASM_STATUS.findall(output)}
    invoked = WASM_INVOKED.findall(output)
    incomplete = [name for name in invoked if name not in outcomes]
    summary = WASM_SUMMARY.search(output)
    problems: list[str] = []
    if summary:
        passed, failed, ignored, filtered = (int(value) for value in summary.groups())
        if passed + failed + ignored != len(outcomes):
            problems.append(f"libtest summary reports {passed + failed + ignored} terminal test(s) but "
                            f"{len(outcomes)} names were observed")
        if "FAILED" in summary.group(0) and failed == 0:
            problems.append("libtest summary reports FAILED with no failed test name")
    else:
        passed = sum(1 for outcome in outcomes.values() if outcome == "passed")
        failed = sum(1 for outcome in outcomes.values() if outcome == "failed")
        ignored = sum(1 for outcome in outcomes.values() if outcome == "ignored")
        filtered = 0
        problems.append("no libtest summary line; output is malformed or the launch was interrupted")
    if incomplete:
        problems.append("invoked test(s) without a terminal result: " + ", ".join(incomplete))
    complete = summary is not None and not problems
    return {
        "complete": complete,
        "passed": observation.exit_code == 0 and failed == 0 and complete and not incomplete,
        "counts": {"passed": passed, "failed": failed, "ignored": ignored,
                   "incomplete": len(incomplete), "filtered_out": filtered},
        "tests": [{"name": name, "outcome": outcome} for name, outcome in sorted(outcomes.items())],
        "groups": group_counts(list(outcomes)),
        "problems": problems,
    }


def page_record(report: dict | None, *, path: str | None, exit_code: int | None) -> dict:
    valid = (isinstance(report, dict) and report.get("schema_version") == 1
             and isinstance(report.get("terminal_outcomes"), list)
             and all(isinstance(item, dict) and isinstance(item.get("test"), str)
                     and item.get("status") in ("passed", "failed", "incomplete")
                     for item in report["terminal_outcomes"])
             and all(type(report.get(key)) is int and report[key] >= 0
                     for key in ("passed_count", "failed_count", "incomplete_count"))
             and type(report.get("complete")) is bool
             and all(isinstance(report.get(key), list) and all(isinstance(item, str) for item in report[key])
                     for key in ("expected_tests", "failed_known_exclusions", "problems")))
    if not valid:
        return {"complete": False, "passed": False, "accepted": False, "counts": None, "tests": [],
                "groups": {}, "raw_failures": [], "known_exclusions": [], "fresh": report is not None,
                "result_json": path, "problems": [f"page-suite result JSON {path} was unreadable, stale or malformed"]}
    terminal = report.get("terminal_outcomes") or []
    tests = [{"name": item.get("test"), "outcome": item.get("status")} for item in terminal]
    known = list(report.get("failed_known_exclusions") or [])
    failed = [test["name"] for test in tests if test["outcome"] == "failed"]
    problems = list(report["problems"])
    for outcome, key in (("passed", "passed_count"), ("failed", "failed_count"), ("incomplete", "incomplete_count")):
        if sum(test["outcome"] == outcome for test in tests) != report[key]:
            problems.append(f"page {key} disagrees with terminal outcomes")
    if report["complete"] and (set(report["expected_tests"]) != {test["name"] for test in tests}
                               or len(tests) != len({test["name"] for test in tests})):
        problems.append("complete page result has missing or duplicate terminal names")
    complete = report["complete"] and not problems
    return {
        "complete": complete,
        "passed": exit_code == 0 and report["failed_count"] == 0 and complete,
        "accepted": exit_code == 0 and not problems,
        "counts": {"passed": report.get("passed_count", 0), "failed": report.get("failed_count", 0),
                   "incomplete": report.get("incomplete_count", 0),
                   "expected": len(report.get("expected_tests") or [])},
        "tests": tests,
        "groups": group_counts([test["name"] for test in tests if test["name"]]),
        "raw_failures": [name for name in failed if name not in known],
        "known_exclusions": known,
        "problems": problems,
        "fresh": True,
        "result_json": path,
    }


def _launch_problems(launch: check_native_tests.Launch) -> list[str]:
    """Why one launched binary is not terminally complete: summary, announcement and named outcomes."""
    name = launch.name
    terminal = {test: outcome for test, outcome in launch.outcomes.items() if outcome != "started"}
    problems: list[str] = []
    if launch.summary is None:
        problems.append(f"native binary {name} has no terminal test-result summary; "
                        "the launch is interrupted or its output is malformed")
    else:
        status, passed, failed, ignored, _measured, _filtered = launch.summary
        # Reconcile each named outcome, not just the total: a summary can carry the right number
        # of tested results while mislabelling them (`0 passed; 1 failed` against one `ok`).
        for outcome, reported in (("passed", passed), ("failed", failed), ("ignored", ignored)):
            observed = sum(1 for value in terminal.values() if value == outcome)
            if reported != observed:
                problems.append(f"native binary {name}: its summary reports {reported} {outcome} "
                                f"test(s) but {observed} named outcome(s) were observed")
        # libtest prints FAILED exactly when a test failed; either token alone is contradictory.
        if (status == "FAILED") != (failed > 0):
            problems.append(f"native binary {name}: its summary is {status} but reports "
                            f"{failed} failed test(s)")
    if launch.announced is None:
        problems.append(f"native binary {name} never announced a test count; "
                        "the launch was interrupted before it ran any test")
    elif launch.announced != len(terminal):
        problems.append(f"native binary {name}: announced {launch.announced} test(s) "
                        f"but {len(terminal)} named outcome(s) were observed")
    return problems


def native_record(outputs: list[str], *, full: bool, baseline: dict[str, list[str]] | None,
                  provenance: dict | None = None) -> dict:
    """Native binary counts, outcomes and the retention guard's matched/missing/new semantics.

    Each launched binary is reconciled independently against its own announcement and terminal
    summary, so a truncated launch stays incomplete even when another binary or the command's
    exit code is complete. Only named tests without a terminal result are counted incomplete.
    """
    run = check_native_tests.collect(outputs)
    launches = check_native_tests.launches(outputs)
    known = {(binary, name) for binary, names in (baseline or {}).items() for name in names}
    executed = {(binary, name) for binary, names in run.executed.items() for name in names}
    gone = check_native_tests.missing(baseline, run, full) if baseline is not None else {}
    merged: dict[str, dict[str, str]] = {}
    for launch in launches:
        merged.setdefault(launch.name, {}).update(launch.outcomes)
    counts = {"passed": 0, "failed": 0, "ignored": 0, "incomplete": 0}
    for names in merged.values():
        for outcome in names.values():
            counts[outcome if outcome in counts else "incomplete"] += 1
    binaries = []
    for binary in sorted(merged):
        binary_counts = {"passed": 0, "failed": 0, "ignored": 0, "incomplete": 0}
        for outcome in merged[binary].values():
            binary_counts[outcome if outcome in binary_counts else "incomplete"] += 1
        binaries.append({"name": binary, "counts": binary_counts,
                         "groups": group_counts(list(merged[binary])),
                         "tests": [{"name": name, "outcome": outcome}
                                   for name, outcome in sorted(merged[binary].items())]})
    problems = [f"native binary {launch.name} has invoked test(s) without a terminal result"
                for launch in launches if "started" in launch.outcomes.values()]
    problems += [problem for launch in launches for problem in _launch_problems(launch)]
    if not run.binaries:
        problems.append("no native binary results observed; output is incomplete or malformed")
    return {
        "full": full,
        "binaries": binaries,
        "counts": counts,
        "problems": problems,
        "baseline": {
            "available": baseline is not None,
            "known": len(known),
            "matched": len(known & executed),
            "missing": gone,
            "new": [f"{binary}::{name}" for binary, name in sorted(executed - known)],
            **(provenance or {}),
        },
    }


def _native_missing_report_labels(gone: dict[str, list[str]], outcomes: dict[str, dict[str, str]]) -> list[str]:
    return [f"{binary}: {name} ({'ignored' if outcomes.get(binary, {}).get(name) == 'ignored' else 'absent'})"
            for binary, names in sorted(gone.items()) for name in names]


def _command_record(step: str, command: list[str], observation: Observation | None,
                    page_loader) -> dict:
    scope = scope_of(command)
    record = {"step": step, "scope": scope, "command": list(command), "crate": crate_of(command)}
    if observation is None:
        record.update({"status": "not_run", "exit_code": None, "elapsed_s": None,
                       "complete": False, "passed": None, "counts": None, "tests": [], "groups": {},
                       "problems": []})
        return record
    record.update({"status": observation.status, "exit_code": observation.exit_code,
                   "elapsed_s": observation.elapsed, "started_at": observation.started_at,
                   "ended_at": observation.ended_at})
    record.update({"attempt_id": observation.attempt_id, "retry_of": observation.retry_of})
    if observation.status != "ran":
        record.update({"complete": False, "passed": False, "counts": None, "tests": [],
                       "groups": {}, "problems": ["receipt recording failed" if observation.status == "recording_failed"
                                                   else "attempt did not finish"]})
        return record
    if scope == WASM_CRATE:
        record.update(crate_record(observation))
    elif scope == NATIVE:
        native = native_record([observation.output], full=False, baseline=None)
        record.update({"binaries": native["binaries"], "counts": native["counts"],
                       "tests": [test for binary in native["binaries"] for test in binary["tests"]],
                       "groups": {binary["name"]: binary["groups"] for binary in native["binaries"]},
                       "problems": native["problems"],
                       "complete": not native["problems"],
                       "passed": observation.exit_code == 0 and not native["problems"]
                                 and native["counts"]["failed"] == 0})
    elif scope == PAGE_SUITE:
        path = _result_json_path(command)
        record.update(page_record(observation.page_report if observation.page_captured else
                                  (page_loader(path) if path else None), path=path,
                                  exit_code=observation.exit_code))
    else:
        record.update({"complete": None, "passed": observation.exit_code == 0,
                       "counts": None, "tests": [], "groups": {}, "problems": []})
    return record


def _before_after(native: dict | None, commands: list[dict], base: dict | None,
                  base_path: Path | None) -> dict:
    if not base or base.get("schema_version") != SCHEMA_VERSION or not (base.get("source") or {}).get("revision"):
        return {"status": "unknown", "reason": "no base-revision run evidence supplied"}
    base_by_key = {(record["scope"], tuple(record.get("command", []))): record
                   for record in base.get("commands", [])}
    groups = []
    for record in commands:
        if record["scope"] != WASM_CRATE or not record.get("passed") or not record.get("complete"):
            continue
        base_record = base_by_key.get((record["scope"], tuple(record["command"])))
        if base_record is None or not base_record.get("passed") or not base_record.get("complete"):
            continue
        current = record.get("groups") or {}
        before = base_record.get("groups") or {}
        delta = {prefix: current.get(prefix, 0) - before.get(prefix, 0)
                 for prefix in sorted(set(current) | set(before))}
        groups.append({"scope": record["scope"], "crate": record.get("crate"),
                       "current": current, "base": before, "delta": delta})
    return {
        "status": "compared" if groups else "unknown",
        "reused": True,
        "base": {"path": str(base_path) if base_path else None,
                 "run_id": base.get("run_id"), "source": base.get("source"),
                 "generated_by": base.get("generated_by")},
        "reason": None if groups else "no complete base/current launches with matching command scopes",
        "groups": groups,
    }


def build_receipt(plan: list[tuple[str, list[str]]], observations: list[Observation], *,
                  source: dict, exit_code: int, partial: bool, native: dict | None = None,
                  page_loader=None, base: dict | None = None, base_path: Path | None = None,
                  run_id: str | None = None, source_after: dict | None = None) -> dict:
    """Assemble a receipt from `(step, command)` pairs and the observations of the commands that ran."""
    page_loader = page_loader or load_json
    remaining = list(observations)
    commands = []
    for step, command in plan:
        match = next((item for item in remaining if item.retry_of is None
                      and item.step == step and item.command == command), None)
        if match is not None:
            remaining.remove(match)
        commands.append(_command_record(step, command, match, page_loader))
        if match is not None:
            retries = [item for item in remaining if item.retry_of == match.attempt_id and item.retry_of is not None]
            for retry in retries:
                remaining.remove(retry)
                commands.append(_command_record(step, command, retry, page_loader))
    commands.extend(_command_record(item.step, item.command, item, page_loader) for item in remaining)
    not_run = [record for record in commands if record["status"] == "not_run"]
    def valid(identity):
        return (isinstance(identity, dict) and
                re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", identity.get("revision") or "") and
                re.fullmatch(r"[0-9a-fA-F]{64}", identity.get("digest") or "") and
                isinstance(identity.get("dirty"), bool))
    bound = valid(source) and valid(source_after)
    return {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id or uuid.uuid4().hex,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "generated_by": "scripts/check.py",
        "source": source,
        "source_binding": {"scope": "git tracked and non-ignored untracked files; excludes ignored and external inputs",
                           "status": "unknown" if not bound or source_after == source else "mismatch",
                           "after": source_after,
                           "correlated": "unknown" if not bound else
                           ("unknown" if source_after == source else "invalid"),
                           "build_served_binding": "unknown"},
        "invocation": {
            "steps": list(dict.fromkeys(step for step, _ in plan)),
            "planned_commands": len(plan),
            "ran_commands": sum(record["status"] != "not_run" for record in commands),
            "not_run_commands": [{"step": record["step"], "scope": record["scope"],
                                  "command": record["command"]} for record in not_run],
            "partial": bool(partial),
            "exit_code": exit_code,
            "accepted": (exit_code == 0 and not not_run and
                         all(record["status"] == "ran" and record["passed"] is not False
                             for record in commands)),
        },
        "commands": commands,
        "native": native,
        "before_after": _before_after(native, commands, base, base_path),
    }


def _short(revision: object) -> str:
    return str(revision or "unknown")[:12]


def render_summary(receipt: dict) -> str:
    """The reviewer summary, derived only from the receipt."""
    source = receipt.get("source") or {}
    invocation = receipt.get("invocation") or {}
    lines = ["# Verification receipt summary", ""]
    dirty = source.get("dirty")
    source_state = "local changes" if dirty is True else "clean tree" if dirty is False else "source state unknown"
    lines.append(f"Source: {source.get('branch') or '?'}@{_short(source.get('revision'))} ({source_state})")
    binding = receipt.get("source_binding") or {}
    lines.append(f"Source correlation: {binding.get('status', 'unknown')}; "
                 f"build/served binding: {binding.get('build_served_binding', 'unknown')}")
    selection = "partial selection" if invocation.get("partial") else "full selection"
    accepted = "commands accepted" if invocation.get("accepted") else "NOT accepted"
    lines.append(f"Invocation: steps {', '.join(invocation.get('steps') or [])} — {selection}; "
                 f"{invocation.get('ran_commands')}/{invocation.get('planned_commands')} commands ran; "
                 f"exit {invocation.get('exit_code')} ({accepted})")
    for command in invocation.get("not_run_commands") or []:
        lines.append(f"Planned but not run: {command['scope']} ({' '.join(command['command'])})")
    lines.append("")

    native = receipt.get("native")
    lines.append("## Native tests (`cargo test`)")
    if native:
        counts = native.get("counts") or {}
        problems = native.get("problems") or []
        lines.append(f"- {len(native.get('binaries') or [])} binary run(s); {counts.get('passed', 0)} passed, "
                     f"{counts.get('failed', 0)} failed, {counts.get('ignored', 0)} ignored, "
                     f"{counts.get('incomplete', 0)} incomplete"
                     + ("; INCOMPLETE" if problems else ""))
        for problem in problems:
            lines.append(f"  - {problem}")
        baseline = native.get("baseline") or {}
        if baseline.get("available"):
            lines.append(f"- retention list `{baseline.get('path', 'check-native-tests-baseline.json')}` recorded at "
                         f"{_short(baseline.get('recording_revision'))} "
                         f"({'local changes' if baseline.get('dirty') else 'clean'}): "
                         f"{baseline.get('matched', 0)} matched, "
                         f"{sum(len(names) for names in (baseline.get('missing') or {}).values())} missing, "
                         f"{len(baseline.get('new') or [])} new")
            for name in _native_missing_report_labels(
                    baseline.get("missing") or {},
                    {binary["name"]: {test["name"]: test["outcome"] for test in binary["tests"]}
                     for binary in native.get("binaries") or []}):
                lines.append(f"  - missing: {name}")
        else:
            lines.append("- retention list unavailable; matched/missing/new are unknown")
    else:
        lines.append("- not run in this invocation")

    lines.append("")
    lines.append("## WASM crate launches")
    crates = [record for record in receipt.get("commands", []) if record["scope"] == WASM_CRATE]
    if crates:
        for record in crates:
            counts = record.get("counts")
            if counts is None:
                lines.append(f"- web/crates/{record.get('crate')}: {record['status']}; counts unavailable")
                continue
            lines.append(f"- web/crates/{record.get('crate')}: {record['status']} exit {record['exit_code']}; "
                         f"{counts.get('passed', 0)} passed, {counts.get('failed', 0)} failed, "
                         f"{counts.get('ignored', 0)} ignored, {counts.get('incomplete', 0)} incomplete; "
                         f"{'complete' if record.get('complete') else 'INCOMPLETE'}")
            if record.get("groups"):
                groups = ", ".join(f"{prefix} {count}" for prefix, count in record["groups"].items())
                lines.append(f"  - groups: {groups}")
    else:
        lines.append("- not run in this invocation")

    lines.append("")
    lines.append("## Page suite (`web/src`, filtered; one part of the browser gate)")
    pages = [record for record in receipt.get("commands", []) if record["scope"] == PAGE_SUITE]
    if pages:
        for record in pages:
            counts = record.get("counts")
            freshness = "fresh" if record.get("fresh") else "freshness unverified"
            lines.append(f"- {record.get('result_json')} ({freshness}); {record['status']}; "
                         f"{'complete' if record.get('complete') else 'INCOMPLETE'}; "
                         f"{'gate accepted' if record.get('accepted') else 'gate NOT accepted'}")
            if counts is None:
                lines.append("  - counts unavailable; no current, usable page results")
                continue
            lines.append(f"  - {counts.get('passed', 0)} passed, {counts.get('failed', 0)} failed, "
                         f"{counts.get('incomplete', 0)} incomplete of {counts.get('expected', 0)} expected")
            if record.get("raw_failures"):
                lines.append(f"  - raw failures: {len(record['raw_failures'])} — "
                             + ", ".join(record["raw_failures"]))
            if record.get("known_exclusions"):
                lines.append(f"  - known exclusions: {len(record['known_exclusions'])} — "
                             + ", ".join(record["known_exclusions"]))
    else:
        lines.append("- not run in this invocation")

    lines.append("")
    lines.append("## Provider/product smoke (separate evidence scope, not Rust test counts)")
    smoke = [record for record in receipt.get("commands", []) if record["scope"] == SMOKE]
    if smoke:
        for record in smoke:
            lines.append(f"- {script_name(record['command']) or record['command'][0]}: "
                         f"{record['status']} exit {record['exit_code']}")
    else:
        lines.append("- not run in this invocation")

    lines.append("")
    lines.append("## Before/after")
    before_after = receipt.get("before_after") or {}
    if before_after.get("status") == "compared":
        base = before_after.get("base") or {}
        lines.append(f"- compared (reused evidence) against {base.get('path')} at "
                     f"{_short((base.get('source') or {}).get('revision'))}")
        for entry in before_after.get("groups") or []:
            delta = ", ".join(f"{prefix}: {entry['base'].get(prefix, 0)} -> {entry['current'].get(prefix, 0)}"
                              for prefix in entry["delta"])
            lines.append(f"  - web/crates/{entry['crate']}: {delta}")
    else:
        lines.append(f"- unknown: {before_after.get('reason', 'no base-revision run evidence supplied')}")

    lines.append("")
    lines.append("## Limitations")
    if invocation.get("partial"):
        lines.append("- partial selection: omitted steps were not run; this is not full verification")
    if pages:
        lines.append("- the page suite covers only `web/src`; it is not the whole browser gate")
    if any(record["status"] == "not_run" for record in receipt.get("commands", [])):
        lines.append("- planned commands did not all run; see `not_run_commands`")
    if not lines[-1].startswith("-"):
        lines.append("- none recorded")
    return "\n".join(lines) + "\n"


def write_receipt(path: Path | str, receipt: dict) -> tuple[Path, Path]:
    json_path = Path(path)
    json_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary_path = Path(str(json_path) + ".md")
    summary_path.write_text(render_summary(receipt), encoding="utf-8")
    return json_path, summary_path


def site_identity(path: Path) -> tuple[dict | None, tuple | None]:
    """Observe the subpath route used by the build and drag smoke."""
    try:
        path = path / "site-subpath/boardstudio"
        if not (path / "index.html").is_file():
            return None, None
        files = sorted(p for p in path.rglob("*") if p.is_file())
        digest = hashlib.sha256()
        states = []
        for file in files:
            if file.is_symlink():
                return None, None
            relative = file.relative_to(path).as_posix().encode()
            data = file.read_bytes()
            stat = file.stat()
            digest.update(len(relative).to_bytes(8, "big"))
            digest.update(relative)
            digest.update(len(data).to_bytes(8, "big"))
            digest.update(data)
            states.append((relative, stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns))
        return {"path": str(path), "sha256": digest.hexdigest()}, tuple(states)
    except OSError:
        return None, None


def fresh_site(before, after):
    # Snapshots cannot prove that retained files were produced by this build.
    return None


def build_binding(run_id, attempt_id, before, after, code, output, fresh):
    def valid(value):
        return (isinstance(value, dict) and
                re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", value.get("revision") or "") and
                re.fullmatch(r"[0-9a-fA-F]{64}", value.get("digest") or "") and
                isinstance(value.get("dirty"), bool))
    comparison = ("equal" if before == after else "different") if valid(before) and valid(after) else "unknown"
    return {"status": "unknown", "source_snapshot_scope": "git tracked and non-ignored untracked files; excludes ignored and external inputs",
            "source_snapshot_comparison": comparison,
            "site_routes_changed": fresh, "run_id": run_id,
            "attempt_id": attempt_id, "source_before": before, "source_after": after,
            "output": output if code == 0 else None}



def drag_binding(run_id, attempt_id, observed, builds):
    result = {"run_id": run_id, "attempt_id": attempt_id, "receipt_path": None,
              "build_attempt_id": None, "site_comparison": "unknown", "served_content": "unknown"}
    if (not run_id or not attempt_id or not isinstance(observed, dict)
            or observed.get("run_id") != run_id or observed.get("attempt_id") != attempt_id
            or not isinstance(observed.get("receipt_path"), str) or not observed["receipt_path"]):
        return result
    result["receipt_path"] = observed["receipt_path"]
    site = observed.get("site")
    build = next((b for b in reversed(builds) if b.get("run_id") == run_id), None)
    if not build or not build.get("attempt_id") or not isinstance(site, dict):
        return result
    output = build.get("output")
    if (not isinstance(output, dict) or not site.get("path") or site.get("path") != output.get("path")
            or not re.fullmatch(r"[0-9a-f]{64}", site.get("sha256") or "")
            or not re.fullmatch(r"[0-9a-f]{64}", output.get("sha256") or "")):
        return result
    result["build_attempt_id"] = build["attempt_id"]
    result["site_comparison"] = "equal" if site["sha256"] == output["sha256"] else "different"
    return result

def source_identity(root: Path, run=subprocess.run) -> dict:
    """The checked-out revision and whether the tree has local changes."""
    def git(*args: str) -> str:
        try:
            done = run(["git", *args], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        except OSError:
            return None
        return (done.stdout or "").strip() if done.returncode == 0 else None

    status = git("status", "--porcelain=v2", "--ignore-submodules=none")
    revision = git("rev-parse", "HEAD")
    if status is None or not revision or not re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", revision):
        return {"revision": None, "digest": None, "dirty": None}
    # Git's superproject diff does not capture a submodule's dirty contents.
    if any(len(parts) > 2 and parts[2].startswith("S") and parts[2][2:] != ".."
           for line in status.splitlines() if (parts := line.split(" ", 3)) and parts[0] in ("1", "2", "u")):
        return {"revision": revision, "digest": None, "dirty": True}
    digest = hashlib.sha256()
    # Git diffs capture tracked content; untracked files are included explicitly.
    for args in (("diff", "--binary", "HEAD"), ("ls-files", "--others", "--exclude-standard", "-z")):
        try:
            done = run(["git", *args], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        except OSError:
            return {"revision": None, "digest": None, "dirty": None}
        if done.returncode:
            return {"revision": None, "digest": None}
        data = done.stdout or b""
        if isinstance(data, str):
            data = data.encode()
        digest.update(data)
        if args[0] == "ls-files":
            for name in data.split(b"\0"):
                if name:
                    try:
                        content = (root / name.decode()).read_bytes()
                        digest.update(len(content).to_bytes(8, "big"))
                        digest.update(content)
                    except (OSError, UnicodeError):
                        return {"revision": None, "digest": None}
    return {
        "revision": revision,
        "branch": git("rev-parse", "--abbrev-ref", "HEAD") or None,
        "dirty": bool(status),
        "local_changes": len(status.splitlines()),
        "digest": digest.hexdigest(),
    }


def baseline_provenance(path: Path, root: Path, run=subprocess.run) -> dict:
    """The revision that last recorded the retention list, and whether it has local changes."""
    def git(*args: str) -> str:
        done = run(["git", *args], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        return (done.stdout or "").strip()

    relative = str(path.relative_to(root)) if path.is_relative_to(root) else str(path)
    return {
        "path": relative,
        "recording_revision": git("log", "-1", "--format=%H", "--", relative) or None,
        "dirty": bool(git("diff", "--name-only", "HEAD", "--", relative)),
    }


class Collector:
    """Accumulates a check invocation's plan and captured output, then writes the receipt."""

    def __init__(self, *, path: Path | str, source: dict, baseline_path: Path | None = None,
                 baseline_root: Path | None = None, base_path: Path | None = None):
        self.path = Path(path)
        self.run_id = uuid.uuid4().hex
        self.source = dict(source)
        self.baseline_path = Path(baseline_path) if baseline_path else None
        self.baseline_root = Path(baseline_root) if baseline_root else None
        self.base_path = Path(base_path) if base_path else None
        self.plan: list[tuple[str, list[str]]] = []
        self.observations: list[Observation] = []
        self.native_inputs: tuple[list[str], bool] | None = None
        self.page_before: dict[str, tuple | None] = {}
        self.page_active: dict[str, set[str]] = {}
        self.page_ambiguous: set[str] = set()
        self.page_results: dict[str, dict | None] = {}
        self.build_bindings = []
        self.lock = threading.RLock()

    def plan_commands(self, plan: list[tuple[str, list[list[str]]]]) -> None:
        self.plan = [(step, list(command)) for step, commands in plan for command in commands]

    def observe(self, step: str, command: list[str], exit_code: int, elapsed: float,
                output: str = "", *, started_at: str | None = None,
                ended_at: str | None = None, attempt_id: str | None = None) -> None:
        with self.lock:
            self._observe(step, command, exit_code, elapsed, output, started_at=started_at,
                          ended_at=ended_at, attempt_id=attempt_id)

    def recording_failed(self, attempt_id, exit_code, elapsed, started_at, ended_at):
        with self.lock:
            item = next(item for item in self.observations if item.attempt_id == attempt_id)
            if item.status == "ran":
                return
            item.status = "recording_failed"
            item.exit_code = exit_code
            item.elapsed = elapsed
            item.started_at = started_at
            item.ended_at = ended_at

    def _observe(self, step, command, exit_code, elapsed, output, *, started_at, ended_at, attempt_id):
        pending = next((item for item in self.observations if item.attempt_id == attempt_id), None) if attempt_id else None
        if pending is not None and pending.status == "ran" and pending.step == step and pending.command == command:
            return
        if attempt_id and (pending is None or pending.status not in ("interrupted", "recording_failed") or
                           pending.step != step or pending.command != command):
            raise ValueError("unknown or mismatched attempt")
        report = None
        captured = False
        if scope_of(command) == PAGE_SUITE and (path := _result_json_path(command)):
            after = self._file_state(path)
            report = load_json(path) if attempt_id not in self.page_ambiguous and after is not None and after != self.page_before.get(attempt_id) else None
            if not isinstance(report, dict) or report.get("run_id") != self.run_id or report.get("attempt_id") != attempt_id:
                report = None
            self.page_active.get(path, set()).discard(attempt_id)
            captured = True
        item = Observation(step, list(command), "ran", exit_code, float(elapsed),
                           output or "", started_at, ended_at, attempt_id or uuid.uuid4().hex,
                           pending.retry_of if pending else None, report, captured)
        if pending is not None:
            self.observations[self.observations.index(pending)] = item
        else:
            self.observations.append(item)

    @staticmethod
    def _file_state(path: str) -> tuple | None:
        try:
            stat = Path(path).stat()
            return stat.st_ino, stat.st_mtime_ns, stat.st_ctime_ns, stat.st_size
        except OSError:
            return None

    def prepare(self, command: list[str], *, step: str | None = None,
                retry_of: str | None = None) -> str:
        with self.lock:
            return self._prepare(command, step=step, retry_of=retry_of)

    def _prepare(self, command, *, step, retry_of):
        if retry_of is not None and not any(item.attempt_id == retry_of and item.step == step
                                            and item.command == command for item in self.observations):
            raise ValueError("retry parent does not match an earlier attempt")
        attempt_id = uuid.uuid4().hex
        if step is not None:
            self.observations.append(Observation(step, list(command), status="interrupted",
                                                 attempt_id=attempt_id, retry_of=retry_of))
        if scope_of(command) == PAGE_SUITE and (path := _result_json_path(command)):
            self.page_before[attempt_id] = self._file_state(path)
            active = self.page_active.setdefault(path, set())
            if active:
                self.page_ambiguous.update(active | {attempt_id})
            active.add(attempt_id)
        return attempt_id

    def record_native(self, outputs: list[str], *, full: bool) -> None:
        self.native_inputs = (list(outputs), full)

    def _native(self) -> dict | None:
        if self.native_inputs is None:
            outputs = [item.output for item in self.observations if scope_of(item.command) == NATIVE]
            if not outputs:
                return None
            full = False
        else:
            outputs, full = self.native_inputs
        baseline = provenance = None
        if self.baseline_path is not None:
            try:
                baseline = check_native_tests.load_baseline(self.baseline_path)
            except (OSError, ValueError):
                baseline = None
            if baseline is not None and self.baseline_root is not None:
                provenance = baseline_provenance(self.baseline_path, self.baseline_root)
        return native_record(outputs, full=full, baseline=baseline, provenance=provenance)

    def finish(self, exit_code: int, *, partial: bool = False,
               source_after: dict | None = None) -> dict:
        base = load_json(str(self.base_path)) if self.base_path else None
        built = build_receipt(self.plan, self.observations, source=self.source, exit_code=exit_code,
                              partial=partial, native=self._native(), base=base, base_path=self.base_path,
                              page_loader=lambda path: self.page_results.get(path),
                              run_id=self.run_id, source_after=source_after)
        built["build_bindings"] = list(self.build_bindings)
        built["drag_bindings"] = []
        for item in self.observations:
            if "scripts/run-drag-browser.py" not in item.command:
                continue
            observed = None
            if item.status == "ran" and item.exit_code == 0:
                lines = [line.removeprefix("BOARDSTUDIO_DRAG_BINDING ") for line in item.output.splitlines()
                         if line.startswith("BOARDSTUDIO_DRAG_BINDING ")]
                if len(lines) == 1:
                    try:
                        observed = json.loads(lines[0])
                    except (ValueError, TypeError):
                        pass
            earlier = self.observations[:self.observations.index(item)]
            preceding = {entry.attempt_id for entry in earlier}
            builds = [entry for entry in self.build_bindings if entry.get("attempt_id") in preceding]
            built["drag_bindings"].append(drag_binding(self.run_id, item.attempt_id, observed, builds))
        write_receipt(self.path, built)
        return built
