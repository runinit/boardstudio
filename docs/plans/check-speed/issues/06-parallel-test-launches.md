# 06: Run browser test launches in parallel

Status: ready-for-agent
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

Commits, three run times, counts, peak memory, and any crate that must stay serial and why.
