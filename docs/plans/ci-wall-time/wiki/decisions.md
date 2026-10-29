# Decisions

Dated decisions. Newest last. Each entry stands until a later dated entry supersedes it.

- On 2026-10-29, the user set the destination: the PR-gate tier clears **17 minutes warm for
  both** the canonical small PR and the worst-case core-touch PR (reference sets in ticket 10),
  not just the typical case. Consequence recorded the same day: tiering alone is not expected to
  cover the worst case (gate stays lint+build bound there), so a lint/build phase follows from
  tickets 10 and 14.
- On 2026-10-29, the user approved tiering with no human in the loop for the lane: tickets 08
  through 11 and 13–14 run as agent work with mechanical rules. Human decisions stay only in
  tickets 04 (web-split design) and 07 (retry policy).
- On 2026-10-29, PR #9 (`stack/03-toolbar-polish`) merged, lifting the hold on the heartbeat
  logging merge (ticket 07) and unblocking the completion wave (03 draft PR, 05 local A/B).
- On 2026-10-29, the user ordered: no changes to the `stack/02-split-board` branch (PR #8) from
  this effort; every workflow change ships with same-head before/after receipts plus a green full
  gate; check-speed tickets 11–13 are re-measured, never claimed on old numbers.
- On 2026-10-29, the user ordered the heartbeat logging merged after PR #9
  (`stack/03-toolbar-polish`) lands. The change is committed as `3e5dd5f` on branch
  `ci/heartbeat-logging` (pushed, no PR); see ticket 07 for the remaining retry-policy half.
- On 2026-10-29, the effort adopted delegate routing: default key first, one retry on a 403,
  then Opus (`claude-opus-5-5`) or Codex for the retry. Rationale: the default key began denying
  every new delegate launch (`client_attestation_denied`) while Opus succeeds first time;
  Opus is reserved for implementation, reviews, and anything that already failed once —
  read-only research stays on the cheapest working key.
- On 2026-10-29, the user tightened routing: fallback for implementation is terra 6.1 xhigh
  (normal speed); Opus only for hard planning or troubleshooting.
- On 2026-10-29, the user set research to terra medium.
- On 2026-10-29, the effort adopted the rule that research evidence lives on off-tree
  `research/*` branches (pushed, no PR): `research/cache-readiness` (`70eedc93`),
  `research/provider-overlap` (`3efcce2`), `research/tier-map` (`d5cc9d2`). Tickets depending on
  them must promote load-bearing content to `dev` before closing.
- On 2026-10-29, the user redirected near-term work toward quick local web-build improvements
  and fixing real failing tests without weakening checks. Defer any new tuning matrix, hosted
  runs and merges until the user returns to GitHub work; do not touch PR #8 or `stack/*`.
  Existing claims and historical CI acceptance criteria remain intact. Concrete local actions
  and verification are in the [map](../map.md#near-term-focus--2026-10-29).
- On 2026-10-29, the user authorized the chief-of-staff thread to take ownership and claim
  related tickets, after checking active ownership and without silently displacing a live owner.
  Record actual claim and status changes in the plan and ticket, keeping them consistent.
  Source: user direction in the parent conversation, 2026-10-29. The focused local-only,
  no-matrix and no-hosted-run constraints remain in force.
- On 2026-10-29, the user changed the target to approximately **10 minutes for the overall full
  required PR gate**, with native and web both green on the same head and no reduced coverage or
  unsafe cache. This supersedes the 17-minute tiered destination and its coverage reduction, not
  the historical measurements or existing claims. Work remains local-first; no new hosted CI,
  PRs, merges, or ticket 15 tuning matrix until the user returns to GitHub. Success requires a
  comparable hosted end-to-end measurement after the local phase, identifying cold and warm
  cache states. The [map](../map.md#full-gate-destination-and-sequence--2026-10-29) records the
  practical sequence, resource and trust constraints, and unproven assumptions; the
  [baseline](baseline.md) records the measured anchors. Source: user direction, 2026-10-29.
- On 2026-10-29, the user said approximately **10 minutes cold** would be great for the full
  gate and authorized topology changes if needed. This is an aspirational objective, not a
  demonstrated result; track cold and warm separately, retain full coverage, and do not infer
  approval of an unsafe cache or a particular artifact, caching, runner, or cost policy. The
  local-first and no-hosted-CI-until-later constraints above remain. Source: user direction in
  the parent conversation, 2026-10-29.
