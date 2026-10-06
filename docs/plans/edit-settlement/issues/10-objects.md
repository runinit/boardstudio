# 10: Objects: align, placement, mirrored halves, boards and keycap size

Status: ready-for-agent
Type: build
Blocked by: 06
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Move the Layout objects actions that are not matrix editors onto resolvers and the
edit ticket, and delete their `Pending*` structs. These are mostly one-shot actions:
align, place a component, apply a component to a key, create a mirrored pair, mirror
an existing half, add a board and resize keycaps.

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

From [inventory.md](../inventory.md) (paths relative to `web/crates/layout/src/`;
line numbers at `3368825`, orientation only):

| # | Action | Submit | Kind | Settlement to delete |
|---|---|---|---|---|
| 9 | Canvas Align | `objects/layout_align_controller.rs:357` | one-shot, `SetMatrix`/`MoveParts` | `PendingAlign`, `pending_settlement_gate`, `expected_applied` |
| 10 | Component placement commit | `part_placement.rs:1217` | one-shot, `ReplaceDocument` | `PendingCommit`, `completion_is_accepted` |
| 11 | Apply component to selected key | `part_placement.rs:748` | one-shot, `SetMatrix` | `PendingKeyEdit` |
| 22 | Mirrored pair create | `objects/mirrored_pair_controller.rs:704` | one-shot, `CreateMirroredPair` | `PendingPair`, transaction-id match in `mirrored_pair_lifecycle.rs` |
| 23 | Existing-half mirror | `objects/existing_half.rs:258` | one-shot, `ReplaceDocument` | `PendingExistingHalf` |
| 24 | Add board | `objects/board_setup_controller.rs:159` | one-shot, `ReplaceDocument` | `PendingBoard` |
| 25 | Keycap size | `objects/keycap_size_controller.rs:272` | field edit, `ReplaceDocument` | `PendingResize` |

## Background you need

- #10 and #11 follow a landed edit with `Event::SelectParts`. Keep that: on
  Landed, select from the accepted document at the landing, and select nothing
  if the parts are no longer there.
- `layout_align_geometry.rs`'s `pending_settlement_gate` is shared Std-landed logic;
  delete it once nothing uses it.
- The `part_placement` tests use a fake runtime (inventory section 6). Move the
  tests that assert landing onto ticket 01's adapter; keep pure geometry tests
  native.

## Acceptance criteria

- [ ] Each action submits an intent; all listed `Pending*` structs and heuristics are gone.
- [ ] One-shot controls are disabled while their ticket is pending; a double click creates one board, one pair, one placement.
- [ ] Rapid test: keycap size, then an unrelated Inspector edit queued behind it; both survive.
- [ ] Align of parts deleted before execution retires with a reason.
- [ ] Placement and apply-to-key still select the placed parts after landing.

## Verification

```sh
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/layout --locked --lib
```

## Out of scope

- Matrix setup, placement, Inspector and transform (ticket 11).
