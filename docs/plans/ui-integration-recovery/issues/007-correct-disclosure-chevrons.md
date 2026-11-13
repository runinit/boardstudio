# 007: Correct disclosure chevrons

Status: declined
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A6/A7](../../../research/2026-11-06-dev-ui-integration-audit.md#a6a7--shared-disclosure-integration)

## What to build

Each affected Layout disclosure arrow reflects its own expanded state using the
pinned primitive's actual state contract. Preserve nested independence and the
newer controls. This is separate from the motion/content-lifetime work in 006.

## Acceptance criteria

- [ ] Account for the audit's seventeen affected Layout-related sites against
  the current source, including any changed by other recovery work. Correct all
  applicable bindings in Objects, matrix and component inspection, companion
  placement and the Layout workspace.
- [ ] Mounted tests observe the rendered arrow orientation in open and closed
  states. A changed selector string alone is not proof of the correction.
- [ ] Representative nested sections show that opening a parent or sibling does
  not give a child's arrow the wrong state. Exercise repeated toggles and normal
  keyboard activation.
- [ ] Demonstrate RED before the binding repair and GREEN afterward (A4), using
  production callers and their real disclosure controls.
- [ ] Do not globally replace state selectors used by unrelated primitives or
  upgrade the dependency. Keep content motion changes in 006.
- [ ] Update any affected state-contract comments and record the verified caller
  inventory and behavior (A8).

## Verification and completion

Run focused mounted Layout/shared-control caller tests that establish computed
arrow state and nested independence, plus applicable compile/style checks.
Record exact commands and source identity. A motion repair is not a prerequisite
for testing disclosure state. Keep the parent specification and audit unchanged.

## Comments

- 2026-11-06: Cancelled by the user. This ticket will not be actioned; its
  acceptance criteria are retained as historical context, not completed evidence.
- 2026-11-06: Published independently from 006 as accepted; one consistent
  binding correction does not need a separate ticket for every call site.
