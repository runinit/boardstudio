# 025: Rewrite DESIGN.md for the new structure

Status: ready-for-agent
Type: build
Blocked by: [024: Delete the old layer](024-delete-the-old-layer.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [structure decision](../../rust-ui-migration/issues/011-target-workbench-structure.md), [theme contract](../../rust-ui-migration/theme-contract.md), [shared contract](../map.md#shared-contract)

## What to build

DESIGN.md describes the shipped workbench: the header, docks, cards style, narrow-screen behaviour, controls and Themes, in place of the old Layout and Components sections.

## Acceptance criteria

- [ ] DESIGN.md's Layout, Components and token sections match the shipped shell and the theme contract, with no contradiction about where Workspace navigation lives
- [ ] README commands and the glossary are checked and updated if affected; the documentation link check passes
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
