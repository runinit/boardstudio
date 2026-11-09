#!/usr/bin/env python3
"""Count the old-CSS names still used under web/: .board-* and .m1-* selectors and --wb-* variables.

Counts distinct names in every text file under web/ (styles and Rust sources alike). The default run
only reports; --strict exits nonzero while any remain, which gates the final merge (branch plan section 4).

Usage: check-old-css.py [--strict] [ROOT]
"""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys

SELECTOR = re.compile(r"(?<![\w-])\.(?:board|m1)-[\w-]+")
VARIABLE = re.compile(r"(?<![\w-])--wb-[\w-]+")
BUILD_OUTPUT = {"target", "node_modules", "pkg", "dist"}


def names(root: Path) -> tuple[set[str], set[str]]:
    selectors: set[str] = set()
    variables: set[str] = set()
    for path in (root / "web").rglob("*"):
        if not path.is_file() or BUILD_OUTPUT & set(path.relative_to(root).parts):
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        selectors.update(SELECTOR.findall(text))
        variables.update(VARIABLE.findall(text))
    return selectors, variables


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--strict", action="store_true", help="fail while any old-CSS name remains")
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args(argv)
    selectors, variables = names(args.root.resolve())
    print(f"old-css: {len(selectors)} .board-*/.m1-* selectors and {len(variables)} --wb-* variables remain.")
    if args.strict and (selectors or variables):
        print("old-css: strict mode fails until every old-CSS name is gone.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
