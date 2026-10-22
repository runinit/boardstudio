# 004: Map each building block and control pattern to one rust-ui component

Status: resolved
Type: grilling
Blocked by: [003](003-inventory-existing-ui.md), [011](011-target-workbench-structure.md)
Related: [002](002-rust-ui-catalogue-facts.md), [009](009-close-rust-ui-behaviour-gaps.md), [010](010-measure-keyboard-and-aria-conformance.md)
Parent: [Map](../map.md)

## Question

For every row of the inventory, which single rust-ui component (or composition of
components) replaces it, and which rows have no direct match? Also: which rust-ui
components we adopt that improve on today's selection (for example Toast, Command,
Tooltip, Kbd), and where they would apply.

## Answer

Decided with the user, 2026-10-21, in [mapping.md](../mapping.md): every `Board*` control, raw control type, shell
element and recurring pattern maps to one rust-ui component or a composition of them. No new custom components
are needed; the custom work is behaviour (dock resize, a non-modal Sheet, script-free Command, Sidenav and
Popover behaviour).

Rules the user set: sizes map by pixels (Comfortable 40px to `Lg`, Compact 36px to `Default`; naive name mapping
would shrink controls); `Select` for short fixed lists and `Combobox` for long or filterable ones; vertical
`Stepper` for the setup guide; inline messages split by the field / page / notice / transient rule; `Tooltip`
on icon-only buttons; file inputs stay `Button` plus native input. Gained components: Command, Toast, Tooltip,
Kbd, Badge, Alert / Callout, Empty, Item, Card, Separator, Sheet, Sidenav, Dropdown Menu, Spinner / Skeleton.
Not adopted: Context Menu, Menubar, Navigation Menu, Data Grid, Dropzone, marketing blocks.

The sizing question (does the Original theme tighten 40px / 36px?) is added to [005](005-theme-structure.md);
native-to-custom behaviour parity and the old-CSS removal order are added to [007](007-branch-and-lane-plan.md).
