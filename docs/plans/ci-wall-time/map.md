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
   No trivial safe repeat-`wasm-opt` skip was found; do not copy the existing CAD stamp
   shortcut. Investigate its stale-output risk as a focused correctness item, with regression
   coverage for changed build flags and missing JS output, before proposing a fix.
2. Reproduce and fix actual test failures locally, keeping the relevant checks intact. Verify
   each fix with its focused test and any directly affected checks.
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

See [decisions](wiki/decisions.md) and [baseline](wiki/baseline.md) for dated context and
existing measurements. Hosted CI receipts and the original gate criteria remain necessary when
GitHub work resumes; the local check is not a substitute.

## Destination

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
- Tiering policy: the PR gate runs the minimal viable set (10/09); the full suite stays required
  on the main/release path, so no test stops blocking before release — it stops blocking every
  PR push.
- Sessions consult the [check-speed map](../check-speed/map.md) and tickets
  [11](../check-speed/issues/11-browser-measure.md),
  [12](../check-speed/issues/12-browser-quick-wins.md),
  [13](../check-speed/issues/13-browser-structural.md) before touching an overlapping candidate.
- Green baseline: run [40791505198](https://github.com/runinit/boardstudio/actions/runs/40791505198),
  head `d650e1ddf` — web 33m37s (build 624.3s + browser 1363.4s), native 30m19s, both cold.
  Receipt: `.evidence/web-ci-green-baseline-d650e1dd/receipt.md` (gitignored).
- Checkpoints: 03 scored — warm web 26m03s / native 14m41s (native fits 17; web needs ~9 min
  more). 10's predicted PR-gate wall (~12 min typical) is the go/no-go for 17 on the typical
  set; the worst-case set needs the split or a lint/build phase.
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
- [09: Implement tiered PR gate](issues/09-tiered-pr-gate.md): wire 10's tier map into the
  workflow and measure both tiers on the same head.
- [10: Tier map driven by rules](issues/10-tier-map.md): PR-gate versus slower-tier assignment
  by explicit rules, with predicted walls for the reference change sets.
- [11: Redundant-test removals](issues/11-redundant-test-removals.md): remove what 10 proves
  redundant, green gates on both tiers.
- [12: Land the extracted cache change on dev](issues/12-land-cache-change.md): cherry-pick the
  five commits with a green full gate.
- [13: CI browser debug profile](issues/13-ci-debug-profile.md): set the browser profile to
  `false` on CI, same-head A/B; owned here, cross-linked with check-speed 12.
- [14: Native warm wall and budgets](issues/14-native-warm-wall.md): break down the first warm
  native run and set per-step budgets.
- [15: Compile-demand tuning caps](issues/15-tuning-caps.md): existing jobs × lanes work and
  claim preserved; further matrix work deferred under the near-term focus above.

## Decisions so far

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
- Conditional-skip policy (`--skip-providers`/page on untouched paths): needs an artifact cache
  first; graduates after 04, must respect the 10 tier map, and any cross-run artifact cache
  written by PR code needs 01's scope rules.
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
