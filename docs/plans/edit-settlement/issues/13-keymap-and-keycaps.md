# 13: Keymap and Keycaps edits land through resolution

Status: ready-for-agent
Type: build
Blocked by: 06
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

These panels already send field-scoped operations, so a queued edit can't overwrite
another. What they lack is one settlement: four `Pending*` structs, four content
checks with their own "no longer matches… retry" messages, and guards that drop a
second edit silently (bindings) or refuse it. Move them onto the edit ticket with
resolvers that check the target still exists, delete the structs and checks, and
let field edits queue.

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

From [inventory.md](../inventory.md) (line numbers at `3368825`, orientation only):

| # | Action | Submit | Kind | Settlement to delete |
|---|---|---|---|---|
| 36 | Macro add/edit/remove | `web/crates/keymap/src/keymap/macro_controller.rs:344` | edit: field edit; add/remove: one-shot | `PendingMacroEdit`, `intent_applied` |
| 37 | Key/encoder binding edit | `web/crates/keymap/src/keymap/binding_controller.rs:924` | field edit | `PendingBindingEdit` |
| 38 | Layer add/rename/remove | `web/crates/keymap/src/keymap/layer_controller.rs:492` | rename: field edit; add/remove: one-shot | `PendingLayerEdit`, `is_applied_to` |
| 39 | Keycap settings (board, matrix, key) | `web/crates/keycaps/src/keycaps_settings.rs:1340` | field edit | `PendingEdit`, `change_is_applied` |

## Background you need

- All four use `EditKeymap` or the keycap `Set*` operations (field-scoped). The
  resolver checks the layer, key, encoder, macro, matrix or board still exists and
  submits; otherwise it retires ("This layer no longer exists").
- New layer or macro IDs must be chosen in the resolver against the accepted
  document, so two queued adds don't collide.
- Keycaps re-checks over time whether its "Saved" status is still relevant
  (`keycaps_settings.rs:~415`). With landings, show the edit ticket's settlement and
  drop the re-check.
- Bindings today drop a second edit with a silent `return` (`binding_controller.rs:~825`).
  That guard goes; both edits queue.

## Acceptance criteria

- [ ] Rapid test: two binding edits on different keys committed back-to-back both land; Undo removes them in order.
- [ ] Rapid test: add layer twice queued; two layers with distinct IDs.
- [ ] Renaming a layer removed before execution retires with a reason.
- [ ] All four `Pending*` structs and content checks, and their messages, are gone.
- [ ] Add/remove controls are disabled while pending; field edits are not.

## Verification

```sh
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/keymap --locked --lib
wasm-pack test --headless --chrome web/crates/keycaps --locked --lib
```

## Out of scope

- Keymap or keycap UI changes beyond pending/failed display.
