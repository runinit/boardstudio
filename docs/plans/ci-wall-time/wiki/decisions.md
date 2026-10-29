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
- On 2026-10-29, the user authorized the chief-of-staff to decide cache and runner policies
  for the approximately 10-minute cold full gate. This grants decision authority, not approval
  to purchase or provision resources or trigger hosted CI. No specific policy is selected by
  this grant; full coverage, cache trust and the local-first constraints remain in force.
  Source: user direction in the parent conversation, 2026-10-29.
- On 2026-10-29, under that grant, the chief-of-staff selected a reversible starting policy:
  - A cold cache miss runs the complete full gate; caches optimize but never define correctness.
    Do not manipulate mtimes. Treat cache data as untrusted and keep secrets out.
  - Start with Rust-cache saves on trusted main/dev as in the baseline at `aaa0787f1`;
    introduce no new PR-scoped Rust-cache saves until scope, trust, reachability, quota and
    eviction are validated. The PR #13 proposal remains pending, not rejected. Reuse same-run
    artifacts only after checking source, toolchain, profile, features, provenance and transfer cost.
    See [cache readiness](../issues/01-cache-readiness.md) and [cache change](../issues/12-land-cache-change.md).
  - Start with existing standard hosted runner classes and measured independent job parallelism;
    preserve every test, failure propagation and fail-closed aggregation. Revisit sizing after
    actual availability, cost and critical-path evidence. This baseline is not claimed to meet
    10 minutes cold.

  This is not approval to run hosted CI, buy or provision runners, skip acceptance, or block
  future improvements. The user's topology authorization, local-first constraints and separate
  cold/warm tracking remain. Source: parent selection under the user grant of 2026-10-29; see the
  [full-gate plan](../map.md#full-gate-destination-and-sequence--2026-10-29) and `6ec74bb`.
- On 2026-10-29, under the user's delegated topology, cache and runner authority, the selected
  minimal web design is independent standard GitHub-hosted site and crates jobs, each with a fresh
  workspace and complete local lane validation. The stable `web` aggregate must require both direct
  GitHub job conclusions to be `success`; skipped, cancelled or failed jobs must not pass. Receipts
  and failure artifacts are diagnostic only, with no receipt or artifact transfer on the
  merge-critical path. Preserve release dependencies on `native` and `web` and least token
  privileges. This selects a design, not deployment or ticket completion; see
  [04](../issues/04-web-split-design.md#selected-trust-design--2026-10-29).

  This depends on enforceable review/ruleset protection for the gate workflow **and all**
  gate-defining PR scripts, fixtures, configuration and allowlists. A host job conclusion does
  not prove untrusted PR code honestly performed validation; hostile test code needs stronger
  isolation. Actual branch protection and rulesets are unknown. `needs.result` alone does not
  bind results to the same run attempt on partial reruns: require a full workflow rerun policy
  pending hosted verification, without claiming strict same-attempt evidence. Source: user
  delegation and parent research, 2026-10-29. GitHub documents
  [needs results](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#needs-context),
  [reruns](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs),
  [rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets),
  and [token permissions](https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication#permissions-for-the-github_token).
- On 2026-10-29, the user clarified that tests may be adapted to a new workflow rather than kept
  line for line, while preserving their actual protected behavior and failure detection; coverage
  must not be removed or silently skipped for timing or Node-free goals. For face completeness,
  browser diagnostic rendering and stacks may differ from Node if first failure, original message,
  assertion class/code, actual/expected values (including `-0` versus `0`), and operator survive.
  This does not relax failure detection, assertion order, output handling, CDP errors or async
  requirements. Evaluate async/output against the real regression and intended guarantees, and
  report material detection losses, rather than requiring parity for every artificial perturbation.
  See [ticket 18](../issues/18-face-completeness-replacement.md#test-adaptation-and-assertion-diagnostics--2026-10-29).
- On 2026-10-29, the face ticket was resolved solely for its local replacement decision, so the
  user-approved face-local parity then Ergogen sequence can proceed. The complete browser step
  recommended by the combined reviewer and the Node-free full gate remain required in
  [22](../issues/22-node-free-gate-verification.md); the current browser step still invokes seven
  Node regressions, and the user prohibits Node anywhere in the full gate. This is not approval to
  skip them or a claim of full-gate success. See [18](../issues/18-face-completeness-replacement.md#answer).
