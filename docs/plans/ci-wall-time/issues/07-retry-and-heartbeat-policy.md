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
  PR #9 (`stack/03-toolbar-polish`, open and mergeable) lands, per user call. PR #9 has now
  merged — heartbeat PR opened as https://github.com/runinit/boardstudio/pull/12
  (`ci/heartbeat-logging` → `dev`; rebased with zero conflicts, `test-check.py` 31/31).
  First CI: audit/native green, web FAILED on infra (runner lost communication 56m in, no test
  failure — first real case for the new timestamps). Failed web job re-run 2026-10-29 per the
  adopted policy. Merge on green; user's click.
- Open policy call: which log signature counts as infra (exit 143 with no test failure?) and
  where a retry lives (workflow-level rerun, not test-level repeat).
- Resolve with the user: confirm the merge after PR #9 lands, decide the retry rule together.
- Policy decided 2026-10-29 (user): retry signature is exit 143 / runner shutdown with no test
  failure in the logs (D1a-a; broad silence-timeouts rejected — heartbeat makes silence
  attributable, auto-retrying real hangs would burn ~35 min relearning it); retries are manual
  human re-runs on that signature, documented not coded (D1b-a; automate only if infra kills
  recur). Ticket stays open until the heartbeat PR merges.
