# Accessibility floor: measured keyboard and ARIA behaviour

Ticket: [010](issues/010-measure-keyboard-and-aria-conformance.md). Measured 2026-10-21. This is the floor that
[009](issues/009-close-rust-ui-behaviour-gaps.md) and the [branch plan](branch-plan.md) refer to: a control may be adopted only when it
passes the checks in section 3, after the fixes in section 2.

## 1. Method and limits

- **Lab.** A standalone scratch app (gitignored, `.scratch/a11y-lab`) containing the upstream `dioxus-components` demos for the
  controls in the [mapping](mapping.md), built with `dioxus =0.7.10` and `dioxus-primitives` at rev `9a758255` (the pins the app uses),
  release web build. The demos use upstream's own styles, not rust-ui's classes; behaviour and ARIA do not depend on classes.
- **Driver.** Headless Chromium through `agent-browser`: real key presses (`press`), then one line per step reporting the focused
  element and its ARIA state. Static ARIA scan with axe-core 4.14.0 (WCAG 2.0/2.1 A and AA, best practice) on the closed page and with
  each overlay open; colour contrast, page `lang` and page `h1` were excluded as properties of the lab page and demo theme, not of the controls.
- **Not measured.** Screen readers; other browsers; touch; the rust-ui look; combinations inside the real app; Select open by Space or
  ArrowDown; Toggle Group tab stops; Combobox with a disabled item. Every sequence ran once per control.
- axe checks static rules only: it did not catch the Dropdown Menu's wrong roles (found by reading the DOM).

## 2. Results

Legend: ✓ measured pass, ✗ measured failure, ⚠ works but questionable, – not measured.

| Control (primitive) | Passes | Fails and gaps |
| --- | --- | --- |
| **Select** | opens with Enter; ArrowUp / Down, Home, End move focus; typeahead (`b` reaches Banana); Enter selects and closes; Escape closes; roles `listbox` / `option`, `aria-haspopup`, `aria-expanded` | ✗ **focus is not returned to the trigger** after Escape or Enter (it falls to `<body>`; the next keys do nothing). ✗ open listbox has **no accessible name** (axe, serious) |
| **Combobox** | ArrowDown opens, `aria-activedescendant` follows, typing filters, Enter selects, Escape closes, focus stays on the input; axe clean open | – |
| **Dropdown Menu** | arrows, Home, End; skips disabled items; Escape closes | ✗ roles are `listbox` / `option` and `aria-haspopup=listbox`, **not `menu` / `menuitem`**. ✗ focus falls to `<body>` after Escape or Enter on an item. ✗ **no typeahead**. ⚠ Enter opens but focus stays on the trigger (first ArrowDown enters) |
| **Popover** | focus moves in, Tab stays inside, Escape closes and **returns focus to the trigger**; axe clean open | – |
| **Dialog** | `aria-modal`, `aria-labelledby` and `-describedby` resolve; focus moves in; Escape closes and returns focus; axe clean | – |
| **Alert Dialog** | `role=alertdialog`; focus starts on Cancel; Tab and Shift+Tab wrap inside; Escape returns focus; axe clean | – |
| **Tabs** | roving `tabindex`; arrows wrap; Home, End; Enter and Space activate; Tab enters the panel | – (manual activation, which APG allows) |
| **Toggle Group** | Space and Enter toggle `aria-pressed`; arrows move focus | ⚠ the group container has no role or label. – tab stops |
| **Toolbar** | `role=toolbar` with label; arrows cross the separator; Home | ✗ **End does not reach the last button**. ✗ **no single tab stop**: all six buttons have `tabindex=0`, so Tab walks all of them |
| **Checkbox** | Space toggles; Enter does not (correct); `role=checkbox`, `aria-checked` | – |
| **Slider** | Arrow keys change the value; `aria-valuemin/max/now`, orientation, label | ✗ **Home and End do nothing**. ⚠ PageUp / PageDown do nothing |
| **Tooltip** | appears on keyboard focus; `aria-describedby`; Escape dismisses | – |
| **Accordion** | arrows skip the disabled header and wrap; Home, End; Enter and Space; `aria-expanded`, `aria-controls` | – |
| **Collapsible** | Enter and Space; `aria-expanded`, `aria-controls` | – |
| **Toast** | content in `role=alert` inside a `role=region`; the close button is reachable by Tab | ⚠ the toast wrapper is `role=alertdialog` with `tabindex=0`; Escape does not dismiss |

**Pattern.** The overlays that existed first (Popover, Dialog, Alert Dialog) manage focus well. The listbox-based pickers (Select,
Dropdown Menu) lose focus on close. The app's own `BoardPopover` already contains focus-restoration code, which suggests the existing
wrappers patch exactly this kind of gap.

## 3. The floor

A control is adopted only if, in a browser test in the repo using the role-and-name helper, it passes every line for its kind.
"Fix" lines are the changes to make in our copy (decision of 009: fixes stay in our copy) before its lane may use it.

| Kind | Required behaviour | Fix before adoption |
| --- | --- | --- |
| Any overlay | Escape closes; focus returns to the opener; `aria-expanded` / `aria-haspopup` match the popup | **Select, Dropdown Menu:** return focus to the trigger on every close |
| Select | roles `listbox` / `option`; arrows, Home, End, typeahead; Enter selects; the open listbox has an accessible name | **name the listbox** from the Field label |
| Menu (actions) | roles `menu` / `menuitem` (checkbox, radio variants); focus enters on open; arrows, Home, End, typeahead; Enter activates and closes | **Dropdown Menu:** use `menu` roles, focus the first item on open, add typeahead. The Project, Settings, Transform and Layout command menus depend on this |
| Combobox | `role=combobox`, `aria-activedescendant`, filter on type, Enter selects, Escape closes | none found |
| Dialog / Alert Dialog | `aria-modal`, name and description, focus enters, Tab and Shift+Tab stay inside, Escape and focus return; Alert Dialog starts on the safe action | none found |
| Tabs | roving `tabindex`, arrows wrap, Home, End, Enter / Space activate | none found |
| Toggle Group | `aria-pressed`, arrows rove; group has a role and a label | **require** a group label |
| Toolbar | `role=toolbar` with label; arrows, Home, End; **one tab stop** | **single tab stop; End to the last item** |
| Slider | arrows, Home (min), End (max); value attributes | **Home and End** (and PageUp / PageDown) |
| Checkbox, Accordion, Collapsible, Tooltip | as measured above | none found |
| Toast (new) | announced as `role=alert` or `status`; dismiss button reachable | use `alert` / `status` for the item, not `alertdialog`; decide Escape |

**Consequence for the mapping.** The mapping sends action menus (Project, Settings, Transform, Layout command menus) to rust-ui
`Dropdown Menu` on the primitives' behaviour. As shipped, that primitive has the wrong roles, loses focus and has no typeahead, so
the Foundation lane must fix it as above, or those menus keep the `Popover`-based approach `BoardPopover` uses today. Either way the
test above gates the lane.
