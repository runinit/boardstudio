#!/usr/bin/env python3
"""Fail when a ticket has an unknown Status or is missing from the map.

Tickets are `docs/plans/typed-core-edits/issues/*.md`; the vocabulary and the listing rule are
in docs/agents/issue-tracker.md. Tickets that predate the check are recorded in
scripts/check-tickets-baseline.json, so only new violations fail. An entry that no longer
applies fails too, which keeps the baseline shrinking.

Usage: check-tickets.py [--update-baseline] [ROOT]
"""

from __future__ import annotations

import json
from pathlib import Path
import re
import sys

STATUSES = ("needs-triage", "needs-info", "ready-for-agent", "ready-for-human", "claimed", "resolved", "wontfix")
STATUS, LISTED = "status", "listed"  # violation codes, as recorded in the baseline
TICKETS = Path("docs/plans/typed-core-edits/issues")
MAP = Path("docs/plans/typed-core-edits/map.md")
BASELINE = Path("scripts/check-tickets-baseline.json")
HEADER_LINES = 12


def status_of(text: str) -> str | None:
    for line in text.splitlines()[:HEADER_LINES]:
        if line.startswith("Status:"):
            return line.removeprefix("Status:").strip()
    return None


def violations(root: Path) -> dict[str, set[str]]:
    listing = (root / MAP).read_text(encoding="utf-8")
    found: dict[str, set[str]] = {}
    for file in sorted((root / TICKETS).glob("*.md")):
        codes = set()
        if status_of(file.read_text(encoding="utf-8")) not in STATUSES:
            codes.add(STATUS)
        # The name must stand alone: `19-x.md` does not list `9-x.md`.
        if not re.search(rf"(?<![\w.-]){re.escape(file.name)}", listing):
            codes.add(LISTED)
        if codes:
            found[file.name] = codes
    return found


def describe(root: Path, name: str, code: str) -> str:
    if code == STATUS:
        status = status_of((root / TICKETS / name).read_text(encoding="utf-8"))
        shown = f"no Status line in the first {HEADER_LINES} lines" if status is None else f"unknown Status {status!r}"
        return f"ticket-status: {TICKETS.as_posix()}/{name} — {shown}; use one of {', '.join(STATUSES)}"
    return f"ticket-unlisted: {TICKETS.as_posix()}/{name} — not named in {MAP.as_posix()}; add a link to it"


def problems(root: Path, baseline: dict[str, list[str]]) -> list[str]:
    found = violations(root)
    messages = []
    for name, codes in found.items():
        for code in sorted(codes - set(baseline.get(name, []))):
            messages.append(describe(root, name, code))
    for name, codes in sorted(baseline.items()):
        for code in sorted(set(codes) - found.get(name, set())):
            messages.append(
                f"stale-baseline: {name} no longer has {code!r}; "
                "run `python3 scripts/check-tickets.py --update-baseline` and review the diff"
            )
    return messages


def main(argv: list[str] | None = None) -> int:
    arguments = list(sys.argv[1:] if argv is None else argv)
    update = "--update-baseline" in arguments
    arguments = [argument for argument in arguments if argument != "--update-baseline"]
    root = Path(arguments[0]).resolve() if arguments else Path(__file__).resolve().parents[1]
    path = root / BASELINE
    if update:
        found = {name: sorted(codes) for name, codes in violations(root).items()}
        path.write_text(json.dumps(found, indent=1, sort_keys=True) + "\n", encoding="utf-8")
        print(f"Recorded {len(found)} tickets in {BASELINE.as_posix()}.")
        return 0
    baseline = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
    messages = problems(root, baseline)
    for message in messages:
        print(message, file=sys.stderr)
    if messages:
        return 1
    print(f"Ticket check passed: {len(list((root / TICKETS).glob('*.md')))} tickets.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
