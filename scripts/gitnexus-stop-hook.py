#!/usr/bin/env python3
"""Run the GitNexus Stop refresh only for the registered orchestrator session."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

REGISTRATION_NAME = "gitnexus-orchestrator.json"


def git_common_dir(worktree: Path) -> Path:
    result = subprocess.run(
        ["git", "-C", str(worktree), "rev-parse", "--path-format=absolute", "--git-common-dir"],
        capture_output=True,
        check=True,
        text=True,
    )
    return Path(result.stdout.strip()).resolve()


def register(session_id: str, worktree: Path) -> int:
    if not session_id.strip():
        raise ValueError("session ID must not be empty")
    target = worktree.expanduser().resolve()
    if not target.is_dir():
        raise ValueError(f"worktree does not exist: {target}")
    common_dir = git_common_dir(target)
    registration_path = common_dir / REGISTRATION_NAME
    registration_path.write_text(
        json.dumps({"session_id": session_id, "worktree": str(target)}, indent=2) + "\n",
        encoding="utf-8",
    )
    return 0


def read_registration(common_dir: Path) -> dict[str, Any] | None:
    try:
        data = json.loads((common_dir / REGISTRATION_NAME).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None
    if not isinstance(data, dict):
        return None
    session_id = data.get("session_id")
    worktree = data.get("worktree")
    if not isinstance(session_id, str) or not session_id.strip():
        return None
    if not isinstance(worktree, str) or not worktree:
        return None
    target = Path(worktree)
    if not target.is_absolute():
        return None
    data["worktree"] = str(target.resolve())
    return data


def run_refresh(worktree: Path) -> None:
    script = worktree / "scripts" / "gitnexus-refresh.py"
    subprocess.run(
        [sys.executable, str(script), "--background"],
        cwd=worktree,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )


def hook() -> int:
    try:
        event = json.load(sys.stdin)
        if not isinstance(event, dict):
            return 0
        session_id = event.get("session_id") or event.get("thread_id")
        cwd = event.get("cwd")
        if not isinstance(session_id, str) or not session_id.strip() or not isinstance(cwd, str) or not cwd:
            return 0
        common_dir = git_common_dir(Path(cwd).expanduser().resolve())
        registration = read_registration(common_dir)
        if registration is None or registration["session_id"] != session_id:
            return 0

        target = Path(registration["worktree"])
        if not target.is_dir() or git_common_dir(target) != common_dir:
            return 0
        script = target / "scripts" / "gitnexus-refresh.py"
        if not script.is_file():
            return 0
        run_refresh(target)
    except Exception:
        # Stop hooks must never fail the session or emit diagnostics.
        return 0
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--register", metavar="SESSION_ID", help="register this orchestrator session")
    parser.add_argument("--worktree", type=Path, help="registered worktree (defaults to the current directory)")
    args = parser.parse_args(argv)
    if args.register is None:
        if args.worktree is not None:
            parser.error("--worktree requires --register")
        return hook()
    try:
        return register(args.register, args.worktree if args.worktree is not None else Path.cwd())
    except (OSError, subprocess.SubprocessError, ValueError) as error:
        print(f"cannot register GitNexus orchestrator session: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
