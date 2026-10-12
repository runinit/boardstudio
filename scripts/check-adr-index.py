#!/usr/bin/env python3
"""Fail when an ADR in docs/adr/ is not listed in docs/adr/README.md.

The index is how a reader (or an agent) sees the whole set of decisions; an ADR that only a
plan happens to link is effectively hidden.

Usage: check-adr-index.py [ROOT]
"""

from __future__ import annotations

from pathlib import Path
import re
import sys

ADRS = Path("docs/adr")
INDEX = ADRS / "README.md"
NAME = re.compile(r"^\d{4}-.+\.md$")


def unlisted(root: Path) -> list[str]:
    index = root / INDEX
    listing = index.read_text(encoding="utf-8") if index.is_file() else ""
    return [
        f"unlisted-adr: {ADRS.as_posix()}/{file.name} — not named in {INDEX.as_posix()}; add a row for it"
        for file in sorted((root / ADRS).glob("*.md"))
        if NAME.match(file.name) and not re.search(rf"(?<![\w.-]){re.escape(file.name)}", listing)
    ]


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    root = Path(arguments[0]).resolve() if arguments else Path(__file__).resolve().parents[1]
    messages = unlisted(root)
    for message in messages:
        print(message, file=sys.stderr)
    if messages:
        return 1
    print(f"ADR index check passed: {len([f for f in (root / ADRS).glob('*.md') if NAME.match(f.name)])} ADRs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
