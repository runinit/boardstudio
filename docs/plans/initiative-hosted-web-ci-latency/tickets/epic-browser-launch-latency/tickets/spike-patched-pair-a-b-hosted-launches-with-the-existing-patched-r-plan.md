---
title: 'Patched-pair A/B: hosted launches with the existing patched runner'
type: feature
ticket: 3
created: '2026-11-17'
status: in-progress
baseline_revision: 'fba9ddac15c9b16f978c253ff7f5ae3e2bbb1dcb'
route: oneshot
route_source: auto
risk: medium
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
followup_review_recommended: false
context: []
warnings: []
deferred: []
---

<intent-contract>

## Intent

**Problem:** Finding 1.1 showed the existing `tiny_http` patched pair removes the only indefinite server-side stall that can strand a crate launch, but whether it is the hosted trigger is unproven, and entry 1.2's tri-log capture died with its runners twice. This spike tests the mechanism by intervention with a verdict that survives a runner death.

**Approach:** On throwaway branch `experiment/patched-pair-ab` (base `fba9ddac15c9b16f978c253ff7f5ae3e2bbb1dcb`) with a draft PR targeting `audit/layout-engine-pcb-layout`, change only `.github/workflows/check.yaml`'s `web` job: build the patched pair via `python3 scripts/build-patched-wasm-bindgen.py --output "$RUNNER_TEMP/wasm-bindgen-patched"` (step placed after rust-cache, before the check step), set `BOARDSTUDIO_WASM_BINDGEN_DIR=${{ runner.temp }}/wasm-bindgen-patched` on the unchanged `python3 scripts/check.py build browser` step, and wrap that step's script with a background sampler printing UTC time, a `free -m` summary and load average every 5 s into the streamed log, killed after `check.py` with its exit code preserved. Same crates, filters, counts, timeouts, concurrency, caches; never `workflow_dispatch`, never merge.

**Run protocol:** The draft PR's first `web` job is run 1. Per launch, overhead = `check.py` wall (`check: start`/`check: end` timestamps, `(N s)` line) minus the runner's `finished in` test time, read from the streamed job log. Baselines: A1/A2 9/14 slow each (measurements digest), B1 6/14 (run 44100203460 attempt 1), B2 same run attempt 2 (extract its count/table from `gh` logs if not already recorded). PASS = 0 of 14 launches with >=240 s overhead AND counts unchanged (14 launches; page runner 263 tests if it completes) — then exactly one `gh run rerun --job web` as run 2, same read. FAIL = >=4 slow. AMBIGUOUS = 1–3 slow — stop, no rerun. If the runner takes a shutdown signal and the job hangs, `gh run cancel` promptly; the crate-batch evidence is already streamed, and sampler output around the death is recorded.

**Deliverable:** `finding-patched-pair-ab.md` in this epic's folder, committed on the experiment branch: per-launch overhead tables for run(s) vs A1/A2/B1/B2, page-runner wall/counts, sampler evidence, pass/fail/ambiguous verdict with claim labels, what it does/doesn't prove, recommended next step. Doc-carrying commits named for cherry-pick. Trial only: no adoption, no production/source changes, nothing under `epic-ci-progress-feedback/**`. Stop at `done_checkpoint` for user acceptance.

</intent-contract>

## Implementation Notes

Oneshot because the code change is one ~20-line workflow edit on a throwaway branch; the bulk of the work is operating the hosted experiment and reading its streamed log, fully specified in the intent contract above. Mechanics confirmed at planning: `scripts/check.py:1273` calls `wasm_bindgen_tools.configure`, which validates the pair and prepends it to PATH when `BOARDSTUDIO_WASM_BINDGEN_DIR` is set; `check: start`/`check: end` lines print at `scripts/check.py:515,549,636,655`; the build script downloads its two crates if absent from the Cargo registry cache and refuses a pre-existing `--output` dir, so `$RUNNER_TEMP` (fresh per job) is safe.

## Plan Change Log

## Review Triage Log

## Verification

**Commands:**
- `python3 -c 'import yaml,sys; yaml.safe_load(open(".github/workflows/check.yaml"))'` (or ruby equivalent) — expected: workflow parses after the edit; `git diff` shows only the three intended `web`-job changes.
- `gh pr view <draft PR> --json state,isDraft,baseRefName,headRefName` — expected: draft, base `audit/layout-engine-pcb-layout`, head `experiment/patched-pair-ab`.
- `gh run view <run> --job web --log` extraction — expected: 14 launch walls + `finished in` times (or fewer with the death point named), sampler lines spanning the check step, page-runner outcome; verdict computed per the protocol thresholds.
- `gh run view 44100203460 --attempt 2 --log` (web job) — expected: B2 per-launch table extracted for the baseline comparison.

**Manual checks (if no CLI):**
- Finding doc's tables trace every number to a named run/attempt/job log; verdict sentence states the exact decision left for the user.
