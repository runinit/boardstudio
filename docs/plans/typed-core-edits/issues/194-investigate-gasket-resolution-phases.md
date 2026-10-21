# 194: Investigate explicit gasket-resolution phases

Status: ready-for-agent
Type: research
Blocked by: None (can start immediately)
Priority: Opportunity 10; exploratory

## What to build

A verdict on whether base effective-input resolution and gasket enrichment can be separated, replacing the recursion-control boolean, with either a bounded implementation ticket or a documented no-change verdict.

## Starting evidence

Found by a Gortex pass over the current source; verify before relying on it.

- `effective_inputs` calls `effective_inputs_inner` with the flag on; `author_resolved_gasket_support` calls it with the flag off. Those are the only two callers.
- Only the physical-instance branch reads the flag, and only to decide whether to compute the pair's gasket supports. The board branch ignores it.
- In that branch, the gasket-support computation takes the finished configuration as input, and the other fields (parts, contours, findings) are built by non-failing helpers. That suggests the flag-off result is a prefix of the flag-on result: a base resolution, with gasket supports added afterwards.
- `author_resolved_gasket_support` also removes gasket-support features from the cloned document before re-resolving, to stop a two-way linked pair recursing. That is a second termination guard; decide whether the phase split makes it unnecessary or whether it stays.

## Questions to answer

- Is flag-off exactly the base, so enrichment can run after it with no change in error order or results?
- Does anything else depend on gasket supports being computed during the same call (linked-pair features, mirrored boards, flipped instances)?
- Does a split remove the boolean and the cloned-document guard, or only move them?

## Acceptance criteria

- [ ] Record the current call graph and behavior of effective-input resolution, gasket enrichment and linked-pair handling, with the callers affected.
- [ ] Append under `## Answer` either a bounded implementation ticket (scope, tests, risks; it must remove the boolean) or a no-change verdict with reasons.
- [ ] If an implementation is proposed, include the characterization tests that must pass before and after (linked pair, two-way link, flipped instance, board target).
- [ ] No source change in this ticket.

## Execution requirements

Use native Gortex MCP, not CLI or shell navigation fallbacks. Start with task exploration, resolve every target with search, and use relations and trace for callers. Make no source mutation. If the exact checkout view or required tools are unavailable, stop and report the blocker.

## Comments

Approved by the user on 2026-10-21. Tickets are independently startable; priority is scheduling, not a blocking edge. Coordinate overlapping source edits rather than inventing prerequisites.

Revised 2026-10-21 after a Gortex pass over each target's current source: the simplification targets below name the duplication actually found. Re-verify them against the code before starting; if one no longer holds, record that and shrink the ticket rather than inventing a replacement.
