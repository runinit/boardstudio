# 09: Implement tiered PR gate

Status: needs-info
Type: task
Blocked by: [03: Warm-baseline measurement](03-warm-baseline-measure.md), [10: Tier map driven by rules](10-tier-map.md)
Parent: [CI wall time map](../map.md)

> **Suspended 2026-10-29:** The reduced-coverage PR gate below and its implementation are not
> authorized for the [full-coverage goal](../map.md#full-gate-destination-and-sequence--2026-10-29).
> Preserve this history and do not promote the branch. Check the recorded owner's activity before
> changing any claim or status; no current owner state is inferred here.

## Question

Wire 10's tier map into the workflow and `check.py`, and measure the PR-gate wall versus the
full suite on the same head.

## Pointers

- Implement exactly 10's map, nothing more. Tier ONLY when `github.event_name ==
  'pull_request'`: full `native` plus `web` run on every push to `dev`/`main` and on
  `workflow_dispatch` (dispatch also releases — a reduced gate must never ship). Keep job names
  `native`/`web` stable for `release-build: needs` and required status checks.
- `check.py quick` is the likely PR-gate engine — promote or extend it rather than inventing a
  second selector; keep `--only`/`--keep-going` semantics intact. Resolve 10's build conflict
  (`quick` skips builds while the gate keeps build) as 10 specifies.
- 08's dep removals land here too, sequenced after 03 (lockfile churn invalidates caches);
  test removals graduate to 11.
- Measure against 10's reference change sets (canonical small PR plus worst-case core-touch PR).
- Tier moves never touch the count-guard baseline; only a real removal updates it, and only from
  a full-tier run.
- Open points for the CI arm: (1) this branch's own diff can't exercise tiering (quick can't
  map `.github/`+`scripts/` to a crate) — the measurement draft PR needs the Set A/B change
  sets layered on top; (2) `contracts/` is treated as data by quick, so a contracts change
  gets only smoke tests — one-line fix in `check_quick.py` proposed, awaiting approval as it's
  outside 10's map; (3) the guard's success message overstates partial runs ("all 2104
  executed") — wording fix folded into the CI arm. (4) 10's findings doc is truncated at §5
  in git (§§6–8 missing incl. mutation evidence) — recovery noted on ticket 10; 11 needs it.
- Acceptance: same-head before/after walls for both tiers, every test assigned to a tier
  (untiered means PR gate), green full gate on both tiers.

## Comments

- Claimed for implementation delegate (moved to Opus after a provider 403 on the default key)
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3A37676941-30cf-449b-a631-3e4006798da2`
  (implement + local verify; branch `research/tiered-gate`, no PR; CI measurement waits on a
  separate draft-PR approval).
- Implemented 2026-10-29 on `research/tiered-gate` (`e1046a9ad`, pushed, no PR): gate runs
  only on `pull_request` as `quick --gate native|web`; full commands on dev/main pushes and
  dispatch; job names unchanged; `--base`/`BOARDSTUDIO_QUICK_BASE` diff base with fail-closed
  missing base; `build --fast` first in the web half; count guard hardened (smoke runs never
  checked, `--update-native-baseline` refuses non-full runs, `--gate`+baseline combo rejected).
  Local green: test-check 31, test-check-quick 23 (13 new), test-check-native-tests 35;
  8/8 mutants killed; Set A native test step actually run green in 170.7s cold.
