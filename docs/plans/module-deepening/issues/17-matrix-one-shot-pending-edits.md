# 17: Matrix one-shot actions use PendingEdits

Status: resolved
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
- [x] Mounted action tests use the real WASM Runtime; migrate hand-resolved domain
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

Merged `deepening/17-matrix-one-shot-pending-edits` through
`1d45506593da76ad65d6fd6874f74aa1c117059c`, based on local `dev` at `efa60e0ae`.
Matrix preset, delete and unlink observations now live in
`PendingEdits<MatrixActionKey>`. Key equality follows the action kind, while preset and
delete keys retain request metadata for feedback and exact selection follow-ups.
Unlink uses a plain key. Their submission holders and three separate settlement
functions are removed. The remaining Matrix field holder, feedback and field loop
belong to [Matrix fields complete the PendingEdits tracer](18-matrix-field-pending-edits.md).

Actions disable while pending. Preset failures keep inline placement; delete and
unlink failures keep Runtime report placement. Retirement is silent. A landed delete
clears only the captured Matrix selection; a newer Outline selection survives. The
preset no longer emits `Saved` feedback.

The panel's observation owner uses its existing mount/editor lifetime, exact selected
context generation, selection scope generation and Layout workspace. Context identity
tracks Scope, TreeContext, workspace and selection generation; accepted revision and
snapshot token are excluded. Actual owner departure retires the observation permanently;
same-owner accepted revisions leave it live. EditTicket owns Runtime Scope liveness,
so the panel adds no duplicate Runtime Scope settlement check. Retiring observation
does not cancel the Session operation.

The duplicate-variant preset waiter uses a local keyed collection for its sequential
workflow. Silent retirement exits without the genuine-failure recovery/report path.
Scope/owner departure during catalogue preparation also retires silently. Genuine
catalogue, Core, save and duplication failures retain guarded recovery/reporting;
cleanup still proves ownership of the clone. This workflow exception remains distinct
from ordinary panel settlement and must survive shared-helper binding.

Mounted tests click the real Matrix Inspector Delete button, hold and release Core,
and verify pending disabling, authoritative landing and exact selection behavior.
A new failure regression first reproduced `Could not delete matrix` after selection
departure. It now passes alongside the paired same-owner test: both preserve the
real failed save, accepted Matrix and Session `RecoveryRequired`, while only the
departed panel suppresses its report. Real Runtime tests also cover variant preparation
Scope retirement and genuine same-Scope lifecycle failure.

Fresh final verification: native Layout **51 passed, 1 ignored**; Layout browser group
excluding the setup guide **114 passed**, then setup guide **1 passed**, under the
exclusive Chrome lease. `python3 scripts/check.py lint typecheck` and the Layout WASM
test-target compile passed. The compile has one existing unused-import warning in
`part_placement.rs`. Standards and Spec reviews, including the final owner-lifetime
delta pinned to the commit above, have no blocking findings.

The earlier `check.py test` run passed workspace tests before the unchanged CAD
baseline failed: 49 passed, 1 failed, 4 ignored, with
`core_internal_gasket_fixtures_export_connected_positive_regions` rotated-concave/bottom
expected 80481.2399 and actual 80579.55733514718. That full native gate and the full
repository browser gate were not rerun for the last owner-lifetime patch; the affected
native and mounted Layout checks above were rerun. Combined tracer verification remains
with the final Matrix field slice and its parent gate.
