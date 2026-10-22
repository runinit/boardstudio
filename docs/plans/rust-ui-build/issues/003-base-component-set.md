# 003: Base component set

Status: ready-for-agent
Type: build
Blocked by: [001: Walking skeleton: one copied Button, both Themes, one migrated control](001-walking-skeleton.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [mapping](../../rust-ui-migration/mapping.md), [inventory](../../rust-ui-migration/inventory.md), [spec](../../rust-ui-migration/spec.md) section 3, [shared contract](../map.md#shared-contract)

## What to build

The rest of the rust-ui components the mapping needs are available as copied source, ready for the accessibility and migration tickets: Input, Input Group, Field, Label, Select, Checkbox, Collapsible, Accordion, Toggle Group, Toolbar, Popover, Dropdown Menu, Dialog, Alert Dialog, Tabs, Tooltip, Slider, Combobox, Alert, Callout, Badge, Card, Separator, Kbd, Item, Empty, Spinner, Skeleton, Stepper, Toast and Table.

## Acceptance criteria

- [ ] Each component is copied with provenance (upstream path, commit, MIT notice) and compiles on the pinned Dioxus
- [ ] Built-in borders are removed in the copies, including `dark:`-prefixed classes (cards style, theme contract section 8); focus rings stay
- [ ] Copied code contains no inline `<script>`, no `document::eval` and no document listener without an owner; a repository check or test enforces this
- [ ] Interactive components are built on `dioxus-primitives` behaviour where a primitive exists, with rust-ui classes on top; where none exists (Command, Sheet, Drawer) the component is left to its own ticket
- [ ] Upstream icon imports point at the shared icon module name used by ticket 004 so no `icons` crate is needed
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches
