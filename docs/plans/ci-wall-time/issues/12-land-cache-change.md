# 12: Land the extracted cache change on dev

Status: ready-for-agent
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
