# 010: Measure keyboard and ARIA conformance of the controls we would adopt

Status: claimed
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
