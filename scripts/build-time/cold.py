#!/usr/bin/env python3
"""One logged cold run: wipe this worktree's build outputs, then run check.py.

Usage: cold.py <name> [--no-wipe] [check.py steps...]
Logs land in .evidence/build-time/<name>.{log,jsonl} regardless of the caller's cwd.
Refuses to run when the wipe would not be a comparable cold run.
"""
import os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
EVIDENCE = ROOT / ".evidence" / "build-time"
WIPE = [
    "target", "web/target", "cad/wasm/target", "cad/monstertruck/target",
    "cad/monstertruck-qualification/target", "tools/browser-harness/target",
    "cad/wasm/pkg", "renderer/pkg", "cad/.cache",
    "web/assets/cad", "web/assets/cad-worker", "web/assets/core-worker",
    "web/assets/ergogen-models", "web/assets/renderer",
]
TARGETS = [p for p in WIPE if p.endswith("target")]
LOAD_LIMIT = 3.0


def refusal_reasons(env, islink, loadavg):
    reasons = []
    if env.get("CARGO_TARGET_DIR"):
        reasons.append("CARGO_TARGET_DIR is set; the wipe would miss the real outputs")
    for path in TARGETS:
        if islink(path):
            reasons.append(f"{path} is a symlink; its outputs live outside this worktree")
    if loadavg >= LOAD_LIMIT:
        reasons.append(f"load average {loadavg:.2f} is at or above {LOAD_LIMIT:.0f}; timings would not be comparable")
    return reasons


def main(argv):
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    name, steps = argv[0], argv[1:]
    no_wipe = "--no-wipe" in steps
    steps = [s for s in steps if s != "--no-wipe"]
    reasons = [] if no_wipe else refusal_reasons(os.environ, lambda p: (ROOT / p).is_symlink(), os.getloadavg()[0])
    if reasons:
        for reason in reasons:
            print(f"cold: refusing to run: {reason}", file=sys.stderr)
        return 2
    for rel in [] if no_wipe else WIPE:
        path = ROOT / rel
        if path.is_symlink() or path.is_file():
            path.unlink()
        elif path.is_dir():
            shutil.rmtree(path)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    jsonl = EVIDENCE / f"{name}.jsonl"
    jsonl.unlink(missing_ok=True)
    env = dict(os.environ, BUILDTIME_LOG=str(jsonl),
               RUSTC_WRAPPER=str(ROOT / "scripts" / "build-time" / "rustc-log.py"))
    with open(EVIDENCE / f"{name}.log", "w") as log:
        code = subprocess.run([sys.executable, "scripts/check.py", *steps],
                              cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT).returncode
    print(f"exit {code}")
    for line in (EVIDENCE / f"{name}.log").read_text(errors="replace").splitlines():
        if line.startswith(("check: step times", "check: all steps")) or "FAILED" in line:
            print(line[:220])
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
