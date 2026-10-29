# 12: Land the extracted cache change on dev

Status: claimed
Type: task
Blocked by: [03: Warm-baseline measurement](03-warm-baseline-measure.md)
Parent: [CI wall time map](../map.md)

## Question

Cherry-pick the five cache commits onto `dev` with a green full gate, so the warm gain reaches
stack PRs.

## Pointers

- Exactly the five commits from 01's verdict (stable key plus qualification target, per-PR
  cancellation, same-repo PR-scope saves, dependabot exclusion, audit doc) — nothing else from
  `ci/improve-cache-speed`. Expect cherry-pick conflicts: those commits touch `check.yaml` and
  `check.py`, which differ between branches — resolve in favor of the target tree plus the cache
  hunks.
- Wire `scripts/test-ci-workflow.py` (arrives with the cherry-picks) into `check.py`'s
  script-test list on `dev`, so the cache matrix is actually exercised going forward.
- Landing day invalidates existing `dev` caches: say so in the merge, expect one cold transition
  run, then re-score the warm wall.
- Acceptance: green full gate on `dev`, cache-hit receipts on the next stack-PR run.

## Comments

- Claimed for implementation delegate (moved to Opus after a provider 403 on the default key)
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3A52057115-c695-471f-9e63-a57ba37c1ce5`
  (cherry-pick onto current dev, PR, CI to green; merge is the user's click).
