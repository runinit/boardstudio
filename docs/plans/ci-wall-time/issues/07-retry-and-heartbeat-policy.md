# 07: Retry-and-heartbeat policy

Status: ready-for-human
Type: grilling
Parent: [CI wall time map](../map.md)

## Question

Merge the heartbeat logging from `ci/heartbeat-logging` (commit `3e5dd5f6`, merge held for
PR #9) and adopt a retry policy that retries only runner-shutdown signatures — never test
failures.

## Pointers

- Heartbeat logging merged to isolated branch `ci/heartbeat-logging` (commit `3e5dd5f6`,
  pushed, no PR): `+ <cmd> [start …]` lines at parallel-launch submission, `[start/end]` stamps
  on serial lines, no new `monotonic()` calls; `test-check.py` 31/31. Merge to `dev` held until
  PR #9 (`stack/03-toolbar-polish`, open and mergeable) lands, per user call.
- Open policy call: which log signature counts as infra (exit 143 with no test failure?) and
  where a retry lives (workflow-level rerun, not test-level repeat).
- Resolve with the user: confirm the merge after PR #9 lands, decide the retry rule together.
