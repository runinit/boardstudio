#!/usr/bin/env python3
"""Fail when a Core test file is not compiled by any test target.

`core/Cargo.toml` sets `autotests = false`, so a file dropped into `core/tests/` runs only when
a `[[test]]` target names it or `core/tests/integration.rs` pulls it in with `#[path = "..."]`.
An unregistered file passes every check and never runs.

Usage: check-core-tests.py [ROOT]
"""

from __future__ import annotations

from pathlib import Path
import re
import sys
import tomllib

PATH_ATTRIBUTE = re.compile(r'#\[path\s*=\s*"([^"]+)"\s*\]')


def registered(root: Path) -> set[str]:
    core = root / "core"
    manifest = tomllib.loads((core / "Cargo.toml").read_text(encoding="utf-8"))
    names = {Path(target["path"]).as_posix() for target in manifest.get("test", []) if "path" in target}
    integration = core / "tests" / "integration.rs"
    if integration.is_file():
        names |= {f"tests/{match}" for match in PATH_ATTRIBUTE.findall(integration.read_text(encoding="utf-8"))}
    return names


def unregistered_tests(root: Path) -> list[str]:
    known = registered(root)
    issues = []
    for file in sorted((root / "core" / "tests").glob("*.rs")):
        if f"tests/{file.name}" not in known:
            issues.append(
                f"unregistered-core-test: core/tests/{file.name} — never compiled; add "
                f'`#[path = "{file.name}"] mod {file.stem};` to core/tests/integration.rs or a [[test]] target'
            )
    return issues


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    root = Path(arguments[0]).resolve() if arguments else Path(__file__).resolve().parents[1]
    issues = unregistered_tests(root)
    for issue in issues:
        print(issue, file=sys.stderr)
    if issues:
        return 1
    print(f"Core test registration check passed: {len(list((root / 'core' / 'tests').glob('*.rs')))} files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
