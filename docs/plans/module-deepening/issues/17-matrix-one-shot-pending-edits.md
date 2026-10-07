# 17: Matrix one-shot actions use PendingEdits

Status: claimed
Type: build
Blocked by: 15
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [PendingEdits and Matrix tracer gate](05-pending-edits-module.md) · Decision: [Pending-edit settlement answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## What to build

Migrate Matrix preset, deletion and unlink observations onto the keyed Runtime module.
Own the action regions and their tests in
`web/crates/layout/src/objects/matrix_inspector_controller.rs`, with necessary action
presentation changes in `objects/matrix_inspector.rs`. Preserve catalogue preparation,
resolver behavior, one-shot admission and exact post-landing selection.

Run in parallel with [Shared UI helpers](16-pending-edit-ui-helpers.md); this slice
consumes Runtime directly, so it does not depend on unfinished Signal helpers.
It does not edit `ui-shared` or migrate field-edit state. The final Matrix integration
will bind the shared helpers and remove the remaining field feedback layer.

## Acceptance criteria

- [x] MatrixPresetSubmission and MatrixDeletionSubmission are removed; unlink's
  standalone ticket is tracked through PendingEdits with no fabricated panel owner.
- [x] The preset/delete/unlink settle_pending functions are removed and actions
  disable while pending. Landed produces the same precise selection follow-up.
- [x] Action failures use the existing panel/report placement; retired actions are
  silent, including owner departure and Scope changes during preparation or saving.
- [x] Audit the duplicate-design-variant preset waiter as another one-shot observation:
  use the collection where applicable and document any workflow-specific waiter.
  Retired ends observation silently; it must not become an error that triggers
  failure recovery or a user-visible report. Dropping observation does not cancel
  authoritative Session work. Genuine catalogue/Core/save/duplication failures keep
  their existing guarded recovery/reporting. Any lifecycle cleanup must still prove
  ownership of the clone; never redirect or restore over another project or owner.
- [x] Existing field behavior remains usable until the final field integration;
  the remaining MatrixSubmission/field feedback is explicitly deferred to that slice.
- [ ] Mounted action tests use the real WASM Runtime; migrate hand-resolved domain
  tests to the registered native Runtime seam where feasible and remove only shallow
  tests replaced by interface coverage. Never claim dormant tests as executed.

## Verification

```sh
cargo test -p boardstudio-web-layout --locked
python3 scripts/check.py lint typecheck test browser
```

Run the Matrix action/variant tests and affected Layout mounted suites. Serialize
Chrome through the orchestrator. Review the branch on both axes before merging; the
field integration starts only after this action branch is merged.

## Outcome

Matrix preset, delete and unlink observations now live in `PendingEdits<MatrixActionKey>`.
The keys preserve preset request feedback and exact deletion selection follow-up; unlink
uses a plain action key and its ticket's captured Scope. The preset/delete/unlink
submission-holder structs and their separate settlement functions are removed.

Landed clears one-shot pending state. Preset failures stay in the existing inline
feedback area; delete and unlink failures keep their existing Runtime report placement.
Retired results are silent. Scope/owner changes during preset preparation clear the
pending feedback, and the duplicate-variant preset waiter now treats retirement as a
silent result instead of invoking recovery/reporting. Genuine catalogue, Core, save,
and duplication failures retain their guarded recovery/reporting. A landed delete clears
selection only if the same captured matrix remains selected. The preset no longer emits
a `Saved` status.

The browser-gated one-shot test now uses `PendingEdits` with a real Runtime/Core gate;
the existing edit field submission/feedback layer is left for ticket 18. Duplicate-variant
preparation maps a departed Scope, missing matrix, or closed owner directly to silent
retirement, before the generic genuine-failure recovery path. A WASM Runtime test covers
Scope departure during this preparation window.

Verification: `cargo test -p boardstudio-web-layout --locked` passed 51 tests with one
ignored; `python3 scripts/check.py lint typecheck` passed; and
`cargo check -p boardstudio-web-layout --tests --locked --target wasm32-unknown-unknown`
passed with one existing unused-import warning in `part_placement.rs`. The workspace test
step passed before the CAD step reproduced the known baseline failure: 49 passed, 1
failed, 4 ignored in `core_internal_gasket_fixtures_export_connected_positive_regions`
(rotated-concave/bottom expected 80481.2399, actual 80579.55733514718). Browser tests and
final pinned Standards/Spec reviews remain pending.
