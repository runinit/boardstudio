# 05: Fast `build` step and an opt-in `release` step

Status: resolved
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

Resolved 2026-10-09 (commit `12f37299b`). `check.py build` runs `build-web.py --fast`: the four
`wasm-pack build` commands gain `--no-opt` and `dx build` runs without `--release`. New opt-in
`check.py release` runs `build-web.py` unchanged (optimized workers, `dx build --release`); it is
in `--list` but not in the default check. Pinned by `test_build_is_fast_and_release_is_opt_in` and
`test_fast_provider_commands_skip_wasm_opt_and_the_shipped_ones_keep_it`.

Warm `build`: 192.0 s to 52.5 s. The first build in a fresh worktree took 191.5 s because it
compiled everything cold; the saving there is the skipped `wasm-opt` (core worker 62 s total, CAD
worker 3.4 s). The following `browser` step ran on the fast build with counts equal to the baseline.
What the fast build no longer proves: that `wasm-opt` accepts the workers and that the optimized
page builds. `release` still proves both. The `release` step itself was not run in this effort; its
commands are the unchanged previous `build`.

