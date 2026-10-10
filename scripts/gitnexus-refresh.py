#!/usr/bin/env python3
"""Refresh this checkout's GitNexus index, once, safely.

  gitnexus-refresh.py            refresh if HEAD moved or the tree has uncommitted source changes
  gitnexus-refresh.py --status   print whether the index is current and why not, change nothing
  gitnexus-refresh.py --dry-run  print the analyze command that would run
  gitnexus-refresh.py --background   for a Stop hook: refresh only a commit-stale index, only while
                                 nothing heavy is running, at most every 15 minutes, detached,
                                 silent, always exit 0

Only one refresh runs at a time for the whole repository family (the main checkout and every
worktree share one lock in the git common dir). A second caller exits at once with
"refresh in progress" instead of queueing or racing the first. Agents must not run this: the
orchestrating session refreshes at batch boundaries (docs/agents/gitnexus-index.md).
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TIMEOUT_SECONDS = 30 * 60
BACKGROUND_INTERVAL_SECONDS = 15 * 60
# Work whose timings or CPU a re-index would disturb. The Stop hook waits until none is running.
HEAVY_PROCESSES = ("cargo", "rustc", "wasm-pack", "wasm-bindgen-test-runner", "chromedriver", "chrome", "chromium", "podman")
HEAVY_SCRIPTS = ("check.py", "build-web.py", "run-wasm-tests.py", "check-wasm-tests.py")


def git(*args: str, cwd: Path = ROOT) -> str:
    done = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=True)
    return done.stdout.strip()


def alias(root: Path = ROOT) -> str:
    """A name unique to this checkout, so worktrees never share the registry name `boardstudio`."""
    return "boardstudio" if not (root / ".git").is_file() else f"boardstudio-{root.name}"


def dirty_sources(root: Path = ROOT) -> list[str]:
    """Uncommitted changes that the index would miss (the index itself is ignored by git)."""
    lines = git("status", "--porcelain", "--untracked-files=all", cwd=root).splitlines()
    return [line for line in lines if ".gitnexus/" not in line]


def indexed_commit(root: Path = ROOT) -> str | None:
    try:
        return json.loads((root / ".gitnexus" / "meta.json").read_text())["lastCommit"]
    except (OSError, ValueError, KeyError):
        return None


def stale_reason(root: Path = ROOT) -> str | None:
    head = git("rev-parse", "HEAD", cwd=root)
    indexed = indexed_commit(root)
    if indexed is None:
        return "no index in this checkout"
    if indexed != head:
        return f"index is at {indexed[:9]}, HEAD is {head[:9]}"
    dirty = dirty_sources(root)
    if dirty:
        return f"{len(dirty)} uncommitted change(s) are not in the index"
    return None


def analyze_command(root: Path = ROOT) -> list[str]:
    runner = root / ".gitnexus" / "run.cjs"
    if runner.is_file():
        return ["node", str(runner), "analyze", str(root), "--index-only", "--name", alias(root), "--force"]
    return [
        "pnpm", "--allow-build=@ladybugdb/core", "--allow-build=gitnexus", "--allow-build=tree-sitter",
        "dlx", "gitnexus@latest", "analyze", str(root), "--index-only", "--name", alias(root), "--force",
    ]


def heavy_work_running() -> bool:
    """True when a build, test run or browser is going (looks at every process of this user)."""
    me = os.getpid()
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit() or int(entry.name) == me:
            continue
        try:
            argv = (entry / "cmdline").read_bytes().split(b"\0")
            exe = Path(argv[0].decode(errors="replace")).name
        except (OSError, IndexError):
            continue
        if exe in HEAVY_PROCESSES:
            return True
        if exe.startswith("python") and any(Path(a.decode(errors="replace")).name in HEAVY_SCRIPTS for a in argv[1:3]):
            return True
    return False


def head_stale(root: Path = ROOT) -> bool:
    """Committed staleness only: the Stop hook must not re-index on every uncommitted edit."""
    indexed = indexed_commit(root)
    return indexed is None or indexed != git("rev-parse", "HEAD", cwd=root)


def background(root: Path = ROOT) -> int:
    """Stop-hook mode: cheap checks, then a detached refresh. Never blocks, never prints, exits 0."""
    try:
        if not head_stale(root) or heavy_work_running():
            return 0
        stamp = Path(git("rev-parse", "--git-common-dir", cwd=root)).resolve() / f"gitnexus-refresh-{alias(root)}.stamp"
        if stamp.exists() and time.time() - stamp.stat().st_mtime < BACKGROUND_INTERVAL_SECONDS:
            return 0
        stamp.touch()
        log = open(root / ".gitnexus" / "refresh.log", "ab") if (root / ".gitnexus").is_dir() else subprocess.DEVNULL
        nice = [shutil.which("nice") or "nice", "-n", "19"]
        subprocess.Popen(
            [*nice, sys.executable, str(Path(__file__).resolve())],
            cwd=root, stdout=log, stderr=log, stdin=subprocess.DEVNULL, start_new_session=True,
        )
    except Exception:  # a hook must never fail the session
        pass
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--status", action="store_true", help="report only; exit 1 when stale")
    parser.add_argument("--dry-run", action="store_true", help="print the command and exit")
    parser.add_argument("--background", action="store_true", help="Stop-hook mode (see above)")
    args = parser.parse_args(argv)
    if args.background:
        return background()

    reason = stale_reason()
    if args.status:
        print(f"{alias()}: " + ("current" if reason is None else f"stale ({reason})"))
        return 0 if reason is None else 1
    if reason is None:
        print(f"{alias()}: index is current, nothing to do")
        return 0
    command = analyze_command()
    if args.dry_run:
        print(" ".join(command))
        return 0

    lock_path = Path(git("rev-parse", "--git-common-dir")).resolve() / "gitnexus-refresh.lock"
    with open(lock_path, "w") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print("refresh in progress in another checkout or session; not starting a second one")
            return 0
        lock.write(f"{alias()} {time.strftime('%FT%T')}\n")
        lock.flush()
        print(f"refreshing {alias()}: {reason}", flush=True)
        started = time.monotonic()
        try:
            done = subprocess.run(command, cwd=ROOT, timeout=TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            print(f"refresh timed out after {TIMEOUT_SECONDS}s", file=sys.stderr)
            return 1
        print(f"refresh exit {done.returncode} in {time.monotonic() - started:.0f}s")
        if done.returncode != 0:
            return done.returncode
    after = stale_reason()
    print(f"{alias()}: " + ("current" if after is None else f"still stale ({after})"))
    return 0 if after is None else 1


if __name__ == "__main__":
    sys.exit(main())
