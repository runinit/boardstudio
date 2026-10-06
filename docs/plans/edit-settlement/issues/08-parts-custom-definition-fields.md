# 08: Parts custom definition and definition-name fields land through resolution

Status: ready-for-agent
Type: build
Blocked by: 06, 07
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

This is the top risk in the inventory. Each field of a custom part definition
(courtyard width and height, pads, body and so on) and the definition name commit
on blur by cloning the whole document as the panel last saw it and sending
`ReplaceDocument`. Tabbing from width to height sends two replacements built from
the same document, so the height commit silently puts back the old width. Neither
action has a guard or observes its outcome.

After this ticket each field commit is a field edit through the edit ticket, with a
resolver that finds the definition in the accepted document, applies only that
field, and builds the replacement there. Width, Tab, height, Enter keeps both
values, and one Undo reverts only height.

## Migration rules (same for every cluster ticket)

Ticket 06 sets the pattern; read its code and Outcome before starting. In short:

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

| # | Action | Submit | Kind |
|---|---|---|---|
| 60 | Custom definition fields, each on blur | `parts_custom_definition.rs:~139` → `:881` | field edit, `ReplaceDocument` |
| 61 | Definition name, on blur | `parts_definition_name.rs:103` → `:226` | field edit, `ReplaceDocument` |

## Background you need

- Ticket 07 moved this panel's tests onto the in-process adapter; build on those.
- `accept_document_replacement` in `parts_definition_name.rs` shows how the panel
  currently turns a draft into a whole document. Move that work into the resolver.
- Retire when the definition no longer exists in the accepted document (for example
  it was deleted or the project changed), with a reason such as "This part
  definition no longer exists".
- If the definition is built in (not editable) in the accepted document, retire.

## Acceptance criteria

- [ ] Mounted test: width, Tab, height, Enter with Core replies gated; after release both values are in the accepted definition and one Undo reverts only height.
- [ ] Mounted test: rename, then a field edit queued behind it; both survive.
- [ ] Mounted test: failing the save returns the field to the accepted value with the edit ticket's inline message.
- [ ] Mounted test: deleting the definition before the queued edit runs retires it with a reason.
- [ ] Neither action sends `Event::Edit` directly; both submit intents.

## Verification

```sh
python3 scripts/check.py typecheck
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/parts --locked --lib
python3 scripts/check-doc-links.py
```

## Out of scope

- Other Parts actions (ticket 15).
- A typed `RenameDefinition` or per-field Core edit (decision ticket 19).

## Pitfalls

- Blur fires after Enter. Enter then blur must commit once; an untouched field must
  not commit on blur.
- The draft must not become a second copy of the definition. Once the ticket
  settles, render from the accepted document.
