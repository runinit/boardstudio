# 009: How to close rust-ui's control and accessibility gaps

Status: claimed
Type: grilling
Blocked by: None
Parent: [Map](../map.md)
Related: [002](002-rust-ui-catalogue-facts.md), [ADR-0034](../../../adr/0034-adopt-rust-ui-components-with-two-themes.md)

## Question

rust-ui's overlays, Select, Tabs, Tooltip and Switch are script-driven and uncontrolled,
with thin keyboard and ARIA support, while the workbench needs controlled open state
(Layout command menus, layer dismissal), keyboard reach and accessibility. Which route:
take rust-ui markup and styling over the `dioxus-primitives` behaviour we already ship;
rewrite the copied rust-ui sources to be controlled and accessible; contribute fixes
upstream first; or a mix decided per component? Does the answer change ADR-0034's
"rust-ui replaces Dioxus Components"?
