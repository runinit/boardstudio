# 06: Overlap the harness and tooling compiles

Status: ready-for-agent
Type: build
Blocked by: [02](02-measurement-kit.md)
Parent: [map](../map.md)
Related: [check-speed 12](../../check-speed/issues/12-browser-quick-wins.md)

## Question

Three compiles run alone though nothing depends on them yet. Which can overlap without changing what
a check executes?

## Evidence

| Compile | Where | Today |
|---|---|---|
| Harness `cargo clippy` | last in `lint`, `check.py` `LINT` | 28.6 s span, own target dir, alone; 26.5 s attributed |
| Harness `cargo build` | first thing in `browser`, inside drag smoke | Drag smoke is 88 s: about 40.6 s compile, 47.3 s run (receipt `started_at` to `completed_at`); 43.7 s attributed |
| `cargo build ... --example artifact_request` | `scripts/test-import-kicad-parts.py:25` in `tooling` | 33.9 s; 100% of `tooling`'s compile activity; 63 CPU-s |

- `chromiumoxide_cdp` is the bulk of both harness costs: 50 CPU-s, 46 s attributed. It is generated
  protocol code in a separate target dir, so it does not touch root locks.
- Across the `browser` step, rustc runs at 1 to 2 units for 227 s, leaving most cores idle.
- The KiCad test's Core build cannot be reused later: `test` builds a different variant (different
  metadata for all 37 shared crates, because `-p` and `--workspace` unify features differently). So
  its cost can only move into an overlap, not disappear. The test step's workspace lane is not on the
  critical path, so it has room.
- `check-speed` 12's related proposal pools the whole of drag smoke with the crate batch, estimated
  50 to 70 s. Its risk is load on a timing-sensitive test, with recorded translate flakes in
  `ci-wall-time` ticket 03. This ticket moves only compiles, so test execution stays alone.

## Gain

| Change | Cold wall | CPU | Warm |
|---|---:|---:|---|
| Harness clippy beside root clippy | -20 to -28 s | 0 | small |
| Harness prebuilt beside the crate launches | -30 to -40 s | 0 | none when fresh |
| KiCad test into the `test` batch | -25 to -30 s | 0 | none when fresh |

Total -60 to -90 s. CI gain is unknown: `native` runs `tooling` and `lint` on 4 vCPU, so measure it there.

Effort: about 1 hour each. Risk: low. `scripts/test-check.py` pins command order; update it
deliberately. `_build_harness` serializes through a machine-wide `flock` when no descriptor is
inherited (`browser_tools.py:67-76`), so a parallel build from another worktree can add a wait.

## Work

Three separate commits, each revertible alone.

1. In `run_commands`, run the harness clippy command concurrently with the two root clippy commands. Root
   host and wasm clippy stay in order: both should need the `target/debug` host lock (inferred).
2. Add a harness prebuild command to the crate launch batch and move drag smoke after the batch, which
   only needs the built site and the binary. Fail-fast order changes; say so in the commit.
3. Move `scripts/test-import-kicad-parts.py` from `tooling` to the `test` batch. Make sure
   `check.py`'s selection logic and `test-check-quick.py` still pick it for the same paths, and that
   running `tooling` alone still runs it, or state the change explicitly.

## Verification

```sh
python3 -B scripts/test-check.py && python3 -B scripts/test-check-quick.py
python3 scripts/build-time/cold.py overlap
python3 scripts/build-time/report.py rebaseline overlap --lanes
grep -E 'PASS drag browser smoke: 15 checks|^check: all steps' .evidence/build-time/overlap.log
```

Check each drag `pointerDownToUpMs` and `releaseToDurableMs` in the receipt stays within the reference
run's range.

## Stop rule

Revert any single commit that gains under 10 s, or whose failure output becomes harder to read. Revert
the drag reorder if drag smoke fails in any of three consecutive runs that pass before it.
