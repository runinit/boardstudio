# 17: Overlap the idle parts of the browser step

Status: ready-for-agent
Type: build
Blocked by: [16](16-browser-launch-concurrency.md)
Parent: [map](../map.md)

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
