# 03: Turn debug info off for wasm test runs

Status: ready-for-agent
Type: build
Blocked by: 02
Map: [map.md](../map.md)

## Goal

The wasm test launches in `check.py browser` and `scripts/run-wasm-tests.py` build and load test
wasm without DWARF. Measured earlier with `CARGO_PROFILE_DEV_DEBUG=false` in a throwaway target
directory: about 86% of each test wasm was debug info, and a launch fell from 17.3 s to 6.2 s
(catalogue), 25.1 s to 10.9 s (keycaps) and 49.0 s to 23.6 s (layout).

## Scope

- Set the environment for the test runs only, in `check.py` (browser step) and
  `run-wasm-tests.py`, so a direct `wasm-pack test` by hand and dev-web builds are unchanged.
  Prefer an explicit, documented variable over editing profiles globally (decision 2 in
  [01](01-destination-and-scope.md)).
- Confirm panics in a failing test still report their file and line, and that the artifacts land in
  the normal target directory without invalidating dev-web's cache (a differing profile
  fingerprint is expected; note the disk cost).
- Update the script tests and the README paragraph that describes the browser runner.

## Verification

Executed counts equal the [02](02-timing-and-count-baseline.md) baseline; the browser step time
before and after is recorded; `python3 scripts/check.py repo tooling typecheck browser` passes.

## Outcome

Commits, before/after times, counts, and the panic-location check.
