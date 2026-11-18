# 02: Commit the measurement kit and rebaseline

Status: resolved
Type: build
Blocked by: [01](01-land-round-one-and-audit.md)
Parent: [map](../map.md)

## Question

Every ticket here needs the unit log, but the tools live in gitignored `.evidence/build-time/` in
one worktree. How does any worktree reproduce the measurement?

## Evidence

- `.evidence/build-time/{rustc-log.py,report.py,cold.sh}` exist only in this worktree.
- `cold.sh` runs `rm -rf target ...`. In a worktree whose `target` is a symlink, as the 3 symlinks
  found by the map's scan and any lane worktree are, the build outputs live elsewhere, so a wipe
  plus a run would not be a comparable cold run.
- The log lacks whether a unit is incremental, and `report.py` has no lane timeline or attribution.
  Those two views are what produced this plan's findings, from throwaway scripts.

## Gain

None directly. Effort: 1 hour plus one cold run (21 min). Risk: low; it is a measuring tool, and
its `rm -rf` is the only sharp edge.

## Work

Smallest thing that works:

1. Move the three files to `scripts/build-time/` as `rustc-log.py`, `report.py`, `cold.py`. Keep
   writing logs to `.evidence/build-time/`.
2. `cold.py` refuses to run if `CARGO_TARGET_DIR` is set, if any target path it would wipe is a
   symlink, or if load average is 3 or more. Print why.
3. `report.py` gains two views: `--lanes` (per target dir and profile: start, end, CPU, parallelism)
   and `--attrib SET` (attributed wall for a named crate list). Record `-C incremental` presence in
   the unit log.
4. One test file, `scripts/test-build-time.py`, covering the refusals and the attribution sum, and
   registered in `check.py`'s tooling step like its neighbours.

## Verification

```sh
python3 -B scripts/test-build-time.py
python3 scripts/build-time/cold.py rebaseline     # one full cold run after 01 lands
python3 scripts/build-time/report.py rebaseline
python3 scripts/build-time/report.py rebaseline --lanes
grep -c boardstudio_core .evidence/build-time/rebaseline.jsonl
```

Record the rebaseline's wall, CPU and per-step numbers in this ticket. It replaces the map's
reference point for tickets 03 onward.

## Stop rule

If the rebaseline's wall or CPU is more than 10% above round one's final run with no code reason,
stop and find out why before starting any other ticket. A moved baseline invalidates their
estimates.

## Resolution (2026-10-30)

Kit committed as `7aefe4ce` in `scripts/build-time/`; `scripts/test-build-time.py` (7 tests)
registered in `check.py` tooling. Rebaseline cold run (exit 0, all steps passed):

| Step | Round-one final wall | Rebaseline wall | Rebaseline CPU-s |
|---|---:|---:|---:|
| tooling | 77.6 | 75.3 | 94 |
| lint | 127.0 | 136.0 | 429 |
| build | 297.6 | 282.1 | 682 |
| test | 265.1 | 269.2 | 1370 |
| browser | 504.7 | 678.9 | 419 |
| **Total** | **1273.8** | **1443.0** | **2994 (vs 3069)** |

Stop-rule investigation: wall is +13.3% but CPU is −2.4% and every crate count matches
(`boardstudio_core` 9×, 282 vs 288 CPU-s). The run started at 1-minute load 2.5 with 5-minute
load 5.5 from other sessions, breaching the map's quiet-machine protocol; the excess is browser
execution wait, not compile. No code reason found. Treat CPU (2994) as the rebaseline comparator
for tickets 03 onward; re-measure wall on a quiet machine before quoting it.
