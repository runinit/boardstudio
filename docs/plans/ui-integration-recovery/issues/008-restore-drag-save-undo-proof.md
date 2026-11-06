# 008: Restore real drag/save/Undo proof

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A5](../../../research/2026-11-06-dev-ui-integration-audit.md#a5--what-the-existing-green-results-actually-cover), [Migration gate record](../../rust-ui-build/issues/026-merge-gate.md)

## What to build

Make the existing packaged-app drag harness exercise current controls and prove
real gesture preview/commit, durable save, cancellation and exact Undo. Restore
its existing behavioral coverage before adding the separate reload scenarios
owned by 009.

## Acceptance criteria

- [ ] Reproduce and diagnose the actual setup/focus and obsolete save-status
  observation failures against an identified current packaged build. Distinguish
  harness assumptions from product defects; do not guess an application race.
- [ ] The harness creates/selects the intended project and uses the current
  accessible controls through the real workflow. Required preconditions fail
  explicitly rather than accepting absent controls or skipping assertions.
- [ ] Existing Part, Column, Stagger, Ctrl+Offset and Matrix drag scenarios reach
  their trusted gestures, visible previews and intended accepted changes. Keep
  pointer/capture and changed-field observations meaningful.
- [ ] Durable document reads establish that committed edits are saved. Preserve
  meaningful save-state observations; a badge alone is not durable-save proof.
- [ ] Undo restores the exact prior document content and canvas scene and saves
  the Undo result as the appropriate new revision. Cancellation restores the
  scene without creating an accepted edit or saved revision.
- [ ] Restore the existing assertions with RED/GREEN evidence (A4). Keep unrelated
  collision/provider smoke scenarios and source/build/browser receipt checks;
  do not delete or bypass them to obtain a passing result.
- [ ] Record which drag scenarios completed and the complete runner's actual
  outcome. A later non-drag failure remains visible and cannot be reported as a
  full smoke pass or silently expanded into unrelated repair work.
- [ ] Update the drag evidence and historical acceptance records with current
  selector/workflow facts, tested source and remaining reload/full-gate limits
  (A8). Do not overwrite historical run results.

## Verification and completion

Use the existing supported Rust browser runner and checked packaged app. Record
actual commands, RED/GREEN evidence and source/build/browser identity. Run focused
runner controls when changing their behavior; preserve original semantic
assertions and receipt integrity. All existing drag/cancellation/Undo scenarios
must have terminal results. Full combined acceptance remains with 010.

## Comments

- 2026-11-06: The user accepted splitting the former drag/save/Undo/reload slice.
  This ticket restores the existing end-to-end proof; 009 adds a distinct reload
  boundary after this workflow is reliable. Parent spec and audit stay unchanged.
