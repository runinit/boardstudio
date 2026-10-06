#!/usr/bin/env python3
"""Fail when a local Markdown link in the authored docs does not resolve.

Checks README.md, PRODUCT.md, DESIGN.md and every Markdown file under docs/.
Only file destinations are checked: remote URLs and heading anchors are not,
and fenced code blocks are ignored.

Usage: check-doc-links.py [ROOT]
"""

from __future__ import annotations

from pathlib import Path
import re
import sys
from urllib.parse import unquote

EXCLUDED = {"node_modules", "dist", "pkg", "target", ".git", ".impeccable", ".generated", "library"}
ROOT_DOCUMENTS = ("README.md", "PRODUCT.md", "DESIGN.md")
FENCE = re.compile(r"```[\s\S]*?```")
LINK = re.compile(r'\[[^\]]*\]\((?:<([^>]+)>|([^\s)]+))(?:\s+"[^"]*")?\)')
EXTERNAL = re.compile(r"^(?:[a-z][a-z\d+.-]*:|#)", re.IGNORECASE)


def markdown_files(root: Path) -> list[Path]:
    files = [root / name for name in ROOT_DOCUMENTS]

    def walk(directory: Path):
        if not directory.is_dir():
            return
        for entry in sorted(directory.iterdir()):
            if entry.is_dir():
                if entry.name not in EXCLUDED:
                    yield from walk(entry)
            elif entry.is_file() and entry.suffix == ".md":
                yield entry

    return files + list(walk(root / "docs"))


def broken_links(root: Path) -> list[str]:
    issues = []
    for file in markdown_files(root):
        try:
            source = file.read_text(encoding="utf-8")
        except FileNotFoundError:
            continue
        for match in LINK.finditer(FENCE.sub("", source)):
            link = match.group(1) or match.group(2)
            if EXTERNAL.match(link):
                continue
            target = unquote(link.split("#", 1)[0])
            if not (file.parent / target).exists():
                issues.append(f"broken-markdown-link: {file.relative_to(root).as_posix()} — Local link does not resolve: {link}")
    return issues


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    root = Path(arguments[0]).resolve() if arguments else Path(__file__).resolve().parents[1]
    issues = broken_links(root)
    for issue in issues:
        print(issue, file=sys.stderr)
    if issues:
        return 1
    print(f"Documentation links check passed: {len(markdown_files(root))} Markdown files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
