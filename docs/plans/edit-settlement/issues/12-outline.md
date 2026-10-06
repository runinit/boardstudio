# 12: Outline actions land through resolution

Status: ready-for-agent
Type: build
Blocked by: 06
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Every outline action goes through one helper, `submit_action`, and one `Pending`
record whose settle effect combines Std-landed with a per-action content check.
Replace that with one resolver per action and the edit ticket. Six of the nine
actions send a whole document or outline built from the snapshot the panel last saw;
their clone-and-mutate step moves into the resolver.

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
line numbers at `3368825`, orientation only). All go through
`outline_lifecycle.rs:2071` (helper `submit_action` `:1339`, guard `:1346`,
`Pending` `:325`, settle effect `:795`).

| # | Action | Payload | Kind |
|---|---|---|---|
| 27 | Activate | field-scoped | one-shot |
| 28 | Copy | field-scoped | one-shot |
| 29 | Delete | field-scoped | one-shot |
| 30 | AddFeature | whole document/outline | one-shot |
| 31 | AddConnection | whole document/outline | one-shot |
| 32 | SetFeature | whole document/outline | field edit |
| 33 | RemoveFeature | whole document/outline | one-shot |
| 34 | Settings Update (`outline_settings.rs:118`) | whole document/outline | field edit |
| 35 | EditPerimeter commit | whole document/outline | field edit |

Operations used across these: `SelectOutline`, `CopyOutline`, `RemoveOutline`,
`ReplaceDocument`, `SetOutline`, `RenameOutline`.

## Background you need

- EditPerimeter's previews (`outline_lifecycle.rs:~2044`) are untracked and stay on
  `Event::Edit` preview. Only its commit becomes an intent.
- Retire when the outline, feature or connection is gone. New IDs (AddFeature,
  AddConnection, Copy) must be chosen inside the resolver against the accepted
  document, so a queued add can't collide with one that landed first.
- Tests: `outline_lifecycle_browser_tests` capture submitted events and
  `outline_lifecycle_tests` use a stub runtime (inventory section 6). Move landing
  assertions to ticket 01's adapter; keep pure outline-geometry tests native.
- Board name (#26) also appears in the outline browser tests; it migrates in
  ticket 16. Leave its assertions working.

## Acceptance criteria

- [ ] Rapid test: two SetFeature edits on different features committed back-to-back both survive; Undo removes them in order.
- [ ] Rapid test: AddFeature twice queued; both features exist with distinct IDs.
- [ ] SetFeature on a feature deleted before execution retires with a reason.
- [ ] `Pending`, its settle effect and per-action content checks are gone; `submit_action` submits intents.
- [ ] One-shot controls are disabled while pending; settings fields are not.

## Verification

```sh
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/layout --locked --lib
```

## Out of scope

- One outline module or typed outline edits (architecture review candidates 9 and 2; decision ticket 19).
- Perimeter preview staleness (`docs/backlog.md`).
