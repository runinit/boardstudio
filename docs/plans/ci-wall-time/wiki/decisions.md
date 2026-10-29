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
- On 2026-10-29, the user ordered: no changes to the `stack/02-split-board` branch (PR #8) from
  this effort; every workflow change ships with same-head before/after receipts plus a green full
  gate; check-speed tickets 11–13 are re-measured, never claimed on old numbers.
- On 2026-10-29, the user ordered the heartbeat logging merged after PR #9
  (`stack/03-toolbar-polish`) lands. The change is committed as `3e5dd5f` on branch
  `ci/heartbeat-logging` (pushed, no PR); see ticket 07 for the remaining retry-policy half.
- On 2026-10-29, the effort adopted the rule that research evidence lives on off-tree
  `research/*` branches (pushed, no PR): `research/cache-readiness` (`70eedc93`),
  `research/provider-overlap` (`3efcce2`), `research/tier-map` (`d5cc9d2`). Tickets depending on
  them must promote load-bearing content to `dev` before closing.
