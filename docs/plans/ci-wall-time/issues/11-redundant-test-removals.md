# 11: Redundant-test removals

Status: needs-info
Type: task
Blocked by: [10: Tier map driven by rules](10-tier-map.md), [09: Implement tiered PR gate](09-tiered-pr-gate.md)
Parent: [CI wall time map](../map.md)

> **Suspended 2026-10-29:** Do not remove tests or update baselines under this historical tier
> plan. Full coverage is required by the [current goal](../map.md#full-gate-destination-and-sequence--2026-10-29).
> Check ownership before any claim or status change; the instructions below are historical.

## Question

Remove the tests 10 proves redundant, with per-item justification and a green full gate.

## Pointers

- Only items 10 lists with mutation evidence (another test kills the same mutants) — nothing
  hand-picked here.
- Tier moves never touch the count-guard baseline; only a real removal updates it, and only from
  a full-tier run (`--update-native-baseline` deliberately, never from a PR-tier subset).
- Acceptance: same-head before/after walls, green full gate on both tiers.
