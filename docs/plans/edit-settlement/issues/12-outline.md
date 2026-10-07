# 12: Outline actions land through resolution

Status: resolved
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
- [ ] `outline_lifecycle_browser_tests.rs` runs against the real Session and Core
  (accepted-result assertions, no interception); whichever of tickets 11 and 12 resolves
  second deletes the layout-inspector interception from Runtime (deferred from ticket 05;
  see its Comments). If ticket 11 is still open, leave the interception and say so in
  the Outcome.

## Verification

```sh
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/layout --locked --lib
```

## Out of scope

- One outline module or typed outline edits (architecture review candidates 9 and 2; decision ticket 19).
- Perimeter preview staleness (`docs/backlog.md`).

## Outcome

Commits: claim; "Delete the layout-inspector interception from Runtime" (shared
`runtime.rs`, its own commit); "Land outline actions through resolution".

- `submit_action` keeps admission (workspace, generation, selection, `is_current`, the gap
  camera) and then plans the action with the new pure `plan_action(snapshot, action, seed)`,
  once at dispatch (an unsupported action is ignored early, as before) and again as the
  resolver (`action_resolver`) against the accepted snapshot at execution. Commit actions begin
  an `EditTicket`; perimeter previews stay on `Event::Edit` with the preview phase.
- `plan_action` returns the operation and target ids, retiring with a reason when the board,
  version, feature or perimeter is gone or changed ("The outline feature no longer
  exists.", …) and `Unchanged` for equal values. New identities (copied versions, added
  features, connections) are chosen inside it against the accepted document from a seed
  captured at submit; an added feature whose minted id is already taken gets a fresh one, so
  two queued adds cannot collide. `SetFeature` no longer compares the accepted feature with
  the panel's `before` snapshot: the committed `after` wins unless the feature is gone.
- Deleted: `Pending`'s kind/outcome/snapshot, `PendingKind`, the settle effect's per-action
  content checks and revision/token/durability gates, `OutlineExpectation`,
  `expectation_applied`, the "saved outline no longer matches this action" message and the
  untracked wait task. The settle effect now maps `Settlement` to the existing feedback
  states and supports several in-flight edits.
- One-shot controls (activate, copy, delete, create automatic, add/subtract/connect drawing,
  remove feature) disable on `OutlineInspectorProjection::one_shot_pending`; settings
  fields, feature values and perimeter edits are field edits and the panel stays editable
  while an earlier edit is applying or saving.
- The layout-inspector interception (`set_layout_component_inspector_test_state`, the event
  capture and its read-model override) is gone from Runtime; ticket 11 had already moved
  the transform Inspector suite, and `outline_lifecycle_browser_tests.rs` now opens its
  fixtures through the real Session and Core and asserts accepted results (revisions,
  accepted features, connection identities, the fixed copy's points).
- New tests (real Session/Core, gated reply): two `SetFeature` edits on different features
  both survive and Undo removes them in order; two queued `AddFeature`s with the same minted
  id both exist with distinct ids; `SetFeature` on a feature removed first retires with a
  reason; a one-shot ticket stays pending until it settles.

Checks: `cargo test -p boardstudio-web-layout --locked` 50 passed; `cargo test -p
boardstudio-web-runtime --locked` 103 passed; `python3 scripts/check.py typecheck`;
`python3 scripts/check-wasm-tests.py`; `python3 scripts/check-doc-links.py`;
`wasm-pack test --headless --chrome web/crates/layout --locked --lib` — 92 passed;
`run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs` — 20
executed, 0 failed. `check.py test` still fails only on the missing `step-oracle/`.

Follow-ups: `outline_lifecycle_tests.rs` kept its native stub-runtime pure-geometry tests;
the board name action (ticket 16) is untouched. `Unchanged` from `apply_outline_edit`
also covers edits whose target is missing (no resolver retire reason there).
