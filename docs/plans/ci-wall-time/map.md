# Map: CI wall time on stack PRs

## Destination

PR validation — the PR-gate tier of the `native` and `web` jobs — completes in under 17 minutes
on a warm cache (about half the 33m37s cold full-gate baseline), with every test explicitly
assigned to a tier (untiered stays in the gate by default), and every hang attributable from the
log alone.

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
- Checkpoints: 03 scores real warm walls; then 10's predicted PR-gate wall for the reference
  sets is the go/no-go for 17 minutes. Current gap: ~22–27 min warm predicted, so ~5–10 min
  must come from tiering or the split.
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

## Decisions so far

- [01: Cache-branch readiness](issues/01-cache-readiness.md): do not land
  `ci/improve-cache-speed` as-is (stack product code); extract the five cache commits as a
  standalone change — predicted warm wall ~22–27 min.
- [02: Provider-build overlap](issues/02-provider-overlap.md): conditional GO for three-lane
  overlap (serial wasm-pack lane, concurrent CAD-WASM + ergogen lanes, no target-dir
  isolation) — predicted ~90–110s off the cold build.
- [10: Tier map driven by rules](issues/10-tier-map.md): PR-gate ~12 min warm for the
  canonical small PR, ~full wall for core-touch (gate stays lint+build bound); 1 proven
  redundant test, 3 must-not-remove, browser smoke pins are placeholders pending runs.

## Not yet specified

- `dx` page-build 311s internals: profile the cold page build before deciding anything about it.
- Conditional-skip policy (`--skip-providers`/page on untouched paths): needs an artifact cache
  first; graduates after 04, must respect the 10 tier map, and any cross-run artifact cache
  written by PR code needs 01's scope rules.
- Job-count tuning matrix (`BOARDSTUDIO_CHECK_JOBS`, cargo jobs on 4 vCPUs): needs the warm
  baseline as comparison; graduates after 03.
- Pooling drag smoke with the page crate batch (check-speed 12's top item, ~50–70s est.):
  check-speed-12-owned; counts toward the wall only via 12's own receipts, cited here as a dependency.

## Out of scope

- Any change to the `stack/02-split-board` branch (PR #8 fixes stay on the PR thread).
- Release-build/deploy optimization: `release-build`/`release-deploy` skip on PRs, so they never
  move the PR wall.
- Check-speed's settled out-of-scope (one combined test binary, `sccache`/`mold`, shared
  `chromedriver`): adopted by reference from the
  [check-speed map](../check-speed/map.md#out-of-scope); reopen only with fresh evidence.
