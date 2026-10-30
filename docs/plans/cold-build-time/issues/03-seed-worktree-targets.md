# 03: Seed new worktree targets by reflink

Status: resolved
Type: prototype
Blocked by: [02](02-measurement-kit.md)
Parent: [map](../map.md)
Related: [12: sccache](12-sccache.md)

## Question

Can a new worktree start with a copy-on-write copy of an idle worktree's target dirs, so Cargo finds
every registry crate fresh, with no wrapper, daemon or machine-wide install?

## Why this before `sccache`

The brief asks to compare `sccache` and a shared target. Both have a blocker, set out in
[12](12-sccache.md). This third option needs neither. It reuses more than `sccache` can (proc macros
and build scripts too), costs nothing to try, and leaves nothing installed if it fails.

## Evidence

- `/home/chris/01_Projects`, `~/.t3/worktrees` and `~/.cache` are on the same btrfs filesystem
  (`/dev/nvme0n1p5[/@home]`), so `cp --reflink=always` is near-instant and shares extents.
- Output to seed from this worktree: `target` 20 G, `cad/wasm/target` 1.1 G,
  `cad/monstertruck/target` 460 M, `cad/monstertruck-qualification/target` 1.2 G,
  `tools/browser-harness/target` 1.2 G, `web/target` 320 M.
- Registry and git units total 1694 CPU-s (55%) and 303 s attributed wall.
- `research/native-warm` F1 proposes a related test ("a new worktree with the target dir copied in")
  for path crates, and reports them recompiling on fresh checkout, cause unconfirmed. Registry
  units are expected to stay fresh, because their fingerprint uses no source mtime and their
  package id holds no worktree path. **Not verified: no copy has been tried.**

## Gain

| Cold wall (new worktree only) | CPU | Warm |
|---:|---:|---|
| -250 to -300 s (about -20 to -24%) | up to -1694 CPU-s (-55%) | none |

An existing worktree cleaned of its targets gains nothing unless it is re-seeded. CI gains nothing.

Effort: 1 to 2 hours for the prototype, then a short script if it works. Risk: low to the repo.
Copying a donor mid-build can capture a truncated rlib, so the donor must be idle. Stale workspace
artifacts come along for free in disk terms until they diverge, and Cargo never removes them.

## Work

1. In a scratch worktree at the donor's commit, seed all five dirs, then ask Cargo what it thinks is
   dirty. Expect path crates to compile and registry crates not to:
   ```sh
   for d in target cad/wasm/target cad/monstertruck/target \
            cad/monstertruck-qualification/target tools/browser-harness/target; do
     mkdir -p "$(dirname "$d")" && cp -a --reflink=always "$DONOR/$d" "$d"
   done
   CARGO_LOG=cargo::core::compiler::fingerprint=info cargo test --workspace --locked --exclude boardstudio-footprints --no-run 2> .evidence/seed-fingerprint.log
   ```
2. If registry units are fresh, run the full cold measurement in that worktree.
3. If that passes, add `scripts/seed-worktree-target.py DONOR`: refuse unless the donor is idle
   (non-blocking `flock` on each `.cargo-lock`), the toolchain file matches, and the destination has
   no target dir. Document it in the README's worktree section. Skip `web/target`, whose outputs are
   checked for freshness elsewhere.

## Verification

```sh
python3 scripts/build-time/cold.py seed-a --no-wipe     # kit flag added by this ticket
python3 scripts/build-time/report.py rebaseline seed-a
grep -E '^check: (native tests|all steps)' .evidence/build-time/seed-a.log
du -sh --apparent-size target; btrfs filesystem du -s target
```

Run two sequential seeded worktrees from the same donor, then a real edit in each, to confirm no
stale artifact wins over changed source.

## Stop rule

Drop it and go to [12](12-sccache.md) if registry units are not fresh after the copy, if the gain is
under 150 s, or if any gate or count differs from a clean run. Any wrong-artifact result is a
permanent no: this would be a correctness bug with a very confusing failure.

## Resolution (2026-10-30)

Prototype passed every gate. Seeding 21 GB by reflink took 2.5 s. In the seeded worktree only
21 units recompiled, all workspace/path crates (fresh-checkout source mtimes are newer than the
donor fingerprints); every registry crate stayed fresh. Full seeded run: **921.8 s wall /
877 CPU-s** against the same-day rebaseline's 1443.0 s / 2994 CPU-s, all steps passed, native
floor held (2378 + 27), `boardstudio_core` 7× against 9×. Two further seeded worktrees behaved
identically, and a real source edit in each recompiled exactly the edited crate. `btrfs du` on
the seeded `target`: 27.7 GiB total, 10.1 GiB shared.

Landed as `scripts/seed-worktree-target.py` (idle-donor flock with create-if-missing, toolchain
match, no-overwrite refusals), documented in the README, with refusal tests in
`scripts/test-build-time.py`. Ticket 12 (`sccache`) is not needed unless this breaks.
