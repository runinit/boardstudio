# Map: Edit settlement

Label: wayfinder:map
Spec: [spec.md](spec.md) · Source: [architecture review](../../investigations/architecture-review-2026-10-06.html)
(candidates 1, 3 and 2, in that order)

## Destination

Every product edit is submitted as intent, resolved against the accepted document
when it runs, and settled through one edit ticket with an exact landing. Mounted
tests assert accepted results through an in-process Core adapter. Then a decision on
which remaining whole-document replacements become typed Core edits.

## Notes

- Domain: keyboard design workbench; use [CONTEXT.md](../../../CONTEXT.md) terms
  (accepted document, draft value, pending edit, landed edit).
- Skills for each session: `tdd` for build tickets, `codebase-design` for seam
  questions, `grilling` + `domain-modeling` for decision tickets, `code-review` before
  resolving a build ticket.
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- Build tickets are execution slices for implementation agents; decision tickets
  need the user (HITL).

## Decisions so far

- Queued edits resolve against the accepted document at execution; vanished targets
  retire: [ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md).
- Pending fields keep the draft; failure or retirement reverts to the accepted value
  with an inline message; no automatic retry: [ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md).
- Runtime ports cover Core and persistence only in this effort: [spec](spec.md).
- Plans live as committed Markdown under `docs/plans/`: [issue tracker](../../agents/issue-tracker.md).

## Progress

<!-- one line per resolved build ticket -->

## Not yet specified

- Cluster migration tickets for panels other than the Layout Inspector (being
  sliced from the call-site inventory).

## Out of scope

- CAD/export/preview ports and export capture leases (review candidates 3 remainder, 7).
