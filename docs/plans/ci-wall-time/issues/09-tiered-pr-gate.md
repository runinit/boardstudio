# 09: Implement tiered PR gate

Status: claimed
Type: task
Blocked by: [03: Warm-baseline measurement](03-warm-baseline-measure.md), [10: Tier map driven by rules](10-tier-map.md)
Parent: [CI wall time map](../map.md)

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
- Acceptance: same-head before/after walls for both tiers, every test assigned to a tier
  (untiered means PR gate), green full gate on both tiers.

## Comments

- Claimed for implementation delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3A9f903478-42c1-41e7-989a-40533aecdf97`
  (implement + local verify; branch `research/tiered-gate`, no PR; CI measurement waits on a
  separate draft-PR approval).
