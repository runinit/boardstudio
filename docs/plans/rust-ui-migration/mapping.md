# Mapping: existing building blocks to rust-ui components

Ticket: [004](issues/004-map-building-blocks-to-rust-ui.md). Decided with the user, 2026-10-21 (every row accepted as drafted, with the sizing, Select/Combobox, Stepper, message, Tooltip and file-input rules below). Inputs: [inventory](inventory.md), [ADR-0039](../../adr/0039-adopt-rust-ui-components-with-two-themes.md),
[ADR-0040](../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md), the catalogue research on branch
`research/rust-ui-catalogue`, and ticket [009](issues/009-close-rust-ui-behaviour-gaps.md) (rust-ui supplies the look,
`dioxus-primitives` the behaviour, nothing ships inline scripts).

Reading the table. **Look** is the rust-ui component whose markup and classes we copy into
`web/crates/ui-shared/src/ui/`. **Behaviour** is who drives it: `native` (the browser element), `primitives`
(`dioxus-primitives`), `ours` (we write it in Rust), or `rust-ui` (rust-ui's own Rust state, no script).
"Composition" means built from several rust-ui components with no new primitive.

## 1. The `Board*` controls

| Existing (uses) | Look | Behaviour | Notes |
| --- | --- | --- | --- |
| `BoardButton` (189) and raw `button` (163) | `Button` | native | Primary→`Default`, Secondary→`Outline`, Quiet→`Ghost`, Danger→`Destructive`. Sizes: our Comfortable 40px = rust-ui `Lg`, Compact 36px = `Default`, so sizes map by pixels, not by name (decided); touch targets stay 44px. Icon-only → `Icon` / `IconSm`. |
| `BoardToolbar`, `BoardToolbarButton` (3, 3) | `Toolbar` | primitives Toolbar | roving arrow keys kept |
| `BoardToggle` (2) and `aria-pressed` buttons (17) | `Toggle Group` (`Outline` variant) | primitives Toggle / Toggle Group | rust-ui has no single Toggle, so a lone toggle is a one-item group |
| `BoardPopover` (2) and Layout command menus | `Dropdown Menu` for action lists, `Popover` for free-form content | primitives | the primitive's Dropdown Menu has `listbox` roles, loses focus on close and has no typeahead, so the Foundation lane fixes it first ([accessibility floor](accessibility-floor.md)); until then these menus keep the `Popover` approach |
| `BoardDialog` (3 uses): the library's project-delete confirmation | `Alert Dialog` | primitives Alert Dialog | the only modal use; the other two are flyout panels, see section 3 |
| `BoardCheckbox` (26) and raw checkbox (8) | `Checkbox` | primitives Checkbox | |
| `BoardNumberField` (20) and raw number input (52) | composition: `Field` + `FieldLabel` + `InputGroup` (`InputGroupInput` number, `InputGroupText` unit) + `FieldError` | native input | no rust-ui number field; parsing, blur commit and key handling stay in the caller. Kept as one thin composed `NumberField` in `ui/`, marked as a composition. |
| `BoardSelectField` (79) and raw `select` (26) | `Field` + `Select` | primitives Select | `Select` for short fixed lists (about 12 options or fewer), `Combobox` for long or filterable ones (part, key, model pickers); each file is classified in the sweep |
| `BoardDisclosure` (65), `details` (11) | `Collapsible` | primitives Collapsible | many sections may stay open together, so not `Accordion` |
| `BoardAccordionSection` (2) | `Accordion` | primitives Accordion | |
| `label` wrappers (167) | `Field` + `FieldLabel` / `Label` | native | |
| `ThemePicker` | header theme control, `Dropdown Menu` with radio items | primitives | designed in [005](issues/005-theme-structure.md) |
| `ObjectOptions` select | `Select` | primitives | |

## 2. Shell, panels and lists

| Existing | Look | Behaviour | Notes |
| --- | --- | --- | --- |
| `ObjectsPanel`, `InspectorPanel` | `Sidenav` (left, right) | ours (resize, widths) | ADR-0040 |
| `LayoutPanelSheet`, `CompactPanelControls`, `CompactPanelScrim` | `Sheet` (non-modal, no backdrop) with `Tabs` for Objects and Inspector | ours + primitives Tabs | ADR-0040; Sheet customised, as upstream's is modal |
| Header | `Button` (`Ghost` links), `Separator`, `Command` search trigger, `Dropdown Menu` | primitives, ours | ADR-0040 |
| Footer, zoom and history controls | `Toolbar` + `Button`; findings counts as `Badge` | primitives Toolbar | |
| Layout strips | `Toolbar` + `Toggle Group` + `Dropdown Menu` (Transform) | primitives | Stationary Controls Rule kept |
| Command palette (new) | `Command` | ours (Dialog and Combobox behaviour, no script) | [command palette](command-palette.md) |
| Toasts (new) | `Toast` | primitives Toast | upstream's depends on a script not in the components |
| Library landing, `KeyboardCard` | `Card`; no project → `Empty` | native | |
| `EmptyBoardCanvas` text | `Empty` | native | |
| Export rows, `ZmkFirmwareExportRow` | `Item` + `Badge` / `Status` + `Button` | native | |
| Findings and collision lists | `Item` + `Badge` (severity) | native | |
| Setup guide steps | `Stepper` (vertical, current and ready states) inside the four `Accordion` sections | rust-ui Stepper, primitives Accordion | sections stay freely navigable |
| Parts catalogue rows, relation rows | `Item` / `ItemGroup` | native | |
| Layer rows (`LayerToggleButton`, `LayerPanel`, `CanvasLayers`) | composition: `Collapsible` + `Item` + `Toggle` (eye) | primitives | no rust-ui layer control; composed, not custom |
| Bounded scroll regions | native `overflow`, Theme scrollbars | native | `Scroll Area` adds nothing here |
| Keyboard hints (new) | `Kbd` | native | |

## 3. Flyout panels that are not dialogs

`BoardDialog { modal: false }` is used for two flyouts, not dialogs: the Layout **Add** view (a create view inside the Objects
dock) and **Mirror existing half**. They become in-dock views built from `Collapsible` sections, `Item` rows and a search
`InputGroup` (Add), and a `Card` panel (Mirror existing half).

## 4. Cross-cutting patterns

| Pattern (count) | Look | Notes |
| --- | --- | --- |
| Inline status and alert text (246) | Rule: `FieldError` for field errors; `Alert` (error) for blocking or persistent page problems; `Callout` (info, warning) for notices and guidance; `Toast` for transient results; Export blockers and Case readiness stay inline. Each message is classified in the sweep. |
| Native `title` attributes (84, some are props) | `Tooltip` (+ `Kbd` where a shortcut exists) on icon-only buttons; redundant `title` dropped from text buttons | |
| File inputs (9) | `Button` + hidden native file input; no `Dropzone` (drag-and-drop import is a new capability, not requested) | |
| Colour inputs (4) | native colour input inside `Field` | rust-ui has none; primitives has a Color Picker if wanted later |
| Range inputs (4) | `Slider` | primitives Slider behaviour |
| Search inputs (5), text inputs, `textarea` | `InputGroup`, `Input`, `Textarea` | |
| Datalist (1) | `Combobox` | primitives |
| Inline SVG icons (84, 71 distinct drawings) | vendored Lucide, one component per icon, replacing hand-drawn ones where a match exists; domain glyphs stay custom ([008](issues/008-choose-icon-set.md)) | decorative by default; stroke from `--icon-stroke` |
| Headings `h2`/`h3` (104) | no component: Theme type scale | |
| Lists (17), table (1) | `Item` / `Table` | |
| Loading text | `Spinner`, `Skeleton` | |
| Tags and counts | `Badge` | |
| Section dividers | `Separator` | |

## 5. What stays custom (after looking for an equivalent)

No new primitive components are needed. Custom *behaviour*, not components: dock resize and per-side persisted widths; making
Sheet non-modal; rewriting Command and the Sidenav, Sheet and Popover behaviour without scripts; the precise numeric entry that
callers already own. Canvas and 3D surfaces are out of scope.

## 6. Components gained by this migration (not used today)

`Command`, `Toast`, `Tooltip`, `Kbd`, `Badge`, `Alert` / `Callout`, `Empty`, `Item`, `Card`, `Separator`, `Sheet`, `Sidenav`,
`Dropdown Menu`, `Spinner` / `Skeleton`. Not adopted: Context Menu, Menubar, Navigation Menu, Data Grid, Carousel, Marquee, the
marketing blocks, and the `date_picker`, `input_otp`, `input_phone` and `workflow` components.
