#!/usr/bin/env python3
"""Mechanical LOC, deletion-manifest, and forbidden-symbol gate for clean-slate reduction tracks."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import Counter
from functools import cache
from pathlib import PurePosixPath


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True)


def _strip_rust_comments_and_strings(src: str) -> str:
    """Fast lexer-free stripping of line/block comments and string literals preserving newlines."""
    out: list[str] = []
    i = 0
    n = len(src)
    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = src.find("\n", i)
            if j == -1:
                break
            out.append(" " * (j - i) + "\n")
            i = j + 1
        elif c == "/" and nxt == "*":
            depth = 1
            j = i + 2
            while j < n and depth > 0:
                if src[j : j + 2] == "/*":
                    depth += 1
                    j += 2
                elif src[j : j + 2] == "*/":
                    depth -= 1
                    j += 2
                else:
                    out.append("\n" if src[j] == "\n" else " ")
                    j += 1
            i = j
        elif c == '"':
            out.append(" ")
            i += 1
            while i < n:
                if src[i] == "\\":
                    out.append("  ")
                    i += 2
                elif src[i] == '"':
                    out.append(" ")
                    i += 1
                    break
                else:
                    out.append("\n" if src[i] == "\n" else " ")
                    i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


@cache
def _test_lines_in_file(ref: str, path: str) -> frozenset[int]:
    if not path.endswith(".rs"):
        return frozenset()
    src = git("show", f"{ref}:{path}")
    lines = _strip_rust_comments_and_strings(src).splitlines()
    result: set[int] = set()
    for i, line in enumerate(lines):
        s = line.strip()
        is_cfg_test = s.startswith("#[cfg(") and bool(re.search(r"\btest\b", s)) and not bool(
            re.search(r"\b(?:not|any)\(", s)
        )
        is_test_attr = s.startswith(("#[test]", "#[wasm_bindgen_test", "#[rstest]"))
        if not (is_cfg_test or is_test_attr):
            continue
        depth = 0
        begun = False
        for j in range(i + 1, len(lines)):
            cur = lines[j].strip()
            if not begun and (not cur or cur.startswith("#[")):
                continue
            depth += cur.count("{") - cur.count("}")
            begun = begun or ("{" in cur)
            if depth <= 0 and (begun or ";" in cur):
                result.update(range(i + 1, j + 2))
                break
    return frozenset(result)


def _category(ref: str, path: str, line_no: int) -> str:
    p = PurePosixPath(path)
    if path.endswith(".rs"):
        is_test_path = bool(
            set(p.parts) & {"tests", "test", "fixtures", "test-runtime", "benches"}
            or re.search(r"(^|[/_])(tests?|fixtures?|test_support|support_tests)([/_.]|$)", path)
        )
        return "test_rust" if (is_test_path or line_no in _test_lines_in_file(ref, path)) else "prod_rust"
    if path.startswith("scripts/"):
        return "scripts"
    return "other"


def compute_deltas(base: str, head: str) -> dict[str, tuple[int, int, int]]:
    counts: Counter[tuple[str, str]] = Counter()
    diff = git("diff", "--no-renames", "--no-color", "--unified=0", base, head)
    old = new = ""
    i = j = 0
    for line in diff.splitlines():
        if line.startswith("diff --git "):
            a, b = line.split(" b/", 1)
            old = a.removeprefix("diff --git a/")
            new = b
        elif line.startswith("@@ "):
            m = re.match(r"^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@", line)
            assert m is not None
            i, j = map(int, m.groups())
        elif line.startswith(("--- ", "+++ ")):
            continue
        elif line.startswith("-"):
            counts[_category(base, old, i), "-"] += 1
            i += 1
        elif line.startswith("+"):
            counts[_category(head, new, j), "+"] += 1
            j += 1
    out: dict[str, tuple[int, int, int]] = {}
    for cat in ("prod_rust", "test_rust", "scripts", "other"):
        added, deleted = counts[cat, "+"], counts[cat, "-"]
        out[cat] = (added, deleted, added - deleted)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify clean-slate reduction budgets and deletion manifests.")
    parser.add_argument("--base", required=True, help="Base git ref (e.g. dev or 5aae86ff4).")
    parser.add_argument("--head", default="HEAD", help="Head git ref (default: HEAD).")
    parser.add_argument(
        "--max-prod-delta",
        type=int,
        default=0,
        help="Maximum allowed net Prod Rust LOC delta (default: 0; anti-bloat floor when --advisory-loc is set).",
    )
    parser.add_argument(
        "--max-test-delta",
        type=int,
        default=0,
        help="Maximum allowed net Test Rust LOC delta (default: 0; anti-bloat floor when --advisory-loc is set).",
    )
    parser.add_argument("--max-script-delta", type=int, default=0, help="Maximum allowed net scripts/ LOC delta.")
    parser.add_argument("--max-other-delta", type=int, default=200, help="Maximum allowed net Other LOC delta.")
    parser.add_argument("--target-prod-delta", type=int, default=None, help="Advisory stretch target for net Prod Rust LOC.")
    parser.add_argument("--target-test-delta", type=int, default=None, help="Advisory stretch target for net Test Rust LOC.")
    parser.add_argument("--target-script-delta", type=int, default=None, help="Advisory stretch target for net scripts/ LOC.")
    parser.add_argument(
        "--advisory-loc",
        action="store_true",
        help="Treat negative --max-{prod,test,script}-delta values as advisory targets while hard-enforcing <= 0 anti-bloat.",
    )
    parser.add_argument("--must-delete", action="append", default=[], help="Tracked path/prefix that must have 0 files.")
    parser.add_argument("--forbid-symbol", action="append", default=[], help="Symbol that must not appear in *.rs.")
    args = parser.parse_args()

    def _limit_and_target(max_val: int, explicit_target: int | None, allow_advisory: bool) -> tuple[int, int | None]:
        target = explicit_target
        limit = max_val
        if args.advisory_loc and allow_advisory:
            if target is None and max_val < 0:
                target = max_val
            limit = min(0, max_val) if target is None else 0
        return limit, target

    deltas = compute_deltas(args.base, args.head)
    labels = {
        "prod_rust": ("Production Rust", *_limit_and_target(args.max_prod_delta, args.target_prod_delta, True)),
        "test_rust": ("Test Rust", *_limit_and_target(args.max_test_delta, args.target_test_delta, True)),
        "scripts": ("Scripts", *_limit_and_target(args.max_script_delta, args.target_script_delta, True)),
        "other": ("Other", args.max_other_delta, None),
    }

    failures: list[str] = []
    print(f"Reduction Gate ({args.base} -> {args.head})")
    print("-" * 74)
    for cat, (label, limit, target) in labels.items():
        added, deleted, net = deltas[cat]
        ok = net <= limit
        status = "PASS" if ok else "FAIL"
        if target is not None:
            t_status = "TARGET MET" if net <= target else "ADVISORY"
            print(
                f"{label:18} +{added:7} -{deleted:7} net {net:+8}  "
                f"(limit {limit:+8}, target {target:+8})  [{status} | {t_status}]"
            )
        else:
            print(f"{label:18} +{added:7} -{deleted:7} net {net:+8}  (limit {limit:+8})  [{status}]")
        if not ok:
            failures.append(f"{label} net delta {net:+d} exceeds limit {limit:+d}")

    tracked = git("ls-tree", "-r", "--name-only", args.head).splitlines()
    for prefix in args.must_delete:
        norm = prefix.rstrip("/")
        remaining = [p for p in tracked if p == norm or p.startswith(norm + "/")]
        if remaining:
            failures.append(f"Mandatory deletion still present ({len(remaining)} files): {prefix} -> {remaining[:3]}")
            print(f"MUST-DELETE FAIL: {prefix} still has {len(remaining)} tracked file(s): {remaining[:3]}")

    for sym in args.forbid_symbol:
        res = subprocess.run(["git", "grep", "-l", sym, args.head, "--", "*.rs"], capture_output=True, text=True)
        hits = [line for line in res.stdout.splitlines() if line.strip()]
        if hits:
            failures.append(f"Forbidden symbol '{sym}' found in {len(hits)} file(s): {hits[:3]}")
            print(f"FORBID-SYMBOL FAIL: '{sym}' found in {hits[:3]}")

    print("-" * 74)
    if failures:
        print(f"REDUCTION GATE FAILED ({len(failures)} violation(s)):")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("REDUCTION GATE PASSED.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
