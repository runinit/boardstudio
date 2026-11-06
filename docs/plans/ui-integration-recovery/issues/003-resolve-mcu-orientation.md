# 003: Resolve the diagnosed MCU orientation symptom

Status: ready-for-agent
Type: build
Blocked by: [002: Diagnose MCU rotation](002-diagnose-mcu-rotation.md)
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Controller contract](../../split-board-editing/issues/12-half-frame-parts-and-mirrored-controller.md), [Half-frame decision](../../../adr/0039-half-frame-parts-mirrored-controller.md)

## What to build

Resolve the MCU orientation symptom through the smallest correction established
by 002. Both halves must follow the accepted controller contract through display,
save/reload and any affected output. The actual diagnosed layer determines the
change; this ticket does not prescribe a Core, renderer or saved-document patch.

## Acceptance criteria

- [ ] Before claiming implementation, 002 has established a real reproduction,
  demonstrated cause and bounded correction. Re-scope before coding if that
  evidence requires independent repairs or a new product decision.
- [ ] Reproduce the failure at the real regression seam before the correction,
  then demonstrate GREEN and rerun the original, unminimized user scenario (A4).
  Do not force a production edit if the demonstrated cause needs a different
  correction, such as loading the intended artifact.
- [ ] Both halves' stored, evaluated and rendered orientations satisfy the
  accepted contract in their respective coordinate frames. Preserve mirroring,
  local exceptions and existing independent-part behavior outside the cause.
- [ ] Save and reopen the affected design and verify both halves' orientation.
  Check relevant exported geometry when the diagnosis establishes output impact;
  record why a particular output check is or is not applicable.
- [ ] Use real Runtime/Session/Core workflows for applicable settlement or domain
  assertions and mounted/packaged browser coverage for the affected presentation
  path. Passing the heading helper's existing unit tests alone cannot close A1.
- [ ] Reconcile the generic heading guidance with the later controller exception
  and the verified result (A8), preserving historical evidence. Do not silently
  restore the generic independent-part half turn for controllers.
- [ ] Remove temporary diagnostic instrumentation and record the cause,
  correction, exact verification commands, source identity and remaining limits.

## Verification and completion

Run the preserved 002 reproduction, focused regressions at the diagnosed seam,
the original scenario, and the applicable save/reload and export checks. Record
each result separately. Full reconciled-candidate acceptance remains with 010
and 011. Keep the parent specification and original audit unchanged.

## Comments

- 2026-11-06: Published adjacent to diagnosis as requested. The correction is
  deliberately not selected before 002 supplies the evidence.
