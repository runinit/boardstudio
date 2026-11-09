# 06: Run browser test launches in parallel

Status: resolved
Type: build
Blocked by: 03
Map: [map.md](../map.md)

## Goal

`check.py browser` runs up to 4 `wasm-pack test` launches at once (decision 5 in
[01](01-destination-and-scope.md)). Earlier measurement: three launches together took 36.8 s
against roughly 70 s serial with debug info, and 24.5 s without it; each launch uses its own
`chromedriver` port.

## Scope

- Run the independent per-crate launches concurrently with a bounded pool; keep each launch's
  output grouped and printed whole so logs stay readable, and keep exit-code behavior (any failure
  fails the step, naming the crate).
- The page-bin batches in `run-wasm-tests.py` keep their own isolation rules (`case_workspace`,
  `setup_guide`, `panels`); do not run two launches that need an isolated DOM against the same
  page process.
- Respect the existing `WASM_BINDGEN_TEST_TIMEOUT`; check memory headroom (16 GB was in use before
  this effort).
- A cap is configurable with an environment variable and defaults to 4.

## Verification

Three repeated full `browser` runs with identical executed counts and no timeout or flake,
recorded with their times. Compare to the serial time from [03](03-wasm-test-debuginfo-off.md).


## Outcome

Resolved 2026-10-09 (commit `12f37299b`). Consecutive crate launches in `browser` run in a pool of
`BOARDSTUDIO_CHECK_JOBS` (default 4, `1` is serial); heavy crates (layout, case, pcb,
parts) start first; each launch's output prints whole with its time; a failure names the launch
and stops the step. The page-bin commands and the isolated batches stay serial after the pool.
Pinned by `test_crate_launches_run_together_and_a_failure_names_the_launch` and
`test_the_browser_job_cap_defaults_to_four_and_one_means_serial`.

Three full `browser` runs (debug info off, 4 parallel): 337.0 s cold in a fresh worktree, then 157.6 s
and 189.3 s warm, against 598.0 s serial with debug info. Executed counts equal the baseline in
all three (crate suites 8/29/5/26+1/30/16/40/12/81/65/49/146+1, 60 page tests); no timeout or
flake. Every crate was safe to run concurrently. Warm run-to-run variance is about 30 s.

