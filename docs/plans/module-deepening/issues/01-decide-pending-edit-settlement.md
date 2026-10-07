# 01: Decide the pending-edit settlement rules and module shape

Status: resolved
Type: grilling
Blocked by: —
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Review: [candidate 01](../../../investigations/architecture-review-2026-10-07.html#c1)

## Question

What does one shared module above the edit ticket own, and which settlement policy do
all panels follow where they disagree today?

Line numbers are at `915d305c0`, orientation only.

## Evidence

- 15 `*Submission` holders (`rg -n "struct \w*Submission\b" web`), ~14 feedback enums
  shaped `{Pending, Saved, Failed}`, and settlement read in ~40 files
  (`rg -l "\.settlement\(" web`).
- `settle_ticket` (`web/crates/parts/src/parts_custom_definition.rs:165`) and
  `settle_field` (`web/src/presentation/layout_component_edits.rs:432`) are identical.
- A ticket settles `Retired` for a departed owner, and also for `Superseded`,
  `Cancelled`, `Closed` and `DOCUMENT_SESSION_CHANGED`
  (`web/crates/runtime/src/edit_ticket.rs:130-170`). With the owner still live, the
  Matrix controllers show "did not complete in the active session"
  (`matrix_inspector_controller.rs:1338-1380`, four copies), Keycap size drops it
  silently (`keycap_size_controller.rs:~575`), and `existing_half.rs` and
  `mirrored_pair_controller.rs` branch on `same_lineage` first.
- Four `fn owner_is_live` (`part_placement.rs:1417`, `parts_custom_definition.rs:448`
  and `:999`, `pcb_wiring/part_input_settings/owner.rs:268`); 51 sites compare
  `runtime.scope()` with a captured scope, several also re-checking the snapshot's
  epoch and document ID that `Scope` already encodes.
- Resolvers hand-write `base_revision: 0` (31) and `transaction_id: String::new()` (47),
  which Session overwrites (`application/src/session.rs:~1503`), and check
  `accepted.session_epoch` in 18 places although Session rejects a stale epoch first
  (`session.rs:~1424`).

## Decide

1. **Retired with a live owner.** Silent everywhere, or one standard inline message?
   (Proposed: silent for `DOCUMENT_SESSION_CHANGED` and `Closed`; one standard message
   for `Superseded`/`Cancelled` while the owner is live.)
2. **Owner liveness.** Does the ticket capture the `Scope` at `begin` and answer
   lineage liveness itself, leaving panels only a visibility predicate (selection
   generation, mounted target)?
3. **Shape.** A keyed collection per panel (`PendingEdits<K>`: `begin(key, resolver)`,
   `is_pending(key)`, `settle(…) -> outcomes`), a per-field type, or both? Does a panel
   still need a `Saved` state, given ADR-0005 says a landed field just shows the
   accepted value?
4. **Placement.** Dioxus-free core in `web/crates/runtime` beside the edit ticket;
   Signal-bound text-field and one-shot helpers in `ui-shared`?
5. **Resolver side.** A `Resolution` constructor that leaves phase, base revision and
   transaction to Session, and deletion of resolver-level epoch checks: in this
   module's ticket or separate?
6. **Name.** If the module needs a domain name, record it in `CONTEXT.md`.

## Done when

`## Answer` records each decision; an ADR-0005 amendment is added if decision 1 changes
user-visible behaviour; ticket 05's interface section is filled in from the answer.

## Answer

Decided with the user on 2026-10-07.

1. **Retired with a live owner: silent everywhere.** The field shows the accepted value
   again; the Matrix panels' "did not complete in the active session" message goes.
   Recorded as an [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07).
2. **Owner liveness: the ticket answers lineage.** `EditTicket::begin` captures the
   `Scope` and the ticket retires itself once the current scope differs. Panels pass
   only their own owner predicate (selection generation, mounted target). The four
   `owner_is_live` functions and the redundant epoch/document-ID clauses go.
3. **No `Saved` state.** The module reports pending and failure; a landed field shows
   the accepted value. One-shot actions disable while their key is pending.
4. **Placement.** The Dioxus-free `PendingEdits` module sits in `web/crates/runtime`
   beside the edit ticket, with native tests; Signal-bound helpers for text fields and
   one-shot controls sit in `ui-shared`.
5. **Resolver side: its own ticket**, [04](04-resolution-constructors.md), after edit
   settlement 20 and before the module.
6. **Name: `PendingEdits`.** No new `CONTEXT.md` term; it holds *pending edits* as
   already defined.

Follow-up round:

7. **Several pending edits to one field: the latest ticket per key drives it.** Earlier
   tickets still run in Session; their settlements are not observed.
8. **Where failures appear: the module returns one message per key; the panel chooses
   field, panel or report.** The `ui-shared` helpers default to inline at the field.
9. Recorded in ADR-0005 (see 1).
