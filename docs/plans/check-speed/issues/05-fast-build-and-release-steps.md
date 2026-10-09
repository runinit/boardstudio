# 05: Fast `build` step and an opt-in `release` step

Status: ready-for-agent
Type: build
Blocked by: 02
Map: [map.md](../map.md)

## Goal

The full check's `build` step stops paying for optimization it does not need, while an opt-in
`release` step keeps today's optimized pipeline for shipping and CI (decision 3 in
[01](01-destination-and-scope.md)).

## Scope

- `scripts/build-web.py` gains a fast mode used by the default `build` step: `wasm-pack build
  --no-opt` for the provider workers (`wasm-opt` took about 85 s and 70 s), and a debug `dx build`
  in place of `dx build --release` (2m18s). Native release fixtures (1m09s) may stay if they are
  needed by later steps; justify keeping or dropping them.
- Add a `release` step to `check.py` that runs the current optimized behavior. It is not in the
  default list; `--list` shows it. Update `scripts/test-check.py`, which pins the documented step
  order.
- Make sure nothing in `browser`, `dev-web.py` or the offline-worker embedding silently depends on
  an optimized artifact. Check `validate_offline_manifest` and the worker size or hash checks.
- Update the README description of `build` and add `release`.

## Verification

`check.py build` and `check.py release` both pass; the fast `build` time before and after is
recorded; the executed counts of the following `browser` step match the baseline.

## Outcome

Commits, timings for both steps, what the fast build no longer proves, and what `release` still
proves.
