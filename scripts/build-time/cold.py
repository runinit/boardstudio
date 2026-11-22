#!/usr/bin/env python3
"""One logged cold run: wipe this worktree's build outputs, then run check.py.

Usage: cold.py <name> [--no-wipe] [--without-cache] [check.py steps...]
Logs land in .evidence/cold-build-port/<name>.{log,jsonl} regardless of the caller's cwd.
Refuses to run when the wipe would not be a comparable cold run.

For warm runs, seed the sccache daemon with the exact same check.py steps on an
empty target/ first: units already fresh in target/ during the seed are skipped
by cargo, never stored, and will miss in the warm run.
"""
import os
import resource
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import browser_tools

ROOT = Path(__file__).resolve().parent.parent.parent
EVIDENCE = ROOT / ".evidence" / "cold-build-port"
WIPE = [
    "target", "web/target", "cad/wasm/target", "cad/target",
    "cad/wasm/pkg", "renderer/pkg", "cad/.cache",
    "web/assets/cad", "web/assets/cad-worker",
    # web/target also covers both worker outputs, including the offline worker.
    "web/assets/ergogen-models", "web/assets/imported-modules.json",
    "web/assets/renderer", "web/assets/tailwind.css",
]
TARGETS = [p for p in WIPE if p.endswith("target")]
LOAD_LIMIT = 5.0
SCCACHE_KEYS = (
    "Compile requests",
    "Compile requests executed",
    "Cache hits",
    "Cache misses",
    "Cache errors",
    "Compilation failures",
    "Non-cacheable compilations",
    "Non-cacheable calls",
)


def sccache_socket(env, uid, exists):
    """UDS path of the shared boardstudio-sccache daemon, or None when absent."""
    runtime = env.get("XDG_RUNTIME_DIR") or f"/run/user/{uid}"
    for candidate in (f"{runtime}/boardstudio-sccache/server.sock",
                      f"/run/user/{uid}/boardstudio-sccache/server.sock"):
        if exists(candidate):
            return candidate
    return None


def sccache_binary():
    """sccache client, preferring ~/.cargo/bin over PATH. None when absent."""
    dot_cargo = Path.home() / ".cargo" / "bin" / "sccache"
    if dot_cargo.is_file():
        return str(dot_cargo)
    return shutil.which("sccache")


def snapshot_sccache(binary, socket):
    """Raw `sccache --show-stats` text against the UDS daemon; None on any failure."""
    try:
        env = dict(os.environ, SCCACHE_SERVER_UDS=socket)
        run = subprocess.run([binary, "--show-stats"], env=env,
                             capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if run.returncode != 0:
        return None
    return run.stdout


def parse_sccache_stats(text):
    """Map each known `sccache --show-stats` counter to its int value (0 when absent)."""
    values = {}
    for line in text.splitlines():
        name, _, value = line.rpartition("  ")
        name, value = name.strip(), value.strip()
        if name and value.isdigit():
            values[name] = int(value)
    return {key: values.get(key, 0) for key in SCCACHE_KEYS}


def format_sccache_delta(before, after):
    """One-line `+N` counter deltas between two parsed snapshots."""
    delta = {key: after[key] - before[key] for key in SCCACHE_KEYS}
    return ("cold: sccache delta: "
            f"+{delta['Compile requests executed']} executed, "
            f"+{delta['Cache hits']} hits, "
            f"+{delta['Cache misses']} misses, "
            f"+{delta['Non-cacheable calls']} non-cacheable, "
            f"+{delta['Cache errors'] + delta['Compilation failures']} errors")


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
    lock = browser_tools._inherited_build_lock()
    if lock is None:
        print("cold: acquire BOARDSTUDIO_BUILD_LOCK_FD before running", file=sys.stderr)
        return 2
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
    socket = None if without_cache else sccache_socket(
        os.environ, os.getuid(), lambda p: Path(p).exists())
    binary = sccache_binary() if socket else None
    before = snapshot_sccache(binary, socket) if binary else None
    if before is not None:
        (EVIDENCE / f"{name}-sccache-before.log").write_text(before)
    env = dict(os.environ, BUILDTIME_LOG=str(jsonl),
               BUILDTIME_NEXT_WRAPPER="" if without_cache else configured_wrapper(os.environ),
               RUSTC_WRAPPER=str(ROOT / "scripts" / "build-time" / "rustc-log.py"))
    env.pop("BUILDTIME_STEP", None)
    with open(EVIDENCE / f"{name}.log", "w") as log:
        start = time.monotonic()
        code = subprocess.run([sys.executable, "scripts/check.py", *steps],
                              cwd=ROOT, env=env, pass_fds=(lock,),
                              stdout=log, stderr=subprocess.STDOUT).returncode
        usage = resource.getrusage(resource.RUSAGE_CHILDREN)
        print(f"cold: wall {time.monotonic() - start:.1f}s, "
              f"process tree CPU {usage.ru_utime + usage.ru_stime:.1f}s", file=log)
        after = snapshot_sccache(binary, socket) if binary else None
        if before is not None and after is not None:
            (EVIDENCE / f"{name}-sccache-after.log").write_text(after)
            print(format_sccache_delta(parse_sccache_stats(before),
                                       parse_sccache_stats(after)), file=log)
    print(f"exit {code}")
    for line in (EVIDENCE / f"{name}.log").read_text(errors="replace").splitlines():
        if line.startswith(("check: step times", "check: all steps", "cold: wall", "cold: sccache")) or "FAILED" in line:
            print(line)
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
