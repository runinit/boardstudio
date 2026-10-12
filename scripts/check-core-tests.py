#!/usr/bin/env python3
"""Fail when a Core test file is not compiled by any test target.

`core/Cargo.toml` sets `autotests = false`, so a file dropped into `core/tests/` runs only when
a `[[test]]` target names it or `core/tests/integration.rs` declares it as a module, with
`#[path = "..."]` or a plain `mod name;`. An unregistered file passes every check and never runs.

Usage: check-core-tests.py [ROOT]
"""

from __future__ import annotations

from pathlib import Path
import re
import sys
import tomllib

CORE = Path("core")
TESTS = CORE / "tests"
# An optional `#[path = "..."]` directly before a `mod name;` declaration.
MODULE = re.compile(r'(?:#\[path\s*=\s*"([^"]+)"\s*\]\s*)?(?:pub\s+)?mod\s+(\w+)\s*;')
COMMENT = re.compile(r"//[^\n]*|/\*.*?\*/", re.DOTALL)


def test_files(root: Path) -> list[Path]:
    return sorted((root / TESTS).glob("*.rs"))


def registered(root: Path) -> set[Path]:
    manifest = tomllib.loads((root / CORE / "Cargo.toml").read_text(encoding="utf-8"))
    paths = {Path(target.get("path", f"tests/{target['name']}.rs")) for target in manifest.get("test", [])}
    integration = root / TESTS / "integration.rs"
    if integration.is_file():
        source = COMMENT.sub("", integration.read_text(encoding="utf-8"))
        paths |= {Path("tests") / (path or f"{name}.rs") for path, name in MODULE.findall(source)}
    return {root / CORE / path for path in paths}


def unregistered_tests(root: Path) -> list[str]:
    known = registered(root)
    return [
        f"unregistered-core-test: {TESTS.as_posix()}/{file.name} — never compiled; add "
        f'`#[path = "{file.name}"] mod {file.stem};` to {TESTS.as_posix()}/integration.rs or a [[test]] target'
        for file in test_files(root)
        if file not in known
    ]


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    root = Path(arguments[0]).resolve() if arguments else Path(__file__).resolve().parents[1]
    messages = unregistered_tests(root)
    for message in messages:
        print(message, file=sys.stderr)
    if messages:
        return 1
    print(f"Core test registration check passed: {len(test_files(root))} files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
