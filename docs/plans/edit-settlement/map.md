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

## Decision preparation

The [review and decision brief](decision-brief.md) records the Astra/high follow-up
and the open questions for tickets 18 and 19. The [cleanup audit](../../investigations/edit-settlement-cleanup-audit.md),
[Event edit options](../../investigations/edit-event-restriction-options.md) and
[whole-document resolver inventory](replace-document-resolvers.md) are provisional
until ticket 15 merges and ticket 17 completes; they do not resolve those tickets.

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
- 02 Settlements report where an edit landed; `Effect::Settled` carries a `Landing`, retried
  saves re-report the original: [issues/02-settlement-reports-landing.md](issues/02-settlement-reports-landing.md).
- 03 Session resolves pending edits against the accepted document; `Event::ResolveEdit`
  submits intent, resolved at execution: [issues/03-session-resolves-pending-edits.md](issues/03-session-resolves-pending-edits.md).
- 04 The edit ticket module landed in the web runtime crate: one port, one outcome
  mapping, standard wording, `is_pending`: [issues/04-web-edit-ticket.md](issues/04-web-edit-ticket.md).
- 05 Layout Inspector mounted tests reach the real Session and Core; the rapid queued X/Y
  regression is a known failure until 06; interception deletion deferred to 11/12:
  [issues/05-layout-inspector-tests-reach-core.md](issues/05-layout-inspector-tests-reach-core.md).
- 06 Tracer bullet landed: the Layout Inspector submits intent through the edit ticket,
  resolvers resolve at execution, rapid X/Y keeps both coordinates and Undo removes only
  Y; the investigation and backlog entry are closed:
  [issues/06-layout-inspector-tracer-bullet.md](issues/06-layout-inspector-tracer-bullet.md).
- 07 Definition-name and generator tests reach the real Session and Core; the definition-name
  interception is gone from Runtime: [issues/07-definition-name-tests-reach-core.md](issues/07-definition-name-tests-reach-core.md).
- 08 Parts custom-definition fields and the definition name land through resolution; the
  dirty-courtyard blur finding is a harness artefact:
  [issues/08-parts-custom-definition-fields.md](issues/08-parts-custom-definition-fields.md).
- 10 Layout objects (align, placement, apply-to-key, mirrored pair, existing half, board,
  keycap size) submit intent through the edit ticket; one-shot controls disable while
  pending; their `Pending*` heuristics are deleted: [issues/10-objects.md](issues/10-objects.md).
- 11 Matrix setup, placement, Matrix Inspector and Transform Inspector, drag commit and handle
  nudges submit intent through the edit ticket; the baseline rejections are gone; the
  transform Inspector tests run on the real Session and Core:
  [issues/11-matrix-and-transform.md](issues/11-matrix-and-transform.md).
- 12 Outline actions go through one planner and the edit ticket; the `Pending` settle
  effect is gone; the layout-inspector interception is deleted from Runtime:
  [issues/12-outline.md](issues/12-outline.md).
- 09 Layout remainder: nudges, the old position Inspector's commit and geometry-script
  New/Apply submit intent through the edit ticket; constraint set/remove covered by
  queued tests: [issues/09-layout-remainder.md](issues/09-layout-remainder.md).

- 16 Case bodies, mechanical settings and project/board names resolve against accepted
  state and settle through edit tickets; fields queue freely, action guards are scoped
  to their controls, and the mechanical closure landing comparison is removed:
  [issues/16-case-and-metadata.md](issues/16-case-and-metadata.md).

- 13 Keymap bindings, layers, macros and Keycaps settings resolve against accepted state
  and settle through edit tickets; rapid edits, field display and Undo are covered:
  [issues/13-keymap-and-keycaps.md](issues/13-keymap-and-keycaps.md).

- 14 PCB wiring, module placement, physical setup and routed-board settings resolve
  against accepted state and settle through edit tickets; pending fields and ordered
  Undo are covered, while electrical remap stays strict:
  [issues/14-pcb-wiring-modules-physical-setup.md](issues/14-pcb-wiring-modules-physical-setup.md).

- 15 Remaining Parts actions resolve against accepted state and settle through edit
  tickets; exact post-landing selection, queued profiles/models and generator
  upload/Apply races are covered:
  [issues/15-parts-remainder.md](issues/15-parts-remainder.md).

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
