# 04: Run the CAD test trees as lanes

Status: ready-for-agent
Type: build
Blocked by: [02](02-measurement-kit.md)
Parent: [map](../map.md)
Related: [09](09-cad-owner-feature-set.md), `ci-wall-time` ticket 17 (branch `ci-wall-time-plan`)

## Question

`test` waits on one lane that runs three independent target dirs in series. Can they overlap?

## Evidence

- `cad/scripts/cad_build.py:137-173` runs `cad/monstertruck`, then `cad/wasm`, then seven
  `cad/monstertruck-qualification` selections, one at a time, each with its own `CARGO_TARGET_DIR`.
  Separate target dirs share no Cargo build lock.
- Lane timeline in the `test` step (seconds from step start, parallelism in brackets):
  monstertruck compile 1-71 (3.2x) then its tests to 86; wasm compile 86-152 (5.6x) then tests to
  175; qualification 175-264 (3.4x). The workspace lane ends at 153, footprints at 24.
- So `test` wall is 265 s, set by CAD alone, with 24.5 s fully idle and 92.5 s at 1 to 2 rustc.
- The output contract is already safe to reorder: the qualification selections capture and print
  whole (`cad_build.py:165-171`), and `run_parallel` in `scripts/check.py:540` prints a whole
  command's output on completion. `check_native_tests.verify` parses that captured text.
- `perf/native-ci-topology` already has `native_test_commands()` in `cad_build.py`, a pure command
  list with its target dir, ready to reuse. Its other changes are not in scope.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| -90 to -110 s (`test` 265 s to 155-175 s) | 0 | up to -50 s on a CAD edit, not measured |

The new floor is the workspace lane's 153 s. CI: a 4 vCPU runner is already saturated in this step,
so expect little, and possibly worse under memory pressure.

Effort: 2 hours. Risk: low to medium. Up to 5 concurrent cargo runs at 12 jobs each, contention, and
peak memory on this 30 GB machine. CAD test bodies (`wasm` 21.8 s, `monstertruck` 10.7 s) may be
timing-sensitive under load.

## Work

1. Add `native_test_commands()` as on that branch; it replaces the three hand-written calls.
2. In `test_cad()`, group by target dir into three lanes. Run them in a thread pool with output
   captured per lane, printed whole in the existing order. Keep each selection's "no executed tests"
   check (`cad_build.py:172`) inside its lane.
3. Run the lanes serially when `BOARDSTUDIO_CHECK_JOBS` is 1, as `check.py` does for its batches.
4. Keep `[PY, "cad/scripts/test-cad.py"]` as the single gate command (`check.py:188`, `:446`, `:573`,
   `:969`, `:1097` all name it). Only a failed lane's output and first failure must stay visible.
5. Extend `cad/scripts/test-cad-build.py` (`:160` asserts the targets) for order, target and failure.

This is a test-only slice under the [CAD closure rule](../../cad-backend-replacement/map.md#closure-and-workflow-decisions-2026-10-18-user):
RED/GREEN and a diff read.

## Verification

```sh
python3 -B cad/scripts/test-cad-build.py
python3 scripts/build-time/cold.py cad-lanes test
python3 scripts/build-time/report.py rebaseline cad-lanes --lanes
grep -E '^check: native tests' .evidence/build-time/cad-lanes.log   # 2378 baselined, no missing names
/usr/bin/time -v python3 cad/scripts/test-cad.py 2> .evidence/cad-lanes-rss.txt   # peak RSS, cold
```

RED: break one test in each lane with `scripts/mutation-check.py` and see `test` fail with that
test's output shown.

## Stop rule

Revert if `test` gains less than 40 s, if peak RSS passes 24 GB, or if any CAD test fails in three
consecutive runs where serial passes. On CI, keep the local gain but run serially through
`BOARDSTUDIO_CHECK_JOBS` if the `native` job gets more than 5% slower or is killed.
