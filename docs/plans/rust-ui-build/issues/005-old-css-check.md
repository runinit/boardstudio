# 005: Old-CSS check

Status: ready-for-agent
Type: build
Blocked by: [001: Walking skeleton: one copied Button, both Themes, one migrated control](001-walking-skeleton.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [branch plan](../../rust-ui-migration/branch-plan.md) section 4, [shared contract](../map.md#shared-contract)

## What to build

A repository check reports how many `.board-*`, `.m1-*` selectors and `--wb-*` variables remain, so progress is visible during the migration and the final merge can be blocked on zero.

## Acceptance criteria

- [ ] `scripts/check.py` has a step that counts remaining `.board-*`, `.m1-*` and `--wb-*` uses in the web sources and styles; the baseline today is 622 selectors and 80 variables
- [ ] The default run reports the counts without failing; a strict mode fails while any remain, and the README lists both
- [ ] The step is covered by the repository check-script tests that already exist for `check.py`
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
