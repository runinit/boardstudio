# 01: Destination, scope and the eight settled choices

Status: resolved
Type: grilling
Blocked by: none
Map: [map.md](../map.md)

## Question

What does "done" look like for making the check fast, and which of the nine measured options
are in, out, or deferred?

## Answer

Settled with the user on 2026-10-09 (all recommendations accepted):

1. **Targets.** Routine loop 3 minutes or less; full pre-PR check 6 minutes or less; measured on
   this machine with the 12-job default. Destination: every option implemented or ruled out with
   its timing recorded.
2. **Debug info.** Turn it off only for the wasm **test** runs, through the environment set by
   `scripts/check.py` and `scripts/run-wasm-tests.py`. Dev-web builds stay debuggable. A later
   ticket ([04](04-line-tables-only-measurement.md)) measures `line-tables-only` as a middle
   ground.
3. **`build` step.** Keep a `build` step in the full check, but make it fast (`--no-opt` workers,
   debug `dx`). Add an opt-in `release` step that keeps today's optimized behavior for shipping
   and CI. See [05](05-fast-build-and-release-steps.md).
4. **Routine loop.** A new `quick` mode driven by changed files: format and lint, typecheck, the
   affected native tests and the affected browser tests (existing `--files` selection); it never
   runs `build`. See [08](08-quick-mode.md).
5. **Parallel launches.** The "one Chrome owner" rule is coordination between agents, not a
   technical limit. The check orchestrator may run up to 4 launches at once; other agents must not
   start a second orchestrator. Prove stability with three repeated full runs and identical test
   counts. See [06](06-parallel-test-launches.md).
6. **`touch core/src/lib.rs`.** Its reason is undocumented. Run the affected gates with and
   without it; if results match (including `boundary_parity`), remove it from documented practice.
   See [07](07-drop-the-touch.md).
7. **Out of scope:** CI changes, merging all crates' tests into one wasm binary, `sccache`/`mold`,
   and reusing one `chromedriver`.
8. **Coverage parity.** Every build ticket records executed test counts before and after; a drop
   is a failure. Baseline counts are recorded by [02](02-timing-and-count-baseline.md).
