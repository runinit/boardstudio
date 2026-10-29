# Map: CI wall time on stack PRs

## Near-term focus — 2026-10-29

Prioritize quick local web-build improvements and fixes for real failing tests; keep the CI
destination and tickets below as historical and later work, not the current execution queue.
Do not weaken or remove checks to make failures pass. No new tuning matrix, hosted runs, or
merges until the user returns to GitHub work; do not touch PR #8 or `stack/*` branches. This
does not change existing ticket claims or imply that active work has finished or stopped.
The chief-of-staff thread may claim related work after checking active ownership; record actual
claim and status changes in the relevant ticket and keep this map consistent. See the dated
[user decision](wiki/decisions.md).

1. Reuse the existing provider parallelism on `research/local-web-builds` at `0f7c63087`.
   No trivial safe repeat-`wasm-opt` skip was found; do not copy the old CAD stamp shortcut.
   The focused correctness fix exists locally on `research/cad-build-stamp-safety` at
   `8d6bbd0fe`: it removes the skip and checks WASM and JS outputs, with regression tests.
   It is not claimed landed on `dev` or in CI; see [baseline](wiki/baseline.md).
2. Reproduce and fix actual test failures locally, keeping the relevant checks intact. Verify
   each fix with its focused test and any directly affected checks. Ticket [03](issues/03-warm-baseline-measure.md)
   records historical `test-serve-web.py` timeout and drag-smoke translate flakes; reproduce
   before attributing current failures to them. Retry policy remains in [07](issues/07-retry-and-heartbeat-policy.md).
3. When no competing build is active, measure one representative warm local web build before
   and after a candidate with the same command, inputs and cache state; record full end-to-end
   wall, revision, result and relevant test counts. Keep changes only with evidence of benefit
   and passing focused tests. Do not present provider-phase timings as end-to-end timings.

   2026-10-29 packaging-copy investigation at `8d6bbd0fe`: not claimed or implemented.
   The final overlays preserve precedence and `--skip-providers` behavior, so deleting them is
   not equivalent. A local warm filesystem microcheck copying the 42 MiB catalogue plus 44 MiB
   provider trees took 0.009s initially and 0.005s on two repeats; this is not an end-to-end
   build or a CI measurement. No credible local wall-time benefit justified a more complex
   selective copy. Revisit only with representative evidence that packaging is material.

   2026-10-29 web fixture investigation: `research/web-build-fixture-extraction` at
   `8d6bbd0fe` passed 10/10 focused `test-build-web.py` tests. No extraction was made: the
   setups serve distinct scenarios and a shared helper would add indirection. No speed gain is
   claimed; no ticket was needed for the no-change investigation.

   2026-10-29 native test simplification: `research/native-test-simplification` at `ae73cf0a`,
   based on `8d6bbd0fe`, removed an unnecessary command mock and added a missing-WASM assertion.
   All 17 tests passed before and after with the same names; single runs of 0.237s and 0.250s are
   too noisy to establish a speedup. No matching existing ticket, full gate or hosted CI run.

The page-list integration portion of [check-speed 12](../check-speed/issues/12-browser-quick-wins.md)
was claimed then released: experimental cherry-pick `c78aaa367` changes locking and listing
diagnostics and needs the documented compatibility work before promotion. No fresh warm browser
comparison was taken; its historical ~5s gain is unverified on this base. Do not promote it as-is;
deprioritize repair unless it directly enables critical-path work. The other quick-win candidates
were not claimed by this session.

See [decisions](wiki/decisions.md) and [baseline](wiki/baseline.md) for dated context and
existing measurements. Hosted CI receipts and the original gate criteria remain necessary when
GitHub work resumes; the local check is not a substitute.

## Full-gate destination and sequence — 2026-10-29

The user now aspires to **approximately 10 minutes cold overall for the full required PR gate**,
not the earlier 17-minute tiered gate, and authorizes topology changes if needed (user direction,
2026-10-29). This supersedes the earlier tiering policy for this goal:
do not remove, defer, or weaken required coverage to achieve the time. The current local-first
constraints above remain in force; no new hosted runs, PRs, merges, or ticket 15 matrix until
the user returns to GitHub work. See the [dated decision](wiki/decisions.md) and
[baseline](wiki/baseline.md).

On 2026-10-29 the user authorized the chief-of-staff to decide cache and runner policies; this
does not itself select a policy or authorize purchases, provisioning, or hosted CI. Existing
local-first constraints and claims remain in force. See the [dated grant](wiki/decisions.md).

Measured anchors: on the green cold baseline, native and web ran concurrently at 30m19s and
33m37s; native tooling/lint/test were 143/477/1095s and web build/browser 624/1363s. The
same-head warm probe measured 14m41s native and 26m03s web, including setup, not a 10-minute
gate. Its native CAD test lane alone took 640s (including 527s compile); web warm build/browser
were about 6.1/19.2 minutes. Provider overlap's local provider-phase savings and the roughly
5s page-list result are not full-gate gains. Details and qualifications: tickets
[03](issues/03-warm-baseline-measure.md), [05](issues/05-provider-parallelism.md),
[06](issues/06-page-list-removal.md), and [14](issues/14-native-warm-wall.md).

Prioritized local-first work, respecting existing ownership:

1. Finish the focused web-build correctness and measurement work above; compare equivalent
   end-to-end commands and retain all tests. The blocked page-list candidate is not a priority.
   Profile the full browser and native paths locally by compile, execution, setup and wait time,
   with test identities/counts and cache state. Do not mistake a `--fast` build for release proof.
2. Investigate CAD executed-name protection before restructuring. Measure native and web actual
   prerequisite graphs and candidate critical paths, including setup, restore, compilation,
   transfer and longest shard. Establish each browser consumer's real inputs; preserve
   `run-wasm-tests --all` isolation and all existing tests. Do not build aggregation before
   measurement and topology decisions. Ticket [17](issues/17-native-topology.md) tracks the
   native follow-up; the coordinator must check active ownership before claiming it.
3. Present measured full-coverage native and web topology and artifact options for human decision
   ([17](issues/17-native-topology.md), [04](issues/04-web-split-design.md)). A single noisy
   shared-target comparison did not establish a gate saving. Only then implement and locally
   validate the chosen design, including observed launch and test-name parity, failure handling,
   isolation and fail-closed required-check semantics. Evaluate existing
   `scripts/verification_receipt.py` where suitable, but verify its separate browser and CAD gaps;
   ticket [16](issues/16-web-partition-proof.md) is not execution proof.
4. Validate cache reachability and provenance before relying on warm performance: current
   checkout saves Rust caches only on main/dev; the scoped cache proposal and its quota and
   cold-transition risks are documented in [01](issues/01-cache-readiness.md) and
   [12](issues/12-land-cache-change.md). PR #13 was recorded there as open with CI pending at
   handoff; no current GitHub status is asserted. A probe rerun's exact-key hit does not prove
   a new PR can restore a trusted base cache. The plan checkout at `aaa0787f1` omits the CAD
   qualification target from native's cache; probe `008cb78f` included it. Their cache setups
   are not interchangeable. Check scope, key/toolchain/profile inputs, hit
   rates, transfer cost, eviction, and untrusted/fork write boundaries; never trade correctness
   for a hit. Cold fallback must still run the complete gate.
5. Only after the local phase and the user's return to GitHub work, agree the required-check
   topology and run comparable hosted same-head before/after receipts for the small and core-touch
   reference changes. Record cold and warm cache states, scopes, restore/save time, setup,
   queue/transfer and all shard/job walls, counts and failures. Shards must collectively execute
   the same tests, with missing or failed shards blocking the gate; release dependencies and
   branch protection must not accidentally accept a partial run. Check whether the separate
   security workflow is required; its warm probe audit was 6m28s ([03](issues/03-warm-baseline-measure.md)),
   not evidence of current required-check configuration.

**Success:** all required native and web checks are green on the same head, with unchanged
coverage, and the cold hosted end-to-end full required gate wall is approximately 10 minutes, with
cold versus warm explicitly identified and comparable receipts. Include other required checks
and queue/setup/aggregation in the overall wall if they determine completion. Report cold and
warm separately; a warm hit is not evidence that cold meets the target. Neither local timings
nor the existing warm probe establish that this is achievable on current hosted resources.

The 19.2-minute warm browser wall is already partly parallel, not divisible serial work; neither
19.2/3 nor a hypothetical shard estimate predicts the new wall. In the recorded native run,
replacing CAD's 640s with the existing 426s workspace lane would still leave roughly 11 minutes
including historical serial steps and setup; that is counterfactual arithmetic, not a measurement
or a proof that a particular split works. Runner capacity, cache access, artifact compatibility,
contention and real shard balance remain unknown. The user has not preselected runner size,
cost policy, cache layout, artifacts, or a specific native/web topology; the chief-of-staff may
decide cache and runner policies under the dated grant above.

## Earlier destination (historical)

PR validation — the PR-gate tier of the `native` and `web` jobs — completes in under 17 minutes
on a warm cache (about half the 33m37s cold full-gate baseline) for both reference change sets:
the canonical small PR and the worst-case core-touch PR (both defined in ticket 10). Every test
is explicitly assigned to a tier (untiered stays in the gate by default), and every hang is
attributable from the log alone.

## Notes

- Vocabulary (effort-local; the product `GLOSSARY.md` is untouched): *validation wall* is
  `max(native, web)` job walls including setup and cache restore; *step time* is the `check.py`
  per-step walls; *cold* is rust-cache miss plus wasm-pack miss; *warm* is both hit; *green/red*
  is full success versus test failure; *infra kill* is runner shutdown (e.g. exit 143) with no
  test failure; *receipt* is before/after walls plus test counts plus cache state on the same head.
- Standing rules: no changes to the `stack/02-split-board` (PR #8) branch from this work; every
  workflow change ships with before/after wall-time receipts on the same head plus a green full
  gate; check-speed candidates (check-speed tickets 11–13) are re-measured with fresh receipts, never claimed
  on old numbers; `run-wasm-tests --all` stays split (OOM/SIGKILL risk); lockfile-changing removals
  land after 03 scores the warm baseline and re-measure after.
- Historical tiering policy (superseded for the full-gate goal above): the PR gate would run the
  minimal viable set (10/09), with the full suite required on the main/release path.
- Sessions consult the [check-speed map](../check-speed/map.md) and tickets
  [11](../check-speed/issues/11-browser-measure.md),
  [12](../check-speed/issues/12-browser-quick-wins.md),
  [13](../check-speed/issues/13-browser-structural.md) before touching an overlapping candidate.
- Green baseline: run [40791505198](https://github.com/runinit/boardstudio/actions/runs/40791505198),
  head `d650e1ddf` — web 33m37s (build 624.3s + browser 1363.4s), native 30m19s, both cold.
  Receipt: `.evidence/web-ci-green-baseline-d650e1dd/receipt.md` (gitignored).
- Historical 17-minute checkpoints, superseded for the full-gate goal: 03 scored warm web
  26m03s / native 14m41s; 10 predicted a reduced PR gate of ~12 min for the typical set.
  Neither establishes the current full-coverage target.
- Evidence for resolved research lives on off-tree `research/*` branches (pushed, no PR), not on
  `dev`; tickets 03 and 05 depend on those branches surviving — promote anything load-bearing
  to `dev` before closing them out.

## Tickets

- [01: Cache-branch readiness](issues/01-cache-readiness.md): verdict on landing
  `ci/improve-cache-speed` and its predicted warm gain.
- [02: Provider-build overlap](issues/02-provider-overlap.md): can the six serial provider builds
  overlap safely?
- [03: Warm-baseline measurement](issues/03-warm-baseline-measure.md): land the cache fix on a
  probe branch and record the warm receipt.
- [04: Web-split design](issues/04-web-split-design.md): shard map plus artifact sharing for the
  web job.
- [05: Provider-build parallelism](issues/05-provider-parallelism.md): implement and A/B with
  receipts.
- [06: Page `--list` removal](issues/06-page-list-removal.md): fresh-receipt re-measurement under
  the check-speed-12 protocol.
- [07: Retry-and-heartbeat policy](issues/07-retry-and-heartbeat-policy.md): merge heartbeat
  logging and adopt an infra-retry policy.
- [08: Dead dependencies](issues/08-dead-dependencies.md): itemized removal list with per-item
  evidence.
- [09: Implement tiered PR gate](issues/09-tiered-pr-gate.md): suspended reduced-coverage work.
- [10: Tier map driven by rules](issues/10-tier-map.md): resolved historical tiering research,
  superseded for execution.
- [11: Redundant-test removals](issues/11-redundant-test-removals.md): suspended removals.
- [12: Land the extracted cache change on dev](issues/12-land-cache-change.md): cherry-pick the
  five commits with a green full gate.
- [13: CI browser debug profile](issues/13-ci-debug-profile.md): set the browser profile to
  `false` on CI, same-head A/B; owned here, cross-linked with check-speed 12.
- [14: Native warm wall and budgets](issues/14-native-warm-wall.md): break down the first warm
  native run and set per-step budgets.
- [15: Compile-demand tuning caps](issues/15-tuning-caps.md): existing jobs × lanes work and
  claim preserved; further matrix work deferred under the near-term focus above.
- [16: Web partition completeness prototype](issues/16-web-partition-proof.md): resolved as a
  local, non-executing proof; ticket 04's human design decision remains open.
- [17: Native topology and critical path](issues/17-native-topology.md): coordinator-owned triage
  and coverage-preserving measurement follow-up; no implementation claim yet.

## Historical decisions and results

The 17-minute tiering conclusions below are historical and superseded for the current full gate;
they do not authorize test removals or reduced PR coverage.

- [01: Cache-branch readiness](issues/01-cache-readiness.md): do not land
  `ci/improve-cache-speed` as-is (stack product code); extract the five cache commits as a
  standalone change — predicted warm wall ~22–27 min.
- [02: Provider-build overlap](issues/02-provider-overlap.md): conditional GO for three-lane
  overlap (serial wasm-pack lane, concurrent CAD-WASM + ergogen lanes, no target-dir
  isolation) — predicted ~90–110s off the cold build.
- [03: Warm-baseline measurement](issues/03-warm-baseline-measure.md): scored on draft PR #11
  — cold native 29m22s / web 42m17s, warm native **14m41s** / web **26m03s** (inside the
  predicted 22–27; 17 needs ~9 more min from split/workload).
- [10: Tier map driven by rules](issues/10-tier-map.md): PR-gate ~12 min warm for the
  canonical small PR, ~full wall for core-touch (gate stays lint+build bound); 1 proven
  redundant test, 3 must-not-remove, browser smoke pins are placeholders pending runs.
- [14: Native warm wall and budgets](issues/14-native-warm-wall.md): native FITS — warm
  14m41s with 57s headroom at budget; CAD lane sets the wall (~20s margin); no follow-up
  required for 17.
- [08: Dead dependencies](issues/08-dead-dependencies.md): 16 removable edges, ~zero wall
  saving (all stay reachable via other edges) — hygiene only; catalogue loading and
  mechanical-settings fields re-scoped as non-Cargo follow-ups.
- [06: Page `--list` removal](issues/06-page-list-removal.md): evidence supports check-speed
  12 — executed set identical (77/77 x4), warm saving ~5s, cold compile relocates to the
  runner.

## Not yet specified

- `dx` page-build 311s internals: profile the cold page build before deciding anything about it.
  Follow-up from 05: the `dx` compile is provider-independent (byte-identical output
  with/without staged providers) — pipelining it ahead of the provider batch is viable.
- Conditional-skip policy (`--skip-providers`/page on untouched paths): not approved for the
  current full-coverage gate; any future artifact reuse needs 04's decision and 01's trust rules.
- Job-count tuning matrix (`BOARDSTUDIO_CHECK_JOBS`, cargo jobs on 4 vCPUs): deferred; any
  future work needs the warm baseline as comparison.
- Pooling drag smoke with the page crate batch (check-speed 12's top item, ~50–70s est.):
  check-speed-12-owned; counts toward the wall only via 12's own receipts, cited here as a dependency.

## Out of scope

- Any change to the `stack/02-split-board` branch (PR #8 fixes stay on the PR thread).
- Release-build/deploy optimization: `release-build`/`release-deploy` skip on PRs, so they never
  move the PR wall.
- Check-speed's settled out-of-scope (one combined test binary, `sccache`/`mold`, shared
  `chromedriver`): adopted by reference from the
  [check-speed map](../check-speed/map.md#out-of-scope); reopen only with fresh evidence.
