#!/usr/bin/env python3
"""One logged cold run: wipe this worktree's build outputs, then run check.py.

Usage: cold.py <name> [--no-wipe] [--without-cache] [check.py steps...]
Logs land in .evidence/cold-build-port/<name>.{log,jsonl} regardless of the caller's cwd.
Refuses to run when the wipe would not be a comparable cold run.
"""
import os
import resource
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
EVIDENCE = ROOT / ".evidence" / "cold-build-port"
WIPE = [
    "target", "web/target", "cad/wasm/target", "cad/monstertruck/target",
    "cad/monstertruck-qualification/target", "tools/browser-harness/target",
    "cad/wasm/pkg", "renderer/pkg", "cad/.cache",
    "web/assets/cad", "web/assets/cad-worker", "web/assets/core-worker",
    # web/target also covers all three worker outputs, including the offline worker.
    "web/assets/ergogen-models", "web/assets/renderer", "web/assets/tailwind.css",
]
TARGETS = [p for p in WIPE if p.endswith("target")]
LOAD_LIMIT = 5.0


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


def configured_wrapper(env):
    for name in ("RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER"):
        if name in env:
            return env[name]
    cargo_home = Path(env.get("CARGO_HOME", Path.home() / ".cargo"))
    for name in ("config", "config.toml"):
        path = cargo_home / name
        if path.is_file():
            with path.open("rb") as config:
                return tomllib.load(config).get("build", {}).get("rustc-wrapper", "")
    return ""


def main(argv):
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    name, steps = argv[0], argv[1:]
    no_wipe = "--no-wipe" in steps
    without_cache = "--without-cache" in steps
    steps = [s for s in steps if s not in ("--no-wipe", "--without-cache")]
    if Path(name).name != name or name in (".", ".."):
        print("cold: run name must be a filename", file=sys.stderr)
        return 2
    lock = os.environ.get("BOARDSTUDIO_BUILD_LOCK_FD")
    if not lock:
        print("cold: acquire BOARDSTUDIO_BUILD_LOCK_FD before running", file=sys.stderr)
        return 2
    os.fstat(int(lock))
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
               BUILDTIME_NEXT_WRAPPER="" if without_cache else configured_wrapper(os.environ),
               RUSTC_WRAPPER=str(ROOT / "scripts" / "build-time" / "rustc-log.py"))
    env.pop("BUILDTIME_STEP", None)
    with open(EVIDENCE / f"{name}.log", "w") as log:
        start = time.monotonic()
        code = subprocess.run([sys.executable, "scripts/check.py", *steps],
                              cwd=ROOT, env=env, pass_fds=(int(lock),),
                              stdout=log, stderr=subprocess.STDOUT).returncode
        usage = resource.getrusage(resource.RUSAGE_CHILDREN)
        print(f"cold: wall {time.monotonic() - start:.1f}s, "
              f"process tree CPU {usage.ru_utime + usage.ru_stime:.1f}s", file=log)
    print(f"exit {code}")
    for line in (EVIDENCE / f"{name}.log").read_text(errors="replace").splitlines():
        if line.startswith(("check: step times", "check: all steps", "cold: wall")) or "FAILED" in line:
            print(line)
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
