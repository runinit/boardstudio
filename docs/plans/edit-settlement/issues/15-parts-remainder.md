# 15: Remaining Parts actions land through resolution

Status: claimed
Type: build
Blocked by: 06, 07
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Move the Parts workspace actions not covered by ticket 08 onto resolvers and the
edit ticket: new component, KiCad import, component model transform/remove/upload,
generator Apply and asset upload, mechanical and module profiles, module attach and
the assembly actions. Almost all send `ReplaceDocument` built from an earlier
document. The mechanical profile save (#68) is in the inventory's top ten risks.

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

From [inventory.md](../inventory.md) (paths relative to `web/crates/parts/src/`;
line numbers at `3368825`, orientation only):

| # | Action | Submit | Kind | Settlement to delete |
|---|---|---|---|---|
| 59 | New custom component | `parts_new_component.rs:136` | one-shot, `ReplaceDocument` | `PendingCreate`, `reconciliation_is_current` |
| 62 | Import KiCad footprint | `parts_import_footprint.rs:701` | one-shot, `ReplaceDocument` | `PendingImport`, `accepted_import_is_current` |
| 63 | Component model transform | `parts/component_model_editor.rs:249` | field edit, `ReplaceDocument` | `PendingModelEdit` |
| 64 | Component model remove | `parts/component_model_editor.rs:308` | one-shot, `ReplaceDocument` | `PendingModelEdit` |
| 65 | Component model upload | `parts/component_model_editor.rs:658` | one-shot, `ReplaceDocument` | `PendingModelEdit` |
| 66 | Generator Apply | `parts/generator_settings.rs:1235` | one-shot, `ReplaceDocument` | `PendingApply` |
| 67 | Generator asset upload | `parts/generator_settings.rs:1180` | one-shot, `ReplaceDocument` | `PendingApply` |
| 68 | Mechanical profile save | `parts/mechanical_profile_ui.rs:130` | field edit, `ReplaceDocument` | `PendingProfileEdit` (`parts/mechanical_profile.rs`) |
| 69 | Module profile save | `parts/module_profile_editor.rs:593` | field edit, `SetModuleDefinition` | outcome only |
| 70 | Module attach | `parts/modules_catalogue/module_attachment.rs:254` | one-shot, `SetMountedModule` | revision = base+1 |
| 71 | Assembly save | `parts/assembly_editor.rs:466` | one-shot, `ReplaceDocument` | `PendingSave` |
| 72 | Assembly place on board | `parts/assembly_editor.rs:712` | one-shot, `ReplaceDocument` | `PendingSave` |
| 73 | Assembly apply to matrix | `parts/assembly_editor.rs:853` | one-shot, `SetMatrix` | `PendingSave` |

## Background you need

- Ticket 07 moved the generator tests onto the in-process adapter; build on them.
- #59 then selects the new definition, and #72 then selects the placed parts. Keep
  both: on Landed, read the accepted document at the landing and select only what
  exists. New definition and assembly IDs are chosen in the resolver.
- Uploads (#65, #67) store bytes by hash before submitting; the resolver only adds
  the `Asset` metadata. If the edit is retired or fails, the bytes stay in the store
  unreferenced, as today. Accept that; asset cleanup is out of scope.
- #62 carries no asset bytes; it adds a definition to the document.
- #68 was rebuilt from the latest saved document but ignored edits still queued;
  with a resolver it reads the accepted document at execution.
- If this ticket is too big for one session, split it at #59–#65 / #66–#73 by adding
  a comment and asking; don't renumber other tickets.

## Acceptance criteria

- [ ] Rapid test: mechanical profile save queued behind a custom-definition field edit keeps the field edit.
- [ ] Rapid test: generator Apply queued behind a definition rename keeps the rename.
- [ ] Editing a model on a definition deleted before execution retires with a reason.
- [ ] Create, import and assembly-place still select what they created after landing.
- [ ] All listed `Pending*` structs and heuristics are gone; no action sends `Event::Edit` commits directly.
- [ ] One-shot controls are disabled while pending; field edits are not.

## Verification

```sh
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/parts --locked --lib
```

## Out of scope

- Custom definition fields and definition name (ticket 08).
- Cleaning up unreferenced asset bytes.

## Comments

2026-10-07: User authorized takeover after inspection found only the claim commit
and four partial uncommitted files in `edit-settlement-agent-a`. Their patch was
preserved and carried into `edit-settlement-b`; the original worktree is untouched.
Complete this ticket and its checks/review before ticket 17.
