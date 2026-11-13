# 006: Restore shared disclosure motion

Status: declined
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A6/A7](../../../research/2026-11-06-dev-ui-integration-audit.md#a6a7--shared-disclosure-integration), [Shared control decision](../../../adr/0034-adopt-rust-ui-components-with-two-themes.md)

## What to build

Restore deliberate opening and closing through the shared disclosure control and
Snap. Reduced motion, draft retention where required and hidden-content focus
handling are part of the same behavior. Chevron-state repair is separately owned
by 007.

## Acceptance criteria

- [ ] Snap and representative shared-control callers demonstrate actual rendered
  opening and closing transitions. Transition declarations or class-string
  assertions alone do not establish visible motion.
- [ ] Reduced-motion preference is respected while expansion, collapse and
  control interaction remain functional.
- [ ] Content lifetime supports closing as well as opening. Draft-owning callers
  retain their drafts across collapse/reopen, without requiring every caller to
  adopt a new persistence policy.
- [ ] Closed content cannot remain an unintended keyboard or pointer target;
  focus remains usable through collapse/reopen. Test both retained content and
  callers whose children would otherwise unmount.
- [ ] Mounted shared-control and production Snap/Inspector tests demonstrate
  RED/GREEN for the restored behavior (A4). Exercise representative nested
  disclosures and both normal and reduced motion.
- [ ] Keep canvas interaction immediate, preserve the newer shared controls and
  limit caller changes to the required contract. Compare other Workspace motion
  claims with accepted design rather than assuming all previously animated.
- [ ] Correct the wrapper's misleading motion description and relevant design
  records with the actual behavior and evidence (A8).

## Verification and completion

Run focused mounted shared-control and Layout caller tests with rendered motion,
draft and focus observations, plus applicable compile/style checks. Record the
exact commands, RED/GREEN results and source. Do not close 007 based on motion
success or replace its arrow-state assertions. Keep the parent specification
and original audit unchanged.

## Comments

- 2026-11-06: Cancelled by the user. This ticket will not be actioned; its
  acceptance criteria are retained as historical context, not completed evidence.
- 2026-11-06: Published with motion, reduced motion and content/focus handling
  together because they define a complete disclosure transition.
- 2026-11-08: REOPENED by user during 011 human acceptance review (see 011
  D7-adjacent disclosure motion expectations). Original cancellation preserved
  above as history. Scoping to follow after 011 defect slices; do not treat the
  2026-11-06 cancellation rationale as current.
- 2026-11-10: Cancelled for now by the user ('skip animation for now, cancel tickets regarding this'). Earlier 2026-11-06 cancellation and 2026-11-08 reopening preserved above as history. Acceptance criteria remain unchecked historical context, not completed evidence; no implementation claimed.
