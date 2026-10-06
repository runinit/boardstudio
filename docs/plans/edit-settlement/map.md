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
- Field edits queue freely; one-shot actions disable their control while pending: [ADR-0005 amendment](../../adr/0005-resolve-queued-edits-at-execution.md#amendment-field-edits-and-one-shot-actions-2026-10-06), terms in [CONTEXT.md](../../../CONTEXT.md).
- The latest committed value wins; resolvers retire only for a vanished or ineligible target, so the Matrix Transform Inspector's baseline rejection goes: [ADR-0005 amendment](../../adr/0005-resolve-queued-edits-at-execution.md#amendment-field-edits-and-one-shot-actions-2026-10-06).
- Landed means landed: per-panel content checks are deleted; mechanical closure evidence is decided in [Case, mechanical settings and project/board names](issues/16-case-and-metadata.md).
- Every action uses a resolver, field-scoped ones included; keyboard nudges become delta intents; previews stay on `Event::Edit`: [cluster migration rules](issues/08-parts-custom-definition-fields.md#migration-rules-same-for-every-cluster-ticket).

## Progress

<!-- one line per resolved build ticket -->

- 01 Runtime Core and persistence ports with an in-process test adapter landed; Core and
  saves go through ports, gates hold/fail replies and saves: [issues/01-runtime-core-and-persistence-ports.md](issues/01-runtime-core-and-persistence-ports.md).

## Not yet specified

Nothing. The cluster migrations are now tickets, sliced from the
[call-site inventory](inventory.md):

- [Definition-name and generator tests reach the real Session and Core](issues/07-definition-name-tests-reach-core.md)
- [Parts custom definition and definition-name fields](issues/08-parts-custom-definition-fields.md)
- [Layout remainder: constraints, old position Inspector, nudges and geometry scripts](issues/09-layout-remainder.md)
- [Objects: align, placement, mirrored halves, boards and keycap size](issues/10-objects.md)
- [Matrix setup, Matrix Inspector and transform edits](issues/11-matrix-and-transform.md)
- [Outline actions](issues/12-outline.md)
- [Keymap and Keycaps edits](issues/13-keymap-and-keycaps.md)
- [PCB wiring, modules and physical setup](issues/14-pcb-wiring-modules-physical-setup.md)
- [Remaining Parts actions](issues/15-parts-remainder.md)
- [Case, mechanical settings and project/board names](issues/16-case-and-metadata.md)
- [Cleanup: one settlement path and a record of what still replaces the document](issues/17-cleanup.md)
- Decisions: [restrict `Event::Edit` at the type level?](issues/18-decide-restrict-event-edit.md) and [which whole-document resolvers become typed Core edits first?](issues/19-decide-first-typed-core-edits.md)

## Out of scope

- CAD/export/preview ports and export capture leases (review candidates 3 remainder, 7).
- Merging held-key nudges or drag repeats into one edit: changes Undo granularity and the gesture path.
- Stale `Event::Edit` previews drawn after a queued commit: never changes the accepted document; tracked in the [backlog](../../backlog.md).
- Cleaning up asset bytes left unreferenced when an upload's edit is retired or fails (unchanged from today).
