---
title: 'Patched-pair A/B: hosted launches with the existing patched runner'
type: feature
ticket: 3
created: '2026-11-17'
status: built
baseline_revision: 'fba9ddac15c9b16f978c253ff7f5ae3e2bbb1dcb'
route: oneshot
route_source: auto
risk: medium
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
followup_review_recommended: true
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

- 2026-11-17: branch `experiment/patched-pair-ab` was already at the base in this worktree; plan committed as `7a99d1995a415f8105652d482e03413f0d22e8bc`, workflow edit as `da590d550a2a1189c76c30c80d49a5a76e6492a4` (YAML parses; sampler snippet and exit-code preservation exercised locally; diff read — only the three intended `web`-job changes). Pushed; draft PR #21 targets `audit/layout-engine-pcb-layout` (remote at `46d758d`, an ancestor of the base; only docs commits between, no workflow/script diff).
- Baselines B1/B2 extracted from run 44100203460 attempt 1/2 web-job logs with a parser keyed on check.py's per-launch blocks (`$ wasm-pack test …` header, `finished in`, `(N s)` footer; cross-checked against `check: start/end` timestamps — equal). B1 reproduces the recorded 6/14; B2 = 5/14 (new). Both B batches span 626 s / 596 s and both page runners died ~50 s in.
- Run 1 (run 44119032280 attempt 1, web job 134164964892): patched pair built in 77 s and selected (`wasm-bindgen pair:` lines). Crate batch 0/14 slow — max overhead 89.5 s, all counts unchanged, batch span 167.2 s. Page runner took a shutdown signal 42 s in (exit 143); sampler shows available memory collapsing 11.0 GiB → ~0.2 GiB across its start. Run 1 reads PASS per protocol; one web rerun launched (attempt 2, job 134180142688).
- Run 2: 0/14 slow again (max overhead 96.7 s, batch span 187.3 s, counts unchanged); page runner died the same way (~48 s in) with the same sampler memory collapse (13.1 GiB → ~0.2 GiB available). Verdict PASS. `finding-patched-pair-ab.md` written in the epic folder with per-launch tables for both runs vs A1/A2/B1/B2, page-runner and sampler evidence, and the decisions left to the user. No further reruns per protocol.

## Plan Change Log

## Review Triage Log

### 2026-11-17 — Review pass
- verdicts: 4 findings — high 0, medium 2, low 2, false 0, maybe-false 0
- findings:
  - `[medium]` `[patch]` Finding misstated the page-runner death as preceding test execution (`finding-patched-pair-ab.md`, "before any `[n/3]` filter line" / "the death precedes test execution") — verified in `scripts/run-wasm-tests.py:540-562`: all filters submit at once to a 3-worker `ThreadPoolExecutor` and `[n/3]` prints only on completion, so up to three filter invocations were in flight at the death. Patched: death-position passages, the sampler hypothesis (concurrent filter launches, not a singular Chrome start), and recommendation 3's attribution scope rewritten to match.
  - `[medium]` `[patch]` Central sentence labelled **measured** while claiming the pair "removes" the slow group in "representative" runs — verified against the finding's own label rule: counts are measured, causation/representativeness are inference, and runs 1–2 are attempts of one workflow run ~25 min apart. Patched: "Does prove" rewritten to separate the measured contrast from the hypothesis-grade causal attribution and to qualify representativeness.
  - `[low]` `[patch]` Finding stopped at step exit 143 without either web job's conclusion/end time, leaving the protocol's cancel-on-hang branch unauditable — verified via run API: both jobs concluded `failure` 5 s / 3 s after their check steps, neither hung, no `gh run cancel` used. Patched: job-conclusion record added to the page-runner section.
  - `[low]` `[patch]` "Counts unchanged" was not checkable for B1/B2 (no per-launch counts in the finding) — verified: extracted B counts existed only in parser output. Patched: baseline table gained a per-launch Tests column (A from digest, B extracted; all batches 897) with an evidence note.

## Verification

**Commands:**
- `python3 -c 'import yaml,sys; yaml.safe_load(open(".github/workflows/check.yaml"))'` (or ruby equivalent) — expected: workflow parses after the edit; `git diff` shows only the three intended `web`-job changes.
- `gh pr view <draft PR> --json state,isDraft,baseRefName,headRefName` — expected: draft, base `audit/layout-engine-pcb-layout`, head `experiment/patched-pair-ab`.
- `gh run view <run> --job web --log` extraction — expected: 14 launch walls + `finished in` times (or fewer with the death point named), sampler lines spanning the check step, page-runner outcome; verdict computed per the protocol thresholds.
- `gh run view 44100203460 --attempt 2 --log` (web job) — expected: B2 per-launch table extracted for the baseline comparison.

**Manual checks (if no CLI):**
- Finding doc's tables trace every number to a named run/attempt/job log; verdict sentence states the exact decision left for the user.

## Auto Run Result

- Summary: ran the patched-pair A/B exactly per protocol. Throwaway branch `experiment/patched-pair-ab` + draft PR #21; only `.github/workflows/check.yaml`'s `web` job changed (patched-pair build step, `BOARDSTUDIO_WASM_BINDGEN_DIR`, in-step 5 s resource sampler). Run 1 (run 44119032280 attempt 1) read 0/14 slow with counts unchanged → PASS → one web rerun (attempt 2), also 0/14. Page runner died in both (~42/48 s, shutdown signal) as in A1/B1/B2; sampler captured available memory collapsing ~11–13 GiB → ~0.2 GiB at page-runner start in both runs. Baselines: A1/A2 9/14 (digest), B1 6/14 (extraction matches record), B2 5/14 (new extraction). Verdict PASS, recorded in `finding-patched-pair-ab.md`; stopped at the ticket's `done_checkpoint` — no adoption, no merge, no `workflow_dispatch`.
- Files changed:
  - `.github/workflows/check.yaml` — experiment-only `web` job edit (build patched pair to `$RUNNER_TEMP/wasm-bindgen-patched`, select via env, sampler wraps check step preserving exit code). Throwaway; not for cherry-pick.
  - `docs/plans/initiative-hosted-web-ci-latency/tickets/epic-browser-launch-latency/finding-patched-pair-ab.md` — the finding (new).
  - this plan — created and kept current through the run.
- Commits on `experiment/patched-pair-ab`: `7a99d1995a415f8105652d482e03413f0d22e8bc` (plan), `da590d550a2a1189c76c30c80d49a5a76e6492a4` (workflow experiment), `067d02be4401996cd020e74684c4ee29ef4cb99e` (finding + plan notes), plus the review-fix/bookkeeping commit that carries this result. Doc-carrying commits for cherry-pick: the plan commit, the finding commit, and the review-fix commit — not the workflow commit.
- Review findings breakdown: quick lens, 4 findings — 2 medium + 2 low, all verified real and patched (finding-doc corrections; see triage log). Deferred: none. Rejected: none.
- Follow-up review: patched counts by verdict — high 0, medium 2, low 2. `followup_review_recommended: true`: two medium entries were patched by the implementer without independent re-read; the specific unverified risk is that the rewritten passages (page-runner death position vs `scripts/run-wasm-tests.py`, claim-label split, job-conclusion record, baseline Tests column) still misstate a source or log fact in a way only a fresh lens would catch.
- Verification performed: YAML parse of the workflow (ok, before push and after patches); sampler snippet + exit-code preservation exercised locally; diff since baseline read in full (only the three intended `web` changes + two docs); `gh pr view 21` confirms draft/base/head; per-launch extraction run on all four web logs (B1 6/14 reproduces the recorded count; run 1/run 2 0/14; walls cross-checked against `check: start/end` timestamps); job conclusions/times read from the run API; verification re-run after review patches (all green). The docs-push-triggered run 44128359986 (not protocol evidence) was cancelled.
- Residual risks: the A/B's two runs are back-to-back attempts of one workflow run, so the causal attribution is intervention inference, not request-level proof (entry 1.2 remains the designed proof path); the 263 page tests are unverified under the patched pair because the runner died first in both runs; the page-runner memory death (5 of last 6 hosted web attempts) is unscoped and will keep failing the `web` job until separately decided.
