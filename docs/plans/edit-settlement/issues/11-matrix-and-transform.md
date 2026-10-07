# 11: Matrix setup, Matrix Inspector and transform edits land through resolution

Status: resolved
Type: build
Blocked by: 06
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Move every matrix editor onto resolvers and the edit ticket: matrix setup and
placement, the Matrix Inspector's fields and presets, delete, unlink, duplicate
variant, the Matrix Transform Inspector, the transform tool's drag commit and its
keyboard nudge. Most of these send a whole matrix (`SetMatrix`) copied from the
document the panel last saw; after this ticket the matrix is read from the accepted
document when the edit runs and only the user's change is applied.

Two visible changes follow from the
[ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-field-edits-and-one-shot-actions-2026-10-06):

- Matrix Inspector fields no longer disable or show "Wait for the current matrix
  change to finish, then retry" while an edit is pending. Field edits queue.
- The Matrix Transform Inspector no longer rejects a commit because "The accepted
  value changed" or "The accepted document changed". The user's committed value
  wins. Its admission checks for selection lifetime and stale callbacks stay.

The transform handle's keyboard nudge becomes a delta intent, like ticket 09's tree
nudge.

## Migration rules (same for every cluster ticket)

Ticket 06 sets the pattern; read its Outcome and copy its shape before starting:
`web/src/presentation/layout_component_edits.rs` (pure resolver builders, one
`EditTicket` per committed field, the settle helper) and
`web/crates/runtime/src/edit_ticket.rs` (`EditTicket::begin(port, label, feature,
resolver)`, `settlement(owner_is_live)`, `is_pending()`; usage example in its module
docs). Ticket 06 left two things for the clusters: it had no one-shot controls, so
the first cluster ticket with one establishes the `is_pending()` disabling; and its
settle helper passes `owner_is_live = true` because the Inspector unmounts with its
owner. Panels that outlive their owner (a selection change keeps the panel mounted)
must pass a real liveness answer. In short:

- Every action in this ticket submits intent through the edit ticket (ticket 04)
  with a resolver. This includes field-scoped operations: their resolver checks the
  target still exists and submits the command, or retires.
- A resolver is a pure function of the accepted snapshot plus values captured at
  submit. It never reads signals or Runtime state. Whole-document actions move their
  clone-and-mutate step into the resolver, so `ReplaceDocument` is built from the
  accepted document at execution.
- **Field edits** queue freely: remove the panel's "refuse while pending" guard and
  never disable the field for a pending edit. **One-shot actions** disable their
  control, with no message, while their ticket `is_pending()`. See the
  [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-field-edits-and-one-shot-actions-2026-10-06)
  and the terms in [CONTEXT.md](../../../../CONTEXT.md).
- The latest committed value wins. Retire only when the target is gone or no longer
  eligible, with a reason the user understands. If the value already equals the
  accepted value, resolve **Unchanged**.
- Delete the panel's `Pending*` settlement struct and its "did it land" heuristics
  (token/revision/durability checks, whole-document equality, revision = base+1,
  content checks and their "does not contain the requested value… retry" messages).
  Landed means landed; the field then shows the accepted value.
- Failure wording comes from the edit ticket. Keep only a feature noun.
- Keep admission checks at dispatch (selection lifetime, stale callbacks, owner
  identity) so stale UI is still ignored early.
- Previews stay on `Event::Edit` with the preview phase; only commits become
  intents.
- Tests assert accepted results (the accepted document, settlement, Undo and the
  field text the user sees), not command shapes or `Pending*` internals. Each ticket
  adds at least one rapid-entry test for its riskiest action: two commits queued
  behind a gated Core reply both survive, and Undo removes them in order.

## Call sites

From [inventory.md](../inventory.md) (paths relative to `web/crates/layout/src/objects/`;
line numbers at `3368825`, orientation only):

| # | Action | Submit | Kind | Settlement to delete |
|---|---|---|---|---|
| 12 | Matrix setup create | `matrix_setup_controller.rs:331` | one-shot, `SetMatrix` | `PendingSetup` |
| 13 | Matrix placement | `matrix_placement_controller.rs:498` | one-shot, `SetMatrix` | `PendingPlacement` |
| 14 | Matrix Inspector fields (name, rows, columns, pitch, switch, diode, edge gap, add row/column) | `matrix_inspector_controller.rs:365` | field edits (add row/column: one-shot), `SetMatrix`/`SetLayout` | `PendingMatrixEdit`, field-value content check |
| 15 | Matrix preset | `matrix_inspector_controller.rs:570` | one-shot, `SetMatrix` | `PendingMatrixPreset` |
| 16 | Matrix delete | `matrix_inspector_controller.rs:658` | one-shot, `RemoveMatrix` | `PendingMatrixDelete` |
| 17 | Unlink mirrored halves | `matrix_inspector_controller.rs:925` | one-shot, `SetLayout` | none |
| 18 | Duplicate design variant | `matrix_inspector_controller.rs:2147` | one-shot, `SetMatrix` after `Open` | `wait_for_variant_edit` |
| 19 | Matrix Transform Inspector | `matrix_transform_controller.rs:465` | field edits, `SetMatrix`/`SetMatrixSplay` | `PendingTransformEdit`, baseline rejection |
| 20 | Transform tool drag commit | `layout_transform_toolbar.rs:1119` | field edit, `SetMatrix`/`SetMatrixSplay` | none |
| 21 | Transform handle keyboard nudge | `layout_transform_toolbar.rs:682` | delta intent, `SetMatrix`/`SetMatrixSplay` | none |

## Background you need

- #18 opens a cloned document, waits for it, then sends one `SetMatrix`, and reopens
  the source if that edit fails (`restore_source_after_variant_failure`). Keep the
  open and the restore. The `SetMatrix` becomes an intent whose resolver retires if
  the source matrix is not in the accepted document; drop `exact_variant_snapshot`'s
  token/revision equality, since the session epoch already rejects intents from
  before the open.
- #20's drag sends previews while moving and a preview then a commit at drag end.
  Previews stay; only the commit becomes an intent. The commit's resolver applies
  the drag's net transform to the accepted matrix.
- `web/crates/layout/src/matrix_transform_lifecycle.rs` holds the transform
  settlement helpers and their native tests. Delete what the edit ticket replaces;
  keep admission (`AcceptedIdentity::admits` for stale callbacks) if dispatch still
  needs it.
- `matrix_transform_inspector_tests` capture submitted events (inventory section 6).
  Move them to ticket 01's adapter and assert accepted results.
- The investigation asked to keep transform policy separate *during the
  investigation*; ADR-0005's amendment now decides it.

## Acceptance criteria

- [ ] Rapid test: rows then pitch in the Matrix Inspector with replies gated; both survive and Undo removes pitch then rows.
- [ ] Rapid test: two Transform Inspector fields committed back-to-back; both survive.
- [ ] Three handle nudges queued behind a gated reply move the matrix three steps.
- [ ] Editing a matrix deleted before the edit runs retires with a reason.
- [ ] The "accepted value changed" and "Wait for the current matrix change" messages are gone.
- [ ] One-shot controls (setup, placement, preset, delete, unlink, duplicate, add row/column) are disabled while pending.
- [ ] All listed `Pending*` structs and heuristics are gone; no action sends `Event::Edit` commits directly.
- [ ] `objects/matrix_transform_inspector_tests.rs` runs against the real Session and Core
  (accepted-result assertions, no interception); whichever of tickets 11 and 12 resolves
  second deletes the layout-inspector interception from Runtime (deferred from ticket 05;
  see its Comments). If ticket 12 is still open, leave the interception and say so in
  the Outcome.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/layout --locked --lib
```

## Out of scope

- Merging held-key nudges or drag previews into one edit (map: out of scope).
- Stale previews drawn after a queued commit (`docs/backlog.md`).

## Outcome

Commits: claim; "Land matrix setup, placement and Matrix Inspector edits through
resolution"; "Land Matrix Transform Inspector, drag commit and handle nudges through
resolution" (plus the review fix folded into the resolve commit).

- Resolvers (all pure, reading the matrix, layout and scene from the accepted snapshot):
  `create_matrix_resolver`, `place_matrix_resolver` (only definitions the accepted document
  lacks), `matrix_field_resolver` (every Inspector field; rebuilds the operation from the
  accepted matrix, resolves Unchanged on an equal value, retires with "The selected matrix
  no longer exists."), `matrix_preset_resolver` (applies the prepared template to the accepted
  matrix; retires if its size changed), `matrix_delete_resolver`, `unlink_layout_resolver`,
  the duplicate-variant preset edit, `transform_edit_resolver` (recomputes the field
  projection from the accepted snapshot via the new `transform_fields`), `transform_drag_resolver`
  (net delta for stagger and splay angle, the dropped point for a splay origin) and
  `transform_nudge_resolver` (delta intent against the accepted offset or column basis).
- Deleted: `PendingMatrixEdit`/`Preset`/`Delete` heuristics and content checks, `PendingSetup`,
  `PendingPlacement` landing checks, `PendingTransformEdit`, `AcceptedIdentity`,
  `PendingSettlement`/`pending_settlement` and their native tests, `wait_for_variant_edit`'s
  revision/content checks, `exact_variant_snapshot`'s token/revision equality. The
  "Wait for the current matrix change to finish", "The accepted value changed" and
  "The accepted document changed" rejections are gone; both Inspectors keep a dirty draft
  and let the committed value win. The Matrix Transform Inspector lost its `busy` flag
  entirely (every control there is a field edit).
- Field edits queue (Inspectors stay editable while an edit is applying or saving); one-shot
  controls (setup, placement, preset, delete, unlink, duplicate) disable while their ticket
  is pending, and Add row / Add column disable while a rows/columns edit is pending.
- Nudge admission (`transform_nudge_admitted`) checks selection, scope, workspace and a live
  session but not token/revision, so queued presses compose.
- `matrix_transform_inspector_tests.rs` now opens its fixtures through the real Session and
  Core and asserts accepted results (the matrix cell offsets, assembly replacement). The
  layout-inspector interception is **not** deleted here: `outline_lifecycle_browser_tests.rs`
  still uses it; ticket 12 deletes it.
- New tests (real Session/Core, gated reply): rows then pitch survive and Undo removes pitch
  then rows; editing a deleted matrix retires with a reason; one-shot ticket stays pending;
  two transform fields back-to-back survive and a committed value wins; three queued
  nudges move the matrix three steps; a drag commit composes with an intervening edit.

Checks: `cargo test -p boardstudio-web-runtime --locked` 103 passed; `cargo test -p
boardstudio-web-layout --locked` 50 passed; `python3 scripts/check.py typecheck`;
`python3 scripts/check-wasm-tests.py`; `python3 scripts/check-doc-links.py`;
`wasm-pack test --headless --chrome web/crates/layout --locked --lib` — 88 passed.
`check.py test` fails only on the missing `step-oracle/` in this worktree; workspace
clippy fails on a pre-existing `chunks_exact` lint in `boardstudio-renderer`.

Follow-ups: the Matrix Inspector and Matrix Transform controllers keep an unused
`last_request_id` ordering guard that no longer serves settlement; drag stagger/angle deltas
use the drag's start offsets, so a drag over a concurrently re-staggered column composes
rather than snaps.
