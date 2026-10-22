# 026: Merge gate and single merge

Status: ready-for-agent
Type: build
Blocked by: [024: Delete the old layer](024-delete-the-old-layer.md), [025: Rewrite DESIGN.md for the new structure](025-rewrite-design-md.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [branch plan](../../rust-ui-migration/branch-plan.md) section 5, [shared contract](../map.md#shared-contract)

## What to build

The migration branch passes the gate and is merged to `dev` once.

## Acceptance criteria

- [ ] Full `scripts/check.py` and the browser tests pass; the accessibility checks pass; the old-CSS check passes in strict mode
- [ ] The user has reviewed every Workspace in both Themes and both Appearances at desktop (above 1150px), mid (981 to 1150px) and narrow (980px and below) widths, with screenshots
- [ ] One two-axis code review (Standards and Spec) of the whole branch is done and its findings resolved or recorded
- [ ] `dev` was merged into the branch before the merge; the single merge to `dev` is done and nothing is pushed without the user's say-so
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
