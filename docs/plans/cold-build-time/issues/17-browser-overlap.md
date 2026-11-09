# 17: Overlap the idle parts of the browser step

Status: resolved
Type: build
Blocked by: [16](16-browser-launch-concurrency.md)
Parent: [map](../map.md)

## Outcome

**Corrected 2026-10-31.** Parts 1 and 2 are kept, and part 3 (strip) stays out, since its proc-macro rebuild
was measured and it was not needed. The rejections below were confounded by the stall.

Likely mechanism: `wasm-bindgen-test-runner` 0.2.129 reads no test output until `goto` returns, with
`page_load: Duration::from_secs(300).max(test)`, so a page slow to reach `load` costs about 300 s while its
tests pass. Launches now navigate with `pageLoadStrategy: "eager"`. The Pi shell, stalled in 8 of 8 runs
before, ran clean once after it (246.4 s browser step). Why pages load slowly there is unknown.

Full cold run with the kept parts: 986.6 s to **868.6 s**, browser 446.6 s to 313.7 s, same 693 launch test
names, 112 page tests, 2378 native tests, 15 drag checks, slowest launch 58 s
(`.evidence/build-time/browser-round2.log`). No flake sweep yet.

Original outcome, kept for the record: Part 1 rejected: the first warm full browser pass exited 0, but its `library`
launch took 287.1 s, above the 120 s stall limit; the part was reverted without
further passes (`.evidence/build-time/t17p1-browser-1.log`). The browser step was
564.4 s; the reference cold browser step was 446.6 s (not a comparable warm
before/after). Load at start was 1.70. Unit checks passed: 67 `test-check.py`
tests and 59 `test-run-wasm-tests.py` tests. No mutation or 4-CPU run was attempted
because the first pass failed the stall rule. The cause and whether this 4-job
stall also occurs on the base are unverified; earlier 6/8-job stalls are in ticket 16.

Part 2 rejected: a focused parallel page run passed all 112 tests in 68 s versus
108.2 s for the previous serial page run; the three filters took 26.9, 61.7 and
27.5 s (`.evidence/build-time/t17p2-focused.{log,json}`). The first full browser
pass exited 0, but its crate-batch `library` launch stalled for 286.3 s and
therefore failed the 120 s limit (`.evidence/build-time/t17p2-browser-1.log`).
No further passes, mutation or 4-CPU run were attempted. Load at the full pass
start was 5.41, so its 525.5 s browser wall is not a reliable gain comparison.
The implementation was reverted. The 0.2.129 runner creates a distinct
`tempfile::tempdir()` per invocation and binds an ephemeral loopback port in
headless mode ([source](https://github.com/wasm-bindgen/wasm-bindgen/blob/0.2.129/crates/cli/src/wasm_bindgen_test_runner.rs));
the 2-job sibling experiments passed, but they do not explain the stalls.

Part 3 rejected: the strip prebuild took 99 s after a warm non-strip build,
compiling 210 units including 27 proc-macro compilation lines
(`.evidence/build-time/t17p3-prebuild.log`, load 5.15). The first full browser
pass exited 0 but its `library` launch stalled for 284.8 s, failing the 120 s
limit (`.evidence/build-time/t17p3-browser-1.log`, load 3.71). Browser wall was
611.3 s versus 564.4 s for the earlier non-strip warm pass, but these include
stalls and are not reliable performance comparisons. The strip changes were
reverted. No further passes, panic-location check, mutation or 4-CPU run were
attempted after the stall. Both Python test suites passed (65 and 59 tests).

Final kept state (no runtime change from the starting head): full cold check exited 0
in 1256.2 s, browser 701.0 s, versus 986.6 s and 446.6 s in the reference. The
count floor held: the same 693 crate-launch names, 112 page tests, 2378 native
baselined tests, 15 drag checks and 364 CAD browser checks. However, the
`ui-shared -- panels::` launch took 284.1 s, so this is **not a stall-free run**
(`.evidence/build-time/t16-17-final.log`; comparison in
`.evidence/build-time/t16-17-final-comparison.txt`). The final run started at
load 1.56. No scratch-base reproduction was run; the final retained runtime code
is unchanged from the starting head.

## Evidence

The browser step takes 446.6 s and averages 1.1 busy threads: CAD and tooling gates 91 s, prebuild
94.6 s, crate batch 76.9 s, drag smoke 54.2 s, `--list` 20.4 s, page run 109.2 s. The page run has three
filters of 1, 109 and 2 tests, about 22 s of launch overhead each (`scripts/run-wasm-tests.py`).

## What must stay as it is

- **Drag smoke stays alone in this ticket.** `ci-wall-time` 03 recorded a "translate diff" failure on
  repeated CI attempts, cause unknown, and it is the most timing-sensitive command here. Reopen only
  with a measured flake rate.
- **`run-wasm-tests --all` stays split into its filters** (OOM and SIGKILL risk, `ci-wall-time`), and
  each filter keeps its own launch: `scripts/run-wasm-tests.py:461` explains why a shared page masks
  failures.
- **`test-cad-browser.py` stays out of overlap with the compile**: its deadlines are 4, 1 and 5 s.
- **Unresolved history:** `wasm-pack` failed once with "invalid type: sequence, expected a string" when
  two launches of one command overlapped (`check-speed` 10). Its cause was never established.

## Work

1. **Gates beside the prebuild.** Run the gates other than `test-cad-browser.py` as one serial lane next
   to `BROWSER_PREBUILD`, then `test-cad-browser.py` after both. Follow the lane pattern in
   `_run_lint_overlap`. Gates keep their build-lock handling; `_build_harness` uses
   `BOARDSTUDIO_BUILD_LOCK_FD` and that must not deadlock or fail its non-blocking `flock`.
2. **Page filters in parallel.** Run the three filters in `run_filter` at once, keeping `browser.json`,
   per-filter logs and the failure report identical in content and deterministic in order. Three
   launches of one test binary overlap here, so first check wasm-bindgen-test-runner's temp directory
   handling (crate launches of one binary have not been shown to overlap either).
3. **Strip A/B, last.** Reopens [08](08-browser-launch-overhead.md), since batch math has changed. Try
   `CARGO_PROFILE_DEV_STRIP=symbols` for the browser step. It may rebuild host proc macros, because
   `strip` applies to the whole profile; count that against the gain.

## Gain

| Part | Cold wall (estimate) | CPU | Warm |
|---|---:|---:|---|
| 1 | -70 to -90 s | none | gates only |
| 2 | -40 to -50 s | none | same |
| 3 | -15 to -40 s | small | same |

## Verification

```sh
python3 -B scripts/test-check.py && python3 -B scripts/test-run-wasm-tests.py
python3 scripts/check.py browser   # five consecutive clean passes per part
taskset -c 0-3 python3 scripts/check.py browser
```

Per part, a mutation with `scripts/mutation-check.py`: a failing page test must still fail the step with
file and line, and a failing gate must fail the step. Then a full cold run. Counts as in
[16](16-browser-launch-concurrency.md).

## Stop rule

Land each part separately. Any failure not reproducible on the base commit means revert that part, not
retry it. Revert 2 or 3 if it gains under 15 s, and 1 if it gains under 30 s. Strip also reverts if a
panic loses its location or `summarize_wasm_failure` gets worse.
