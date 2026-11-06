# 004: Restore placement and Relationships

Status: wontfix
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Placement contract](../../split-board-editing/issues/15-placement-breadcrumb-retire-relations.md), [Audit A2/A4](../../../research/2026-11-06-dev-ui-integration-audit.md#a2a4--relations-contract-and-its-test-changed-together)

## What to build

Restore the approved placement workflow throughout the Layout Inspector using
the current shared controls: “How it is placed” above the fields, Relationships
beside them, and no Relations tab. Matrix and component inspection form one
coherent navigation contract and are recovered together.

## Acceptance criteria

- [ ] Breadcrumbs, carried relationships and every applicable mirror-partner,
  follows and anchor fact are available alongside editing without opening a
  Relations tab. Both matrix-context and component inspection obey the contract.
- [ ] Breadcrumb and reference navigation use normal selection rules without
  accepted-document or Undo-history writes. Hidden references explain their
  layer and remain navigable; removed targets have no dead navigation action.
- [ ] A populated split fixture establishes an observable focused half before
  breadcrumb and reference navigation. The same half remains focused afterward;
  missing labels or an absent split must fail the test, not compare equal.
- [ ] Hover and keyboard focus mark the intended reference targets in violet,
  distinct from amber edit previews and persistent anchor feedback. Transient
  feedback clears on leave, blur, selection change and target removal.
- [ ] Restore the approved behavioral assertions and show RED before the repair
  and GREEN afterward (A4). Use the real mounted Layout/Inspector and existing
  split-focus, relation-highlight and field-preview fixture patterns.
- [ ] Retain shared controls, field editing and draft ownership while retiring
  obsolete tab machinery made unused by this repair. Do not restore old styles
  or refactor unrelated Inspector behavior.
- [ ] Update the placement/design and historical ticket records with the recovered
  behavior and its actual verification scope (A8). Preserve earlier evidence as
  history; do not mark the full acceptance gate complete.

## Verification and completion

Run focused mounted production Layout relation, Inspector, split-focus and
field-preview coverage. Record exact RED/GREEN invocations and source identity,
and run the applicable compile/style checks for the touched modules. No MCU
diagnosis, disclosure-motion or scope-action change is a prerequisite. Keep the
parent specification and original audit unchanged.

## Comments

- 2026-11-06: Cancelled by the user. This ticket will not be actioned; its
  acceptance criteria are retained as historical context, not completed evidence.
- 2026-11-06: Published as one behavior slice because the Inspector views share
  tab state and navigation semantics; tests and documentation travel with it.
