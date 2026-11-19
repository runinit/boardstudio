#!/usr/bin/env python3
"""Fail when a Core or Footprints test file is not compiled by any test target.

Both packages set `autotests = false`, so a file dropped into their tests directory runs only when
a `[[test]]` target names it or `tests/integration.rs` declares it as a module, with
`#[path = "..."]` or a plain `mod name;`. An unregistered file passes every check and never runs.

Usage: check-core-tests.py [ROOT]
"""

from __future__ import annotations

from pathlib import Path
import re
import sys
import tomllib

PACKAGES = ("core", "footprints")
# An optional `#[path = "..."]` directly before a `mod name;` declaration.
MODULE = re.compile(r'(?:#\[path\s*=\s*"([^"]+)"\s*\]\s*)?(?:pub\s+)?mod\s+(\w+)\s*;')
COMMENT = re.compile(r"//[^\n]*|/\*.*?\*/", re.DOTALL)


def test_files(package: Path) -> list[Path]:
    return sorted((package / "tests").glob("*.rs"))


def registered(package: Path) -> set[Path]:
    manifest = tomllib.loads((package / "Cargo.toml").read_text(encoding="utf-8"))
    paths = {Path(target.get("path", f"tests/{target['name']}.rs")) for target in manifest.get("test", [])}
    integration = package / "tests" / "integration.rs"
    if integration.is_file():
        source = COMMENT.sub("", integration.read_text(encoding="utf-8"))
        paths |= {Path("tests") / (path or f"{name}.rs") for path, name in MODULE.findall(source)}
    return {package / path for path in paths}


def unregistered_in_package(package: Path) -> list[str]:
    tests = f"{package.name}/tests"
    known = registered(package)
    return [
        f"unregistered-{package.name}-test: {tests}/{file.name} — never compiled; add "
        f'`#[path = "{file.name}"] mod {file.stem};` to {tests}/integration.rs or a [[test]] target'
        for file in test_files(package)
        if file not in known
    ]


def unregistered_tests(root: Path) -> list[str]:
    return [message for package in PACKAGES for message in unregistered_in_package(root / package)]


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    root = Path(arguments[0]).resolve() if arguments else Path(__file__).resolve().parents[1]
    messages = unregistered_tests(root)
    for message in messages:
        print(message, file=sys.stderr)
    if messages:
        return 1
    for package in PACKAGES:
        print(f"{package.capitalize()} test registration check passed: {len(test_files(root / package))} files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
