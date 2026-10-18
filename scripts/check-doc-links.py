#!/usr/bin/env python3
"""Fail when a local Markdown link in the authored docs is not portable.

Checks README.md, PRODUCT.md, DESIGN.md and every Markdown file under docs/.
Local destinations must resolve inside the checkout and not be Git-ignored.
Remote URLs, heading anchors and fenced code blocks are ignored.

Usage: check-doc-links.py [ROOT]
"""

from __future__ import annotations

import os
from pathlib import Path, PureWindowsPath
import re
import subprocess
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
    root = root.resolve()
    issues = []
    local_links = []
    for file in markdown_files(root):
        try:
            source = file.read_text(encoding="utf-8")
        except FileNotFoundError:
            continue
        for match in LINK.finditer(FENCE.sub("", source)):
            link = match.group(1) or match.group(2)
            target = unquote(link.split("#", 1)[0])
            prefix = f"broken-markdown-link: {file.relative_to(root).as_posix()} — "
            if Path(target).is_absolute() or PureWindowsPath(target).is_absolute() or target.lower().startswith("file:"):
                issues.append(f"{prefix}Local link uses an absolute destination: {link}")
                continue
            if EXTERNAL.match(link):
                continue
            lexical = Path(os.path.abspath(file.parent / target))
            resolved = (file.parent / target).resolve()
            if not lexical.is_relative_to(root) or not resolved.is_relative_to(root):
                issues.append(f"{prefix}Local link points outside checkout: {link}")
                continue
            # Git refuses paths beyond a symlink; check the link itself and its destination.
            git_path = lexical
            for parent in reversed((lexical, *lexical.parents)):
                if parent.is_relative_to(root) and parent.is_symlink():
                    git_path = parent
                    break
            local_links.append((prefix, link, resolved, git_path.relative_to(root).as_posix()))
    if local_links:
        paths = sorted({path for _, _, resolved, path in local_links}
                       | {resolved.relative_to(root).as_posix() for _, _, resolved, _ in local_links})
        result = subprocess.run(["git", "-C", str(root), "check-ignore", "--stdin", "-z"],
                                input="\0".join(paths) + "\0", capture_output=True, text=True)
        if result.returncode not in (0, 1):
            result.check_returncode()
        ignored = set(result.stdout.split("\0"))
        for prefix, link, resolved, path in local_links:
            if path in ignored or resolved.relative_to(root).as_posix() in ignored:
                issues.append(f"{prefix}Local link depends on a Git-ignored path: {link}")
            elif not resolved.exists():
                issues.append(f"{prefix}Local link does not resolve: {link}")
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
