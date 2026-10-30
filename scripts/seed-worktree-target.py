#!/usr/bin/env python3
"""Seed this worktree's target dirs from an idle donor worktree by reflink copy.

Usage (run inside the new, destination worktree):
    python3 scripts/seed-worktree-target.py DONOR_WORKTREE

Registry crates stay fresh after the copy; workspace crates recompile once
because the fresh checkout's source mtimes are newer than the donor's
fingerprints. That is still far cheaper than a cold build (ticket 03:
921.8 s / 877 CPU-s seeded against 1443.0 s / 2994 CPU-s cold, same machine).
"""
import fcntl, subprocess, sys
from pathlib import Path

TARGETS = [
    "target", "cad/wasm/target", "cad/monstertruck/target",
    "cad/monstertruck-qualification/target", "tools/browser-harness/target",
]


def refusals(donor, dest):
    reasons = []
    if not donor.is_dir():
        return [f"donor {donor} is not a directory"]
    toolchain = "rust-toolchain.toml"
    if (donor / toolchain).read_bytes() != (dest / toolchain).read_bytes():
        reasons.append("rust-toolchain.toml differs between donor and destination")
    for rel in TARGETS:
        if not (donor / rel).is_dir():
            reasons.append(f"donor has no {rel} to seed from")
        if (dest / rel).exists():
            reasons.append(f"destination already has {rel}; refusing to overwrite")
    return reasons


def donor_idle(donor):
    """Hold a non-blocking flock on each donor target lock; False if any is busy."""
    handles = []
    try:
        for rel in TARGETS:
            handle = open(donor / rel / ".cargo-lock", "a+")
            try:
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError:
                handle.close()
                return False
            handles.append(handle)
        return True
    finally:
        for handle in handles:
            handle.close()


def main(argv):
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    donor, dest = Path(argv[0]).resolve(), Path.cwd()
    reasons = refusals(donor, dest)
    if reasons:
        for reason in reasons:
            print(f"seed: refusing: {reason}", file=sys.stderr)
        return 2
    if not donor_idle(donor):
        print("seed: refusing: donor is busy (a target lock is held)", file=sys.stderr)
        return 2
    for rel in TARGETS:
        (dest / rel).parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["cp", "-a", "--reflink=always", str(donor / rel), str(dest / rel)],
                       check=True)
        print(f"seeded {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
