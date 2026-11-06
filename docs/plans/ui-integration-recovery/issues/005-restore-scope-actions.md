# 005: Restore nonduplicated scope actions

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A3/A4](../../../research/2026-11-06-dev-ui-integration-audit.md#a3a4--duplicate-actions-explicitly-expected-by-the-new-test)

## What to build

Matrix, Row and Column scope choices expose their intended contextual actions
once, without a competing popup when inline tools are available. Keep useful
empty-selection guidance and recover its transition into a selected editing
context, using the current shared controls.

## Acceptance criteria

- [ ] For Matrix, Row and Column, choosing or reselecting scope preserves its
  selected state and intended action availability without opening a competing
  action panel when the toolbar already contains the tools.
- [ ] With no applicable selection, the scope control still offers useful
  selection guidance. Selecting an object while that hint is open closes the
  panel as inline actions become available.
- [ ] Each intended action appears once across the competing toolbar/popover
  surfaces, including scope changes and reselection. Hidden duplicate labels
  alone are not sufficient if duplicate interactive controls remain.
- [ ] Keyboard scope navigation, action access, hint dismissal and focus remain
  correct. Preserved canvas handles and Command palette actions are not counted
  as unwanted duplicates or removed.
- [ ] Restore and strengthen the original assertions for all three scopes, with
  RED/GREEN evidence (A4). Combine focused toolbar fixtures with production
  Layout checks for real selection and focus transitions.
- [ ] Update relevant design and recovery records alongside the behavior (A8).
  Preserve the shared control migration and avoid changes to transform semantics
  or unrelated command surfaces.

## Verification and completion

Run focused mounted toolbar and production Layout selection/focus tests, recording
exact failing and passing invocations and source identity. Run applicable
compile/style checks for the touched modules. Matrix, Row and Column remain one
slice because they share the scope-control implementation. Keep the parent
specification and original audit unchanged.

## Comments

- 2026-11-06: Published from the accepted breakdown; independent of MCU diagnosis
  and the other UI behaviors, with shared-file changes coordinated as necessary.
