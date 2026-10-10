# 30: Independent parts take the full 360-degree heading rule

Status: claimed
Type: build
Blocked by: [29](29-mirror-switch-and-companion-headings.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-the-heading-rule-in-practice-2026-10-10)

## Request

Ticket 29 found that `mirror_pose` (`core/src/mirrored_half.rs` ~:203) gives independent parts
(everything that is not a switch or a switch companion) the heading `-r`, while ADR-0010's amendment
says the general rule is `180 - r` over the full 360 degrees: a TRRS facing left must face right on
the mirror. The shipped Sofle right half has the TRRS and reset at `-r`, which matches the code and
not the ADR, so the shipped data is the thing to question, not only the code.

## What to do

- Make independent parts follow `180 - r` through the pair map's `forward_angle` over the full 360
  degrees (never modulo 180). Test exactly: TRRS facing left (180) becomes 0, facing right (0)
  becomes 180, facing up (90) stays 90, facing down (270) stays 270; an off-axis LED mirrors by the
  rule. Mutation-check against `-r`.
- Keep switches and their companions as in ticket 29 (switches `-r`, no half turn).
- Report which shipped parts differ (recipes' hand placement, measured fixtures, Sofle TRRS and
  reset). Do not change the recipes (ticket 20) or the shipped data (ticket 25's review); list the
  differences in the Outcome so those tickets can decide.
- Never change a generator parameter, `reversible` or `side`.

## Acceptance

- [ ] The four-heading test and the off-axis LED test pass and are mutation-checked.
- [ ] A list of shipped parts whose hand-placed heading disagrees with the rule.
- [ ] Switch behaviour from 29 unchanged.

## Verification

Core tests, `repo lint test typecheck`, wasm boundary parity if the transcript changes.
