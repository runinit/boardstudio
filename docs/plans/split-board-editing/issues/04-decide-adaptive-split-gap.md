# 04: Decide adaptive split gap

Status: resolved
Type: grilling
Blocked by: [01](01-split-gap-and-outline-merge.md)
Map: [map.md](../map.md)

## Question

What is the adaptive gap rule? Settle the formula, stored vs derived gap,
existing-board behavior, and MCU clearance during onboarding.

## Outcome

A decision: clearance formula, stored-vs-derived, whether existing saved boards
get re-spaced or keep their gap, and MCU snap/clearance during onboarding drags.
Record under the map's Decisions so far with a link here.

Round 1 settled: Q1/Q2 gap default + no migration and Q5 CHANGED (MCU adjusts spacing) recorded in map; round 2 pending.

Round 2 settled: Q6 symmetric widening (MCU unmoved, one Undo, status names gap), Q7 ALWAYS auto-adjust (supersedes onboarding-only), Q8 clearance 2x outline margin recorded in map; round 3 pending.

Round 3 settled (final): Q11/Q12 recorded in map.

## Answer

- Gap default adapts at creation from projected key envelopes; the Split gap is stored and editable, never auto re-spaced after creation (Q1).
- Existing saved boards keep their stored gap on open; no migration (Q2).
- MCU drop does NOT refuse; spacing adjusts to accommodate (Q5, changed).
- MCU drop widens the gap symmetrically around the split axis; MCU unmoved; one Undo entry; status names the new gap (Q6).
- Auto-adjust applies ALWAYS when space is needed, any placement, not onboarding-only (Q7, supersedes onboarding-only recommendation).
- Minimum clearance is 2x the outline margin; islands separate by construction (Q8).
- Auto-adjust fires on ANY committed edit (placement, moves, matrix growth, rotation) whose result would make halves' envelopes touch; widening is part of that same Undo entry (Q11).
- No cap on widening; never refuse for size (Q12).
