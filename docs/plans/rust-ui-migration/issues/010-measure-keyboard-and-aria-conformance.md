# 010: Measure keyboard and ARIA conformance of the controls we would adopt

Status: resolved
Type: task
Blocked by: None
Parent: [Map](../map.md)
Related: [009](009-close-rust-ui-behaviour-gaps.md), [001](001-compile-rust-ui-against-our-build.md)

## Question

In the scratch crate, build the `dioxus-primitives` controls we would adopt with rust-ui classes
(Popover, Dialog, Select, Dropdown Menu, Tabs, Tooltip, Switch, Checkbox, Toggle Group, Slider)
and run a browser keyboard script plus an axe-style ARIA check on each. Record per control which
WAI-ARIA Authoring Practices behaviours pass (roles, arrow keys, Home/End, Enter/Space, Escape,
focus trap and return) and which fail. The result is the accessibility floor for the mapping.
AFK; nothing tracked changes.

## Answer

Measured in a scratch lab (upstream demos of 15 controls built with `dioxus =0.7.10` and `dioxus-primitives` rev `9a758255`,
real key presses in headless Chromium, axe-core 4.14.0 on the closed page and with each overlay open). Full results, limits and the
floor are in [accessibility-floor.md](../accessibility-floor.md).

- **Pass as shipped:** Combobox, Popover, Dialog, Alert Dialog, Tabs, Checkbox, Tooltip, Accordion, Collapsible.
- **Need fixes in our copy before adoption:**
  - Select: focus is lost to `<body>` after Escape or Enter; the open listbox has no accessible name.
  - Dropdown Menu: uses `listbox` / `option` roles instead of `menu` / `menuitem`, loses focus on close, has no typeahead.
  - Toolbar: no single tab stop (all six buttons are tab stops) and End does not reach the last item.
  - Slider: Home and End do nothing.
  - Toggle Group: no group role or label.
  - Toast: `alertdialog` wrapper for a transient notice.
- The existing `BoardPopover` already restores focus itself, so the old wrappers were hiding this class of gap.
- Consequence: the Project, Settings, Transform and Layout command menus cannot move to the primitives' Dropdown Menu until it is fixed;
  the Foundation lane carries those fixes, and the tests in the floor gate every lane.
