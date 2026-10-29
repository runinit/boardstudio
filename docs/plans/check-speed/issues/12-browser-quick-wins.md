# 12: Browser step quick wins

Status: claimed
Type: build
Blocked by: [11](11-browser-measure.md), [34 (typed-core-edits)](../../typed-core-edits/issues/34-test-speed-browser.md)
Map: [map.md](../map.md)

Claim, 2026-10-29: focused local integration of the existing page `--list` removal only,
by the current implementation session, on `research/page-list-stamp-integration` from
`research/cad-build-stamp-safety` at `8d6bbd0fe`. Other candidates remain unclaimed. No
hosted runs, PR or merge; original acceptance remains open pending comparable evidence.

## Request

Apply the cheap wins the measurements support. Candidates from the deep dive (each kept only if 11 or a re-measure shows a net gain,
counts unchanged, and three consecutive runs pass):

- Drop the serial `--list` launch at `scripts/check.py:86`; the page runner already lists (update `scripts/test-check.py`, which pins command order).
- Put `keymap` in `HEAVY_CRATES` (`check.py:116`) so the second-slowest suite starts early.
- Choose `BOARDSTUDIO_CHECK_JOBS` from the sweep (16 CPUs, 30 GB RAM here), keeping a default safe for smaller machines.
- Strip the `name` section from test wasm (`CARGO_PROFILE_DEV_STRIP=symbols` or a test profile) if the A/B holds and panic locations survive.
- Use `--mode no-install` consistently for crate launches.
- Remove the `case_workspace` isolation tail (about 15 s) if the two tests can share a session by cleaning up their roots.
- Make CI set the same debug-info profile as local (`.github/workflows/check.yaml` uses `line-tables-only`, which ticket 04 measured about 48% slower).
- Add the host `renderer_host`, `boundary_parity` and `cad_worker` tests to `browser` if cheap (an addition, not a loss).

## Acceptance

- [ ] `browser` wall before/after, three consecutive passing runs, executed-test-name multiset equal to the baseline (plus any deliberate additions).
- [ ] Mutation: a deliberately failing test in each affected launch still fails the step; a panic keeps its location.
