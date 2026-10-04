#!/usr/bin/env python3
"""Fail when a plain #[test] sits in Rust code that native `cargo test` never compiles.

The web crate gates much of `mod presentation` behind target_arch = "wasm32".
Native `cargo test` only reaches the files that main.rs/lib.rs (or web/tests)
include through non-wasm cfgs or `#[path]` stubs, so a plain `#[test]` in a
wasm-only file silently runs zero tests. Use `#[wasm_bindgen_test]` there, or
include the file natively.

The module tree is resolved with regexes (mod / #[path] / #[cfg] / #![cfg]) and
a small cfg evaluator. Native: target_arch != wasm32, test on, every cargo
feature on. Wasm: target_arch == wasm32, test on, every feature on. Unknown cfg
predicates evaluate true, which keeps the lint conservative (fewer findings).

Existing debt is ratcheted through scripts/check-wasm-tests-baseline.json
({file: known count}). A file fails when its count exceeds the baseline (every
offending line is listed); counts may only go down. Regenerate with
--write-baseline only after deliberately accepting or fixing debt.

Usage: check-wasm-tests.py [--root REPO_ROOT] [--baseline FILE] [--write-baseline]
"""

import argparse
import json
import re
import sys
from pathlib import Path


TOKEN = re.compile(
    r"(?P<attr>\#\s*(?P<inner>!?)\s*\[[^\]]*\])"
    r"|(?P<mod>(?:\bpub(?:\s*\([^)]*\))?\s+)?\bmod\s+(?P<name>[A-Za-z_]\w*)\s*(?P<end>[;{]))"
    r"|(?P<open>\{)|(?P<close>\})|(?P<semi>;)"
)
STRING = re.compile(r'b?r(?P<h>#*)"(?:.|\n)*?"(?P=h)|b?"(?:\\.|[^"\\])*"', re.S)
CHAR = re.compile(r"'(?:\\.|[^'\\])'")
COMMENT = re.compile(r"//[^\n]*|/\*(?:.|\n)*?\*/", re.S)
MOD_RS_NAMES = {"main.rs", "lib.rs", "mod.rs"}


def clean(source):
    """Blank comments; neutralise brackets/braces/hashes inside literals; keep newlines."""
    def blank(match):
        return re.sub(r"[^\n]", " ", match.group(0))

    def neutralise(match):
        return re.sub(r"[{}\[\]#;]", "_", match.group(0))

    source = COMMENT.sub(blank, source)
    source = STRING.sub(neutralise, source)
    return CHAR.sub(neutralise, source)


def cfg_tokens(text):
    return re.findall(r'"[^"]*"|[A-Za-z_]\w*|[(),=]', text)


def eval_cfg(expression, env):
    tokens = cfg_tokens(expression)
    position = 0

    def parse():
        nonlocal position
        name = tokens[position]
        position += 1
        if position < len(tokens) and tokens[position] == "(":
            position += 1
            values = []
            while tokens[position] != ")":
                values.append(parse())
                if tokens[position] == ",":
                    position += 1
            position += 1
            if name == "all":
                return all(values)
            if name == "any":
                return any(values)
            if name == "not":
                return not values[0]
            return True
        if position < len(tokens) and tokens[position] == "=":
            value = tokens[position + 1].strip('"')
            position += 2
            if name == "feature":
                return True
            known = env.get(name)
            return True if known is None else known == value
        known = env.get(name, True)
        return known if isinstance(known, bool) else True

    try:
        return parse()
    except IndexError:
        return True


def attr_cfg(attr):
    match = re.match(r"\#\s*!?\s*\[\s*cfg\s*\((.*)\)\s*\]\s*$", attr, re.S)
    return match.group(1) if match else None


def attr_path(attr):
    match = re.match(r'\#\s*\[\s*path\s*=\s*"([^"]*)"\s*\]\s*$', attr)
    return match.group(1) if match else None


def parse_file(path):
    """Return inner cfgs, file-module declarations and plain tests of one source file."""
    raw = path.read_text(errors="replace")
    source = clean(raw)
    inner, mods, tests = [], [], []
    pending = []
    stack = []  # one entry per open brace: ("mod", name, cfgs) or ("block",)

    def inline_names():
        return [entry[1] for entry in stack if entry[0] == "mod"]

    def inline_cfgs():
        return [cfg for entry in stack if entry[0] == "mod" for cfg in entry[2]]

    for match in TOKEN.finditer(source):
        line = source.count("\n", 0, match.start()) + 1
        if match.group("attr"):
            attr = re.sub(r"\s+", " ", match.group("attr"))
            if match.group("inner"):
                cfg = attr_cfg(attr)
                if cfg is not None and not stack:
                    inner.append(cfg)
                elif cfg is not None:
                    stack[-1] = (*stack[-1][:2], [*stack[-1][2], cfg]) if stack[-1][0] == "mod" else stack[-1]
            else:
                pending.append(attr)
                if re.fullmatch(r"\#\s*\[\s*test\s*\]", attr):
                    tests.append({"line": line, "cfgs": inline_cfgs() + [
                        cfg for cfg in map(attr_cfg, pending) if cfg is not None]})
            continue
        if match.group("mod"):
            cfgs = [cfg for cfg in map(attr_cfg, pending) if cfg is not None]
            explicit = next((value for value in map(attr_path, pending) if value is not None), None)
            if match.group("end") == ";":
                mods.append({"name": match.group("name"), "cfgs": inline_cfgs() + cfgs,
                             "path": explicit, "inline": inline_names(), "line": line})
            else:
                stack.append(("mod", match.group("name"), cfgs))
            pending = []
            continue
        if match.group("open"):
            stack.append(("block",))
        elif match.group("close") and stack:
            stack.pop()
        pending = []
    return {"inner": inner, "mods": mods, "tests": tests}


def module_dir(path, root_files):
    """Directory in which this file's `mod name;` children resolve (before inline segments)."""
    if path.name in MOD_RS_NAMES or path in root_files:
        return path.parent
    return path.parent / path.stem


def resolve(parent, declaration, root_files):
    base = module_dir(parent, root_files)
    for segment in declaration["inline"]:
        base = base / segment
    if declaration["path"] is not None:
        # #[path] is relative to the directory of the declaring file, plus any inline mods.
        anchor = parent.parent
        for segment in declaration["inline"]:
            anchor = anchor / segment
        return (anchor / declaration["path"]).resolve()
    name = declaration["name"]
    for candidate in (base / f"{name}.rs", base / name / "mod.rs"):
        if candidate.is_file():
            return candidate.resolve()
    return None


def reachable(roots, env, cache):
    """Map each compiled file to its parsed record for one cfg environment."""
    compiled = {}
    root_files = {root for root in roots}
    pending = [root for root in roots if root.is_file()]
    while pending:
        path = pending.pop()
        if path in compiled:
            continue
        if path not in cache:
            cache[path] = parse_file(path)
        record = cache[path]
        if not all(eval_cfg(cfg, env) for cfg in record["inner"]):
            continue
        compiled[path] = record
        for declaration in record["mods"]:
            if not all(eval_cfg(cfg, env) for cfg in declaration["cfgs"]):
                continue
            target = resolve(path, declaration, root_files)
            if target is not None and target.is_file():
                pending.append(target)
    return compiled


NATIVE = {"target_arch": "x86_64", "target_family": "unix", "target_os": "linux", "test": True}
WASM = {"target_arch": "wasm32", "target_family": "wasm", "target_os": "unknown", "test": True}


def _trees(root):
    root = Path(root).resolve()
    src = root / "web/src"
    roots = [src / "main.rs", src / "lib.rs"]
    roots += sorted((root / "web/tests").glob("*.rs"))
    roots += sorted((root / "web/examples").glob("*.rs"))
    roots = [path.resolve() for path in roots if path.is_file()]
    cache = {}
    return root, reachable(roots, NATIVE, cache), reachable(roots, WASM, cache)


def wasm_only_files(root):
    """Repo-relative Rust files compiled for wasm32 but never by native `cargo test`."""
    root, native, wasm = _trees(root)
    return sorted(path.relative_to(root).as_posix() for path in set(wasm) - set(native))


def find_violations(root):
    root, native, wasm = _trees(root)
    native_lines = {
        path: {test["line"] for test in record["tests"]
               if all(eval_cfg(cfg, NATIVE) for cfg in test["cfgs"])}
        for path, record in native.items()
    }
    violations = []
    for path, record in wasm.items():
        for test in record["tests"]:
            if not all(eval_cfg(cfg, WASM) for cfg in test["cfgs"]):
                continue
            if test["line"] in native_lines.get(path, ()):
                continue
            violations.append((path.relative_to(root).as_posix(), test["line"]))
    return sorted(set(violations))


def load_baseline(path):
    try:
        return json.loads(Path(path).read_text())
    except FileNotFoundError:
        return {}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=Path(__file__).resolve().parents[1])
    parser.add_argument("--baseline", help="known-debt JSON (default: scripts/check-wasm-tests-baseline.json)")
    parser.add_argument("--write-baseline", action="store_true", help="record current counts as accepted debt")
    args = parser.parse_args(argv)
    baseline_path = Path(args.baseline) if args.baseline else (
        Path(args.root) / "scripts/check-wasm-tests-baseline.json")
    violations = find_violations(args.root)
    counts = {}
    for relative, _line in violations:
        counts[relative] = counts.get(relative, 0) + 1
    if args.write_baseline:
        baseline_path.write_text(json.dumps(dict(sorted(counts.items())), indent=2) + "\n")
        print(f"check-wasm-tests: wrote {len(counts)} file(s) to {baseline_path}")
        return 0
    baseline = load_baseline(baseline_path)
    failed = 0
    for relative, count in sorted(counts.items()):
        if count <= baseline.get(relative, 0):
            continue
        failed += count - baseline.get(relative, 0)
        for path, line in violations:
            if path == relative:
                print(f"{path}:{line}: plain #[test] is not compiled by native `cargo test` "
                      "(wasm32-only module); use #[wasm_bindgen_test] or include the file natively")
    for relative, known in sorted(baseline.items()):
        if counts.get(relative, 0) < known:
            print(f"check-wasm-tests: note: {relative} improved ({counts.get(relative, 0)} < baseline {known}); "
                  "lower the baseline with --write-baseline", file=sys.stderr)
    if failed:
        print(f"check-wasm-tests: {failed} new plain #[test](s) would run zero times natively",
              file=sys.stderr)
        return 1
    print(f"check-wasm-tests: no new wasm-only plain #[test]s ({sum(counts.values())} baselined)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
