# 14: PCB wiring, modules and physical setup land through resolution

Status: claimed
Type: build
Blocked by: 06
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Move the PCB workspace's edits onto resolvers and the edit ticket. Most of them
clone the whole document and send `ReplaceDocument`, then decide they landed by
comparing the whole accepted document with what they sent. After this ticket each
resolver applies its change to the accepted document at execution, and the landing
replaces whole-document equality.

Electrical remap review (`pcb_wiring/remap.rs`) is **not** in this ticket. It keeps
its strict captured-revision check and its `PendingProtectedRemap`.

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

From [inventory.md](../inventory.md) (paths relative to `web/crates/pcb/src/`;
line numbers at `3368825`, orientation only):

| # | Action | Submit | Kind | Settlement to delete |
|---|---|---|---|---|
| 40 | Board reference actions | `board_reference_owner.rs:128` (asset stored first in `pcb_board_reference.rs:~278`) | one-shot, `ReplaceDocument` | result observed and discarded; `wait_for_reference_edit` |
| 41 | Wiring mode | `pcb_wiring/mode.rs:187` | field edit, `ReplaceDocument` | `PendingModeEdit`, whole-document equality |
| 42 | Pin lock | `pcb_wiring/pins.rs:204` | field edit, `ReplaceDocument` | `PendingPinEdit`, whole-document equality |
| 43 | Apply wiring plan | `pcb_wiring/apply.rs:174` | one-shot, `ReplaceDocument` | `PendingApply`, whole-document equality |
| 44 | Release reviewed connections | `pcb_wiring/apply.rs:218` | one-shot, `ReplaceDocument` | none |
| 45 | Firmware position | `pcb_wiring/controller.rs:238` | field edit, `SetKeyBinding` | `PendingFirmwarePositionEdit` → `settle_edit` in `web/crates/runtime/src/firmware_position_projection.rs:135` |
| 46 | Part net assign/create | `pcb_wiring/controller.rs:524` | field edit (create: one-shot), `ReplaceDocument` | `PendingPartNetEdit`, whole-document equality |
| 47 | Part scan mode | `pcb_wiring/part_input_settings/owner.rs:259` | field edit, `SetInputScanMode` | `PendingEdit` |
| 48 | Part generator parameter | same location | field edit, `ReplaceDocument` | `PendingEdit` |
| 49 | Physical setup | `pcb_physical_setup/controller.rs:329` | one-shot, `ReplaceDocument` | `SubmittedSetup`, `accepted_matches_proposal` (`web/crates/catalogue/src/physical_setup.rs:28`) |
| 50 | Module save placement | `pcb_module_inspector.rs:226` | field edit, `SetMountedModule` | outcome only |
| 51 | Module remove | `pcb_module_inspector.rs:300` | one-shot, `RemoveMountedModule` | outcome only |
| 52 | Embed module circuit | `pcb_module_inspector.rs:470` | one-shot, `EmbedModuleCircuit` | none |
| 53 | Remove embedded circuit | `pcb_module_inspector.rs:829` | one-shot, `RemoveEmbeddedCircuit` | none |

## Background you need

- #43 Apply wiring plan and #49 physical setup apply a proposal computed from an
  earlier document. The resolver re-applies the proposal to the accepted document.
  If the proposal no longer applies (its parts or nets are gone), retire with a
  reason; physical setup's "its proposal is no longer the current accepted project"
  becomes that retire reason, not a post-landing check.
- #40 and other uploads store asset bytes before submitting. If the edit is retired
  or fails, the bytes stay in the store unreferenced, as they do today when an edit
  fails. Accept that (map: asset cleanup is out of scope); note it in a code comment.
- `firmware_position_projection.rs` lives in the runtime crate and has native tests;
  remove its `settle_edit` once #45 uses the edit ticket, keeping projection logic.
- Tests: `pcb_wiring/mode_owner_tests` and `pcb_physical_setup/tests` run on a
  native stub that records events; PCB module save already runs on the real Session
  in the browser. Move landing assertions to ticket 01's adapter.

## Acceptance criteria

- [ ] Rapid test: wiring mode then pin lock committed back-to-back both survive; Undo removes them in order.
- [ ] Rapid test: part net assign queued behind an unrelated Layout edit keeps that edit.
- [ ] Applying a plan whose parts were deleted before execution retires with a reason.
- [ ] All listed `Pending*` structs, `SubmittedSetup`, whole-document equality checks and `settle_edit` are gone.
- [ ] Electrical remap review behaves exactly as before (its existing tests unchanged and green).
- [ ] One-shot controls are disabled while pending; field edits are not.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/pcb --locked --lib
```

## Out of scope

- Electrical remap review and PCB handoff/export commits (strict paths).
- Cleaning up unreferenced asset bytes.
- Typed wiring edits such as `SetWiringMode` (decision ticket 19).
