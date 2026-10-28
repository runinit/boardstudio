# 004: Vendored Lucide icons

Status: ready-for-agent
Type: build
Blocked by: [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [icon decision](../../rust-ui-migration/issues/008-choose-icon-set.md), [shared contract](../map.md#shared-contract)

## What to build

Icons come from vendored Lucide, one small component per icon, in place of the `icons` crate. Copied rust-ui components import them with only their `use` line changed, and the app's own hand-drawn icons can be replaced by the nearest Lucide icon.

## Acceptance criteria

- [ ] Every icon the copied components import exists as a component taking a class, decorative by default (`aria-hidden`), sized by class (16px default) and reading its stroke width from `--icon-stroke`
- [ ] A Lucide ISC notice is added to the licence file; no `icons` crate dependency exists
- [ ] The accessible name of a button containing an icon contains no icon text
- [ ] A short list of the app's hand-drawn icons with no Lucide equivalent (transform, keycap, geometry glyphs) is recorded in this ticket's outcome and stays custom through the same wrapper
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
