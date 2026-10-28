# Spec: Migrate the workbench UI to rust-ui

Status: ready to hand off as build tickets. Assembled 2026-10-21 from the [map](map.md) and its 13 resolved tickets. Decisions can change
("ADRs can change, specs can change"); a ticket that finds one wrong revises the source document and notes why.

## 1. Goal

Rebuild BoardStudio's frontend controls and shell on [rust-ui](https://github.com/rust-ui/ui) components, with two Themes (Original and
shadcn) in light and dark, in one long-lived branch that merges to `dev` once. Every existing control maps to one rust-ui component or a
composition of them; the structure of the workbench follows rust-ui's own site, in a cards style.

**In scope:** shared controls, the plain HTML controls of the 210 production components, the header, docks, narrow-screen behaviour, menus,
Command palette, Toasts, Themes, icons, test rewrite, removal of the old CSS and the `Board*` wrappers.
**Out of scope:** canvas and 3D surfaces (`Scene`, `CaseViewer`, `KeycapsCanvas`, `PcbScene`, `KeymapCanvas`, `FootprintGraphics`, canvas
overlays); making the 3D renderer Theme-aware; drag-and-drop import (`Dropzone`); Context Menu, Menubar, Navigation Menu, Data Grid;
contributing fixes upstream to rust-ui; single-key tool shortcuts.

## 2. Decisions (the sources of truth)

| Topic | Where it is decided |
| --- | --- |
| Look from rust-ui, behaviour from `dioxus-primitives`, copied open-code, no inline scripts, Themes, `Board*` removed | [ADR-0039](../../adr/0039-adopt-rust-ui-components-with-two-themes.md), [009](issues/009-close-rust-ui-behaviour-gaps.md) |
| Shell: header, docks on Sidenav, non-modal Sheet on narrow screens, auto-hide dropped, Command palette, Toasts, cards | [ADR-0040](../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md), [011](issues/011-target-workbench-structure.md) |
| What each existing block becomes | [mapping](mapping.md), [inventory](inventory.md) |
| Tokens, Theme and Appearance attributes, preferences, cards style | [theme contract](theme-contract.md), [005](issues/005-theme-structure.md), [013](issues/013-prototype-theme-look.md) |
| Icons (vendored Lucide) | [008](issues/008-choose-icon-set.md) |
| Keyboard and ARIA floor per control | [accessibility floor](accessibility-floor.md), [010](issues/010-measure-keyboard-and-aria-conformance.md) |
| Command palette | [command palette](command-palette.md), [012](issues/012-command-palette-contents.md) |
| Branch, lanes, tests, merge gate | [branch plan](branch-plan.md), [007](issues/007-branch-and-lane-plan.md) |
| Vocabulary | [glossary](../../../GLOSSARY.md): Theme, Appearance, Workspace, Dock, Command palette |

## 3. Technical shape (what the builder needs in one place)

- **Source and provenance.** rust-ui components are copied to `web/crates/ui-shared/src/ui/<name>.rs`, one file per component, from
  `rust-ui/ui` at `7120636e68556ddd023a4ec8e083663c7b745318` (MIT; keep the notice and a provenance note with the commit). Upstream changes
  are pulled in by hand. New dependencies: `tw_merge` 0.1 and `strum` 0.28; no `icons` crate. Copies were proven to compile on
  `dioxus =0.7.10` in [001](issues/001-compile-rust-ui-against-our-build.md); `Button`'s `Link` becomes `a`.
- **Behaviour layer.** `dioxus-primitives` (rev `9a758255`, already pinned) drives every control it covers; the rust-ui class strings are
  applied on top. Sheet, Drawer, Command and Navigation Menu have no primitive: we write their behaviour in Rust. Copied code contains no
  inline `<script>` and no document listener without an owner (one shared hook, listeners removed on unmount).
- **Edits made to the copies** (not at call sites): remove built-in borders, including `dark:` rules (cards style); make Select, Dropdown
  Menu, Toolbar, Slider, Toggle Group and Toast pass the floor; replace upstream sizes with the shared size tokens (40 / 36 / 44px); make
  Sidenav's `collapsible: None` in-flow form usable (the shipped container is `fixed inset-y-0` and hidden below 768px).
- **Themes.** `data-appearance="light|dark"` and `data-theme="original|shadcn"` on `<html>`; one `@custom-variant dark` line;
  rust-ui token names plus the extras in the contract (`canvas`, `selected`, `field`, `toolbar`, geometry, grid, keycap, `--icon-stroke`);
  Original in Graphite values, shadcn from rust-ui's palette with Inter bundled (48 KB). A Theme never changes layout sizes. Preferences:
  `boardstudio:v2:appearance` (reads legacy `boardstudio:v2:theme` once) and `boardstudio:v2:workbench-theme`, default Original.
- **Icons.** Vendored Lucide, one component per icon, decorative by default, size from class, stroke from `--icon-stroke`; ISC notice in
  `web/assets/licenses/README.txt`.
- **Tests.** Controls are found by accessible role and name; behaviour assertions are unchanged except for decided changes; native
  test-name baselines are updated in the same commit as any rename.

## 4. Build tickets

**Published 2026-10-21 as 26 tickets in [rust-ui-build](../rust-ui-build/map.md)**, regrouped from the rows below into expand, migrate and
contract tickets with explicit blocking edges (the rows are the rationale; the tickets are the work). The plan lives in its own directory because
this one's ticket numbers belong to the decision tickets. Lanes follow the [branch plan](branch-plan.md); a ticket is done when its acceptance holds and its tests pass on the migration
branch. "Floor" means the control's rows in the [accessibility floor](accessibility-floor.md) section 3.

### Step 0 (user)
**B0** Merge the docs into `dev` and cut `feat/rust-ui-migration`. *Done 2026-10-21; neither is pushed.*

### Foundation lane (serial; all other lanes wait for it)

| Ticket | Scope | Acceptance |
| --- | --- | --- |
| **F1 Copy skeleton** | `ui-shared/src/ui/` with the base set: Button, Input, Input Group, Field, Label, Select, Checkbox, Collapsible, Accordion, Toggle Group, Toolbar, Popover, Dropdown Menu, Dialog, Alert Dialog, Tabs, Tooltip, Slider, Combobox, Alert, Callout, Badge, Card, Separator, Kbd, Item, Empty, Spinner, Skeleton, Stepper, Toast, Table; provenance note; `tw_merge` and `strum`; borders and scripts stripped; shared size tokens | wasm build passes; no `<script>` or `document::eval` in `ui/`; every file credits its upstream path and commit |
| **F2 Tokens and Themes** | the `@theme inline` map, Original and shadcn blocks (light and dark), `data-appearance` / `data-theme`, `@custom-variant dark`, Inter and licence entry, preference keys with the legacy read, the Appearance and Theme state in `ThemeState` | switching Theme and Appearance restyles the existing app without reload; legacy preference still applies; no flash regression versus today |
| **F3 Lucide icons** | per-icon components and wrapper, licence notice, `--icon-stroke`, the icons rust-ui's copied components import | decorative by default (`aria-hidden`); accessible names of buttons contain no icon text |
| **F4 Accessibility fixes** | the fixes in the floor for Select, Dropdown Menu, Toolbar, Slider, Toggle Group, Toast, each with browser tests | every control passes its floor rows in a repo browser test |
| **F5 Test helper and checks** | the role-and-name helper; the old-CSS check in `scripts/check.py` (fails while any `.board-*`, `.m1-*` or `--wb-*` remains) | helper used by a migrated test; check runs and is documented as expected to fail until Cleanup |
| **F6 Dock primitives** | non-modal Sheet; Sidenav in-flow form; per-side pointer and arrow-key resize with one saved width per side | resize by pointer and keyboard; widths persist per side; Sheet leaves the canvas interactive (no backdrop, no scroll lock) |

### Shell lane (starts after F1 to F6; first parallel lane)

| Ticket | Scope | Acceptance |
| --- | --- | --- |
| **S1 Header** | 56px borderless card per ADR-0040: logo, Project menu, Workspace links, search button, save status, undo and redo, theme control (Appearance and Theme radio groups), Export; Project and Settings as Dropdown Menus; mobile menu | all current header actions work; Workspace links keep `aria-current`; menus pass the floor |
| **S2 Docks** | Objects and Inspector on Sidenav (pinned, collapsed); auto-hide removed with a fallback (saved auto-hide becomes pinned); one Sheet with Objects and Inspector tabs below 980px in every Workspace; cards and desk background | existing dock tests pass after the rewrite rule; narrow layout keeps the canvas visible and interactive |
| **S3 Command palette** | per [command palette](command-palette.md) on Dialog and Combobox behaviour | Ctrl/Cmd+K opens it, including from a field; only existing events dispatched; floor passes |
| **S4 Toasts** | Toast provider and classification of the shell's transient messages | transient results toast; blocking states stay inline |
| **S5 Shell glue** | `workspace_navigation.rs`, `workbench_shortcuts.rs`, `workspace_composition.rs`, `inspector.rs`, `canvas_status_footer.rs`, `empty_board_canvas.rs` | no `Board*` or raw controls left in these files |

### Workspace lanes (after the Shell and Foundation lanes; at most 3 lanes run at once)

| Ticket | Scope (files from the [inventory](inventory.md)) | Acceptance |
| --- | --- | --- |
| **L1 Layout toolbar** | `layout_toolbar.rs`, `layout_transform_toolbar.rs`, `layout_align.rs`; Toolbar, Toggle Group, Dropdown Menu; Snap stays in the canvas toolbar | Stationary Controls Rule holds; Layout toolbar tests pass |
| **L2 Layout sweep** | the rest of `crates/layout` (18 files) and the Layout files in `web/src/presentation` (`layout_*`, `inspector/*`); Add and Mirror existing half as in-dock views | no raw controls or `Board*` in these files; tests rewritten |
| **C1 Case settings** | `case/mechanical_settings.rs` (86 `Board*` uses) | same |
| **C2 Case rest** | the other 7 files in `crates/case` and `presentation/case_workspace.rs` | same |
| **P1 Parts library and inspector** | `parts.rs`, preview panels, `presentation/parts_workspace.rs` | same |
| **P2 Parts editors** | assembly, model, profile and generator editors | same |
| **K1 PCB, Keymap, Keycaps** | `crates/pcb` (9), `keymap` (5), `keycaps` (4) and their `presentation/*_workspace.rs`; Tabs for keymap layers | same |
| **E1 Library and Export** | `crates/library`, project delete as Alert Dialog, `export_workspace.rs`, `zmk_firmware_export.rs`, `setup_workspace.rs` (Stepper in Accordion sections) | same |

For every workspace ticket: raw controls and `Board*` become the mapped components; inline messages are classified (field error, `Alert`,
`Callout`, `Toast`); icon-only buttons get a Tooltip; selects become `Select` or `Combobox` by the 12-option rule; tests find controls by
role and name; the lane deletes only the old CSS its files used.

### Cleanup lane (last)

| Ticket | Scope | Acceptance |
| --- | --- | --- |
| **X1 Delete the old layer** | `Board*`, remaining `.board-*`, `.m1-*`, `--wb-*` | the F5 check passes; no `controls.rs` wrappers |
| **X2 Documents** | rewrite DESIGN.md Layout and Components sections for the new structure (fixing its navigation contradiction); README commands if affected; glossary check | DESIGN.md matches the shipped shell |
| **X3 Merge gate** | full `scripts/check.py`, browser tests, accessibility checks, the user's review of every Workspace in both Themes and both Appearances at desktop, mid and narrow widths with screenshots, one two-axis code review, then the single merge to `dev` | every item in the branch plan's gate holds |

## 5. Risks and open items

- **Dropdown Menu fix is the largest single unknown** (wrong roles, no typeahead, focus loss). Estimate it first (F4); the Project, Settings,
  Transform and Layout command menus depend on it. Fallback: keep the `Popover`-based menus.
- **Sidenav adaptation** (in-flow form, per-side resize, narrow-screen behaviour) is custom behaviour, not a copy.
- **Test rewrite volume:** 144 lines in 15 files select by `.m1-*` classes, about 78 lines select native `input` and 14 `select`.
- **Measured on Chromium only**, with no screen reader; the floor says what was and was not measured.
- **CSS anchor positioning** is used by upstream's Popover and is not adopted (primitives position popups); no browser-support work needed.
- **Wasm size:** Inter adds 48 KB and `tw_merge` a little; the `icons` crate's 459 KB is avoided by vendoring.
- **Unpushed:** `dev` and `feat/rust-ui-migration` exist locally only; the prototype is on `prototype/theme-look` and the research on
  `research/rust-ui-catalogue` and `research/rust-ui-icons` (none merged).
- **Decisions marked revisable:** the 10px Original radius, the header height, the 12-option Select rule and the quiet shadcn selection
  are judgement calls the first lane can revisit with the user.
