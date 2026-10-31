#!/usr/bin/env python3
"""Seed this worktree's target dirs from an idle donor worktree by reflink copy.

Usage (run inside the new, destination worktree):
    python3 scripts/seed-worktree-target.py DONOR_WORKTREE
    python3 scripts/seed-worktree-target.py --auto   # pick the best idle donor

Registry crates stay fresh after the copy; workspace crates recompile once
because the fresh checkout's source mtimes are newer than the donor's
fingerprints. That is still far cheaper than a cold build (ticket 03:
921.8 s / 877 CPU-s seeded against 1443.0 s / 2994 CPU-s cold, same machine).
"""
import contextlib, fcntl, shutil, subprocess, sys
from pathlib import Path

TARGETS = [
    "target", "cad/wasm/target", "cad/monstertruck/target",
    "cad/monstertruck-qualification/target", "tools/browser-harness/target",
]


def present(donor):
    return [rel for rel in TARGETS if (donor / rel).is_dir()]


def refusals(donor, dest):
    if not donor.is_dir():
        return [f"donor {donor} is not a directory"]
    reasons = []
    toolchain = "rust-toolchain.toml"
    if (donor / toolchain).read_bytes() != (dest / toolchain).read_bytes():
        reasons.append("rust-toolchain.toml differs between donor and destination")
    if not (donor / "target").is_dir():
        reasons.append("donor has no target to seed from")
    for rel in present(donor):
        if (dest / rel).exists():
            reasons.append(f"destination already has {rel}; refusing to overwrite")
    return reasons


@contextlib.contextmanager
def donor_idle(donor, rels):
    """Yield False if any donor target lock is busy; else hold them all until exit,
    so a donor build cannot start mid-copy and leave a truncated rlib."""
    handles = []
    try:
        for rel in rels:
            handle = open(donor / rel / ".cargo-lock", "a+")
            handles.append(handle)
            try:
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError:
                yield False
                return
        yield True
    finally:
        for handle in handles:
            handle.close()


def copy(src, dst):
    subprocess.run(["cp", "-a", "--reflink=always", str(src), str(dst)], check=True)


def choose_donor(dest):
    """Best other worktree: same toolchain, has target, idle; most target dirs, then newest."""
    out = subprocess.run(["git", "worktree", "list", "--porcelain"], capture_output=True,
                         text=True, check=True).stdout
    toolchain = (dest / "rust-toolchain.toml").read_bytes()
    ranked = []
    for line in out.splitlines():
        if not line.startswith("worktree "):
            continue
        path = Path(line[len("worktree "):])
        try:
            if path.resolve() == dest.resolve() or not (path / "target").is_dir() \
                    or (path / "rust-toolchain.toml").read_bytes() != toolchain:
                continue
            info = path / "target" / ".rustc_info.json"
            ranked.append((len(present(path)), info.stat().st_mtime if info.exists() else 0, path))
        except OSError:
            continue
    for _, _, path in sorted(ranked, reverse=True):
        rels = present(path)
        try:
            with donor_idle(path, rels) as idle:
                if idle:
                    return path
        except OSError:
            continue
    return None


def seed(donor, dest):
    reasons = refusals(donor, dest)
    if reasons:
        for reason in reasons:
            print(f"seed: refusing: {reason}", file=sys.stderr)
        return 2
    rels = present(donor)
    with donor_idle(donor, rels) as idle:
        if not idle:
            print("seed: refusing: donor is busy (a target lock is held)", file=sys.stderr)
            return 2
        done = []
        for rel in rels:
            (dest / rel).parent.mkdir(parents=True, exist_ok=True)
            done.append(rel)
            try:
                copy(donor / rel, dest / rel)
            except (subprocess.CalledProcessError, OSError) as err:
                for done_rel in done:
                    shutil.rmtree(dest / done_rel, ignore_errors=True)
                print(f"seed: copy of {rel} failed ({err}); removed partial targets", file=sys.stderr)
                return 1
            print(f"seeded {rel}")
    return 0


def main(argv):
    dest = Path.cwd()
    if argv == ["--auto"]:
        donor = choose_donor(dest)
        if donor is None:
            print("seed: no idle donor worktree found; skipping")
            return 0
    elif len(argv) == 1:
        donor = Path(argv[0]).resolve()
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return seed(donor, dest)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
