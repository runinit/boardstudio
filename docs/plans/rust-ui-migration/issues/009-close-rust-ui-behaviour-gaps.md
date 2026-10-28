# 009: How to close rust-ui's control and accessibility gaps

Status: resolved
Type: grilling
Blocked by: None
Parent: [Map](../map.md)
Related: [002](002-rust-ui-catalogue-facts.md), [ADR-0039](../../../adr/0039-adopt-rust-ui-components-with-two-themes.md)

## Question

rust-ui's overlays, Select, Tabs, Tooltip and Switch are script-driven and uncontrolled,
with thin keyboard and ARIA support, while the workbench needs controlled open state
(Layout command menus, layer dismissal), keyboard reach and accessibility. Which route:
take rust-ui markup and styling over the `dioxus-primitives` behaviour we already ship;
rewrite the copied rust-ui sources to be controlled and accessible; contribute fixes
upstream first; or a mix decided per component? Does the answer change ADR-0039's
"rust-ui replaces Dioxus Components"?

## Comments

**2026-10-21, findings (source read at upstream `7120636e` and `dioxus-primitives` `9a75825`; nothing run in a browser):**

- **rust-ui behaviour is thin.** Of 86 components, about 60 are markup and classes only. Tabs,
  Collapsible, Switch, Radio, Toggle Group, Stepper and Pagination keep state in Rust signals.
  Eleven overlay and menu components (Popover, Dialog, Sheet, Drawer, Select, Dropdown Menu,
  Context Menu, Menubar, Navigation Menu, Multi-select, Command) drive themselves with inline
  `<script>` blocks, about 1,050 lines in total, flipping `data-state` on the page outside
  Dioxus. Sizes: Popover 20, Drawer 29, Sheet 29, Select 49, Dialog 54, Dropdown Menu 92,
  Command 305. Its own Rust hooks (`use_click_outside` and the scroll-lock hooks) are used by
  none of them.
- **Script hazards for a long-lived editor.** Each instance adds `document` listeners that are
  never removed; setup polls with `setTimeout(…, 50)` until elements exist; Escape closes every
  open popover at once and Dialog calls `preventDefault`, which competes with the app's own
  Escape flows. Positioning uses CSS anchor positioning (`position-anchor`, `@position-try`),
  whose support in our target browsers is unchecked.
- **Accessibility is partial, with no tests or claims.** Dialog: no `role=dialog`, no focus trap
  or return. Popover: `role=dialog`, Escape and outside click only. Select: `listbox`/`option`,
  but options are `li` with `onclick` only, no arrow keys, no `aria-expanded`. Dropdown Menu:
  `menuitemradio`, Escape only. Tabs: no roles or arrow keys. Tooltip, Slider, Sheet, Drawer:
  no ARIA. Switch, Checkbox and Toggle Group have roles. Command (19 ARIA hits, 9 key handlers)
  and Data Grid (36, 5) are the exceptions. Its e2e suite has no accessibility tests.
- **`dioxus-primitives` is Rust-driven**, with one bundled 1.4 KB `focus-trap.js` asset instead
  of per-instance scripts, controlled `open`/`value` props with change callbacks, and roles plus
  arrow-key handling in Select, Tabs, Dropdown Menu. It has no Sheet, Drawer, Command or
  Navigation Menu. Counts here are source greps, not conformance results.
- **Removing the scripts is feasible**: each is show/hide, outside click, Escape, positioning
  and scroll lock, replaceable by an open signal plus one shared owned hook.

## Answer

Decided with the user, 2026-10-21, on the findings above:

- **rust-ui supplies the look; `dioxus-primitives` supplies the behaviour** for every control it
  covers. Where primitives has no counterpart (Sheet, Drawer, Command, Navigation Menu) we write
  the behaviour ourselves in Rust, on Dialog where possible. `Board*` wrappers still go; code
  calls the copied components directly.
- **No inline scripts and no ownerless document listeners in copied code.** Outside click,
  Escape and scroll lock live in one shared hook that removes its listeners on unmount.
- **Measure, don't assume:** [010](010-measure-keyboard-and-aria-conformance.md) checks the
  controls we'd adopt and sets the accessibility floor.
- **Scope grows to restructuring.** Beyond component choices, the app's structure (workspaces,
  panel layout, navigation) is in scope: [011](011-target-workbench-structure.md).
- Fixes stay in our copy; contributing upstream is optional and outside this effort.
- ADR-0039 revised to match.
