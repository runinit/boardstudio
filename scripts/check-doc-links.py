#!/usr/bin/env python3
"""Fail when a local Markdown link in the authored docs is not portable.

Checks README.md, PRODUCT.md, DESIGN.md and every Markdown file under docs/.
Local destinations must resolve inside the checkout and not be Git-ignored.
Remote URLs, heading anchors and fenced code blocks are ignored.

Usage: check-doc-links.py [ROOT]
"""

from __future__ import annotations

from collections import deque
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


def local_dependencies(root: Path, path: str) -> tuple[dict[str, bool], str | None]:
    dependencies = {}
    current = root
    pending = deque(path.split("/"))
    hops = 0
    try:
        while pending:
            if not current.is_dir():
                return dependencies, "Local link does not resolve"
            part = pending.popleft()
            if part in ("", "."):
                continue
            current = current.parent if part == ".." else current / part
            if not current.is_relative_to(root):
                return dependencies, "Local link points outside checkout"
            if current == root:
                continue
            name = current.relative_to(root).as_posix()
            if current.is_symlink():
                dependencies[name] = False
                hops += 1
                # Linux refuses more than 40 symlink expansions, including cycles.
                if hops > 40:
                    return dependencies, "Local link does not resolve"
                destination = os.readlink(current)
                if Path(destination).is_absolute():
                    reason = ("Local link uses an absolute symlink destination" if Path(destination).is_relative_to(root)
                              else "Local link points outside checkout")
                    return dependencies, reason
                current = current.parent
                pending.extendleft(reversed(destination.split("/")))
            else:
                dependencies[name] = current.is_dir()
        if not current.exists():
            return dependencies, "Local link does not resolve"
    except OSError:
        return dependencies, "Local link does not resolve"
    return dependencies, None


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
            path = f"{file.parent.relative_to(root).as_posix()}/{target}"
            dependencies, reason = local_dependencies(root, path)
            local_links.append((prefix, link, dependencies, reason))
    if local_links:
        paths = sorted({path for _, _, dependencies, _ in local_links for path in dependencies})
        ignored = set()
        if paths:
            result = subprocess.run(["git", "-C", str(root), "check-ignore", "--stdin", "-z"],
                                    input="\0".join(paths) + "\0", capture_output=True, text=True)
            if result.returncode not in (0, 1):
                result.check_returncode()
            ignored = set(result.stdout.split("\0"))
        ignored_directories = {path for _, _, dependencies, _ in local_links
                               for path, directory in dependencies.items() if directory and path in ignored}
        if ignored_directories:
            # Git tracks files, so an ignored directory can still contain portable tracked content.
            tracked = subprocess.run(["git", "-C", str(root), "ls-files", "--cached", "-z"],
                                     capture_output=True, text=True, check=True)
            tracked_directories = {parent.as_posix() for path in tracked.stdout.split("\0") if path
                                   for parent in Path(path).parents}
            ignored -= ignored_directories & tracked_directories
        for prefix, link, dependencies, reason in local_links:
            if reason and ("outside checkout" in reason or "absolute" in reason):
                issues.append(f"{prefix}{reason}: {link}")
            elif ignored.intersection(dependencies):
                issues.append(f"{prefix}Local link depends on a Git-ignored path: {link}")
            elif reason:
                issues.append(f"{prefix}{reason}: {link}")
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
