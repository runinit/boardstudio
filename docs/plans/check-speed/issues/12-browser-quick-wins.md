# 12: Browser step quick wins

Status: ready-for-agent
Type: build
Blocked by: [11](11-browser-measure.md), [34 (typed-core-edits)](../../typed-core-edits/issues/34-test-speed-browser.md)
Map: [map.md](../map.md)

2026-10-29: the implementation session claimed only the page `--list` integration, then
released the claim on finding a compatibility blocker. The other candidates were not claimed.
Experimental cherry-pick `c78aaa367` on `research/page-list-stamp-integration`, based on
`8d6bbd0fe`, must not be promoted as-is: the removed page listing uses `--locked` and streams
its output; `run-wasm-tests.py` lists without `--locked`, suppresses successful listing output,
and reports only the last 30 lines on listing failure. It also uses `--mode no-install` and
checks for a cached runner before listing, unlike the removed launch. Equal historical test
multisets do not establish those behaviors are equivalent.
On the experimental cherry-pick, `test-check.py` passed 31/31, `test-run-wasm-tests.py`
41/41, and `test-verification-receipt.py` 37/37; these do not cover the identified differences.

Next: preserve locked dependency resolution and full listing/failure diagnostics, account for
the runner installation/preflight difference, and add focused regression tests; then compare
the unchanged and corrected versions under the same warm browser-check conditions, including
test-name multisets and failures. This checkout has no staged browser assets or warm target;
no new browser timing was taken. The original acceptance remains open. No hosted runs, PR or
merge were made.

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
