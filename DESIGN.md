---
name: BoardStudio — The Keyboard Lab
description: A compact visual workbench with neutral panels and distinct geometry colors.
colors:
  light-field: '#f7f8fa'
  light-toolbar: '#edf0f8'
  light-toolbar-rule: '#c1cad6'
  dark-field: '#20242D'
  dark-toolbar: '#2C323D'
  dark-toolbar-rule: '#454E5E'
  light-accent: '#3858D6'
  light-accent-dark: '#2945b6'
  light-on-accent: '#FFFFFF'
  light-accent-secondary: '#8E5AA5'
  light-paper: '#f7f8fa'
  light-canvas: '#E9EEF2'
  light-panel: '#FFFFFF'
  light-raised: '#e9eef2'
  light-surface-soft: '#edf0f8'
  light-surface-strong: '#dce3ef'
  light-ink: '#182331'
  light-muted: '#576678'
  light-subtle: '#576678'
  light-rule: '#e2e7ed'
  light-rule-strong: '#9aaabc'
  light-rule-control: '#7D8B9A'
  light-selection-surface: '#E9EDFF'
  light-selection-ink: '#253EAC'
  light-topbar: '#ffffff'
  light-topbar-ink: '#182331'
  light-status-success: '#226c50'
  light-status-success-surface: '#e2f2e9'
  light-status-warning: '#8A4B00'
  light-status-warning-surface: '#FFF1D9'
  light-status-error: '#a2191f'
  light-status-error-surface: '#fff1f1'
  light-grid-small: '#d8e1e8'
  light-grid-large: '#b0c0cd'
  light-geometry-outline: '#596C86'
  light-geometry-fill: '#ddebe5'
  light-geometry-fill-muted: '#e2e9f2'
  light-geometry-key: '#3858D6'
  light-geometry-part: '#8E5AA5'
  light-geometry-label: '#182331'
  light-geometry-body: '#f7f9fc'
  light-geometry-board: '#397568'
  light-geometry-graphic: '#397568'
  light-geometry-zone: '#ddebe5'
  light-geometry-text: '#8E5AA5'
  keymap-key-face: '#252A32'
  keymap-key-ink: '#EDF1F7'
  keymap-key-rule: '#66717F'
  dark-accent: '#9AADFF'
  dark-accent-dark: '#b6c4ff'
  dark-on-accent: '#101725'
  dark-accent-secondary: '#B98CD1'
  dark-paper: '#1B1E25'
  dark-canvas: '#181B21'
  dark-panel: '#242831'
  dark-raised: '#373E4B'
  dark-surface-soft: '#2C323D'
  dark-surface-strong: '#343B48'
  dark-ink: '#EDF1F7'
  dark-muted: '#AFBCCB'
  dark-subtle: '#AFBCCB'
  dark-rule: '#30343B'
  dark-rule-strong: '#66717F'
  dark-rule-control: '#626D7B'
  dark-selection-surface: '#303B56'
  dark-selection-ink: '#D5DDFF'
  dark-topbar: '#1A1D22'
  dark-topbar-ink: '#EDF1F7'
  dark-status-success: '#6ab99d'
  dark-status-success-surface: '#18352d'
  dark-status-warning: '#F4BA68'
  dark-status-warning-surface: '#332A1D'
  dark-status-error: '#ff8389'
  dark-status-error-surface: '#491217'
  dark-grid-small: '#252A32'
  dark-grid-large: '#3A424E'
  dark-geometry-outline: '#8995A5'
  dark-geometry-fill: '#18352d'
  dark-geometry-fill-muted: '#223041'
  dark-geometry-key: '#9AADFF'
  dark-geometry-part: '#B98CD1'
  dark-geometry-label: '#EDF1F7'
  dark-geometry-body: '#252A32'
  dark-geometry-board: '#6AB99D'
  dark-geometry-graphic: '#6AB99D'
  dark-geometry-zone: '#18352d'
  dark-geometry-text: '#B98CD1'
typography:
  brand:
    fontFamily: "'Source Sans 3', 'Segoe UI', sans-serif"
    fontSize: 22px
    fontWeight: 650
    letterSpacing: -0.02em
  page-heading:
    fontFamily: "'Source Sans 3', 'Segoe UI', sans-serif"
    fontSize: 32px
    fontWeight: 650
    lineHeight: 1.12
  panel-heading:
    fontFamily: "'Source Sans 3', 'Segoe UI', sans-serif"
    fontSize: 18px
    fontWeight: 650
  preview-title:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 20px
    fontWeight: 600
  inspector-title:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 20px
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: -0.015em
  section:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 18px
    fontWeight: 600
    lineHeight: 1.45
  body:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: '''tnum'' 1'
  field:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 16px
    fontWeight: 500
    lineHeight: 1.45
  supporting:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.55
  measurement:
    fontFamily: '''Source Code Pro'', Consolas, monospace'
    fontSize: 16px
    fontWeight: 400
  metadata:
    fontFamily: '''Source Code Pro'', Consolas, monospace'
    fontSize: 12px
    fontWeight: 400
rounded:
  field: 8px
  control: 8px
  toolbar: 12px
  tool-pill: 16px
  menu: 5px
  flyout: 8px
spacing:
  compact: 4px
  inline: 8px
  field-gap: 12px
  panel-inset: 16px
  section-end: 24px
  page-inset: 32px
components:
  button-primary:
    backgroundColor: '{colors.light-accent}'
    textColor: '{colors.light-on-accent}'
    rounded: '{rounded.control}'
    padding: 6px 12px
  button-primary-hover:
    backgroundColor: '{colors.light-accent-dark}'
    textColor: '{colors.light-on-accent}'
  button-secondary:
    backgroundColor: '{colors.light-surface-soft}'
    textColor: '{colors.light-ink}'
    rounded: '{rounded.control}'
    padding: 6px 12px
  numeric-field:
    backgroundColor: '{colors.light-field}'
    textColor: '{colors.light-ink}'
    rounded: '{rounded.field}'
    typography: '{typography.measurement}'
  object-workspace-navigation:
    backgroundColor: '{colors.light-panel}'
    textColor: '{colors.light-muted}'
  revision-tag:
    backgroundColor: transparent
    textColor: '{colors.light-subtle}'
    rounded: '{rounded.field}'
    padding: 2px 5px
    typography: '{typography.metadata}'
  inspector-section:
    backgroundColor: '{colors.light-panel}'
    textColor: '{colors.light-ink}'
    typography: '{typography.section}'
  catalog-row-selected:
    backgroundColor: '{colors.light-selection-surface}'
    textColor: '{colors.light-ink}'
    rounded: '{rounded.field}'
    padding: 7px 8px
    typography: '{typography.field}'
  preview-switch:
    backgroundColor: '{colors.light-panel}'
    textColor: '{colors.light-ink}'
    rounded: '{rounded.control}'
    padding: 7px 10px
  command-group:
    backgroundColor: '{colors.light-toolbar}'
    textColor: '{colors.light-ink}'
    rounded: '{rounded.toolbar}'
    padding: 5px
  button-quiet:
    backgroundColor: transparent
    textColor: '{colors.light-ink}'
    rounded: '{rounded.control}'
    padding: 6px 12px
  button-primary-dark:
    backgroundColor: '{colors.dark-accent}'
    textColor: '{colors.dark-on-accent}'
    rounded: '{rounded.control}'
    padding: 6px 12px
  button-primary-hover-dark:
    backgroundColor: '{colors.dark-accent-dark}'
    textColor: '{colors.dark-on-accent}'
  button-secondary-dark:
    backgroundColor: '{colors.dark-surface-soft}'
    textColor: '{colors.dark-ink}'
    rounded: '{rounded.control}'
    padding: 6px 12px
  numeric-field-dark:
    backgroundColor: '{colors.dark-field}'
    textColor: '{colors.dark-ink}'
    rounded: '{rounded.field}'
    typography: '{typography.measurement}'
  object-workspace-navigation-dark:
    backgroundColor: '{colors.dark-panel}'
    textColor: '{colors.dark-muted}'
  revision-tag-dark:
    backgroundColor: transparent
    textColor: '{colors.dark-subtle}'
    rounded: '{rounded.field}'
    padding: 2px 5px
    typography: '{typography.metadata}'
  inspector-section-dark:
    backgroundColor: '{colors.dark-panel}'
    textColor: '{colors.dark-ink}'
    typography: '{typography.section}'
  catalog-row-selected-dark:
    backgroundColor: '{colors.dark-selection-surface}'
    textColor: '{colors.dark-ink}'
    rounded: '{rounded.field}'
    padding: 7px 8px
    typography: '{typography.field}'
  preview-switch-dark:
    backgroundColor: '{colors.dark-panel}'
    textColor: '{colors.dark-ink}'
    rounded: '{rounded.control}'
    padding: 7px 10px
  command-group-dark:
    backgroundColor: '{colors.dark-toolbar}'
    textColor: '{colors.dark-ink}'
    rounded: '{rounded.toolbar}'
    padding: 5px
  button-quiet-dark:
    backgroundColor: transparent
    textColor: '{colors.dark-ink}'
    rounded: '{rounded.control}'
    padding: 6px 12px
---

# Design System: BoardStudio

## Overview

**Creative North Star: "The Keyboard Lab"**

The Keyboard Lab is a precise workspace for exploring and refining keyboard designs. Neutral panels frame a dominant drawing surface. Color identifies actions, geometry, and state; labels and measurements carry the detail. The optional dark palette uses closely related charcoal-blue surfaces, a clearer field and toolbar separation, and softer rounded controls.

The working character is calm, precise, and restrained: compact controls, aligned values, modest corners, and short state transitions. Catalog, canvas, and inspector share one workspace, with optional properties organized into disclosures. These descriptions record the approved unified workspace now implemented locally.

Shared typography follows a readable 12 / 14 / 16 / 18 / 20 / 24 / 32px scale,
with a 22px brand mark. Shared spacing follows 4 / 8 / 12 / 16 / 24 / 32px.
Structural dividers stay subdued; editable fields retain a clear boundary, and
focus, selection, validation and CAD geometry keep their distinct visible states.

**Key Characteristics:**

- BoardStudio's light slate surfaces and white panels around a dominant canvas; dark role tokens remain available as an optional theme.
- Action blue, Board teal, Pad violet, and dashed key boundaries.
- Compact fields and rule-separated inspector disclosures.
- Fixed canvas controls and independently scrolling side panels.
- Light and dark palettes with a persisted System option.

The current workbench uses one header, docked Objects and Inspector panels, solid contextual toolbars, and shared semantic controls. The OpenDesign Application prototype follows BoardStudio's light theme and guides control grouping; current Rust/Dioxus code remains the source for supported behavior and edit ownership. Older Graphite explorations are archived in [the reference packet](docs/design/graphite/approved-suite/) and are not the target for this prototype. Shell, panel, Inspector and geometry styles use semantic `--wb-*` roles in [Tailwind source](web/tailwind.css) and [the base stylesheet](web/assets/m1.css); workspace components live in [presentation](web/src/presentation/).

The same dock vocabulary now covers PCB Placement/Wiring, Case assembly and
construction disclosures, Keymap layer and key editing, Keycaps board appearance and
selected-key overrides, and the Parts catalogue. Contextual shortcuts reveal and
focus existing Inspector editors; they do not own document edits. Hidden PCB
panes and collapsed disclosures retain their mounted draft owners.

The project browser temporarily hides the mounted editor while selecting a saved
keyboard and its real geometry preview. Setup uses a dedicated process surface
beside the actual viewer; pre-layout hardware silhouettes are explicitly
schematic. Export shows the accepted project revision and local-save state
separately from portable downloads and fabrication readiness.

## Colors

Role-led names describe the implemented palette. The frontmatter records separate
`light-*` and `dark-*` values; neither theme is a global inversion. Removing the
mode prefix from a paired color key gives its corresponding `--wb-` CSS token
suffix. The `keymap-*` values are a fixed dark key-face treatment shared by both
themes.
Tokens in this document record the implementation; update the document and its
sidecar together when the implementation's palette changes.

### Primary

- **Action blue:** `accent` marks primary actions, active navigation, selected
  objects, and most inspector focus rings. `on-accent` is the paired foreground.
- **Action hover:** `accent-dark` is the existing hover token. Its name is legacy:
  it becomes lighter in the dark theme.
- **Selection surface:** `selection-surface` marks selected catalog and tree rows.
  Selected buttons and toggles use the filled surface without a persistent outline;
  keyboard focus keeps its 2px Action blue ring.

### Secondary

- **Secondary violet:** `accent-secondary` supplies secondary emphasis.
- **Legacy `teal` token:** despite its name, `teal` currently equals Action blue
  in both themes and no stylesheet rule consumes it. Do not give it a new role;
  use `accent` or a geometry/status token instead.

### Tertiary

- **Board teal:** `geometry-board` outlines the board, `geometry-fill` fills it,
  and `geometry-graphic` draws footprint graphics.
- **Key blue:** `geometry-key` marks key boundaries and handles.
- **Pad violet:** `geometry-part` fills pads and draws copper traces and vias.
- **Geometry support:** `geometry-outline`, `geometry-body`, `geometry-label`,
  `geometry-text`, and `geometry-zone` keep boundaries, bodies, labels, and zones
  distinct. `geometry-fill-muted` supports inactive matrix cells.
- **Keymap keys:** the fixed `keymap-key-face`, `keymap-key-ink`, and
  `keymap-key-rule` tokens keep binding labels legible across themes.
- **Status colors:** `status-success`, `status-warning`, and `status-error` have
  separate surface tokens for filled backgrounds. Geometry keepouts and cutouts
  currently reuse the error color and retain dashed boundaries.

### Neutral

- **Neutral canvas:** `canvas` is the drawing, footprint, and 3D preview field.
  `grid-small` and `grid-large` form its quiet measurement grid.
- **Shell surfaces:** `paper`, `panel`, and `raised` distinguish the application,
  permanent panels, and raised or hovered areas. `surface-soft` and
  `surface-strong` provide additional interaction surfaces.
- **Field and toolbar surfaces:** `field` distinguishes editable controls; `toolbar` and `toolbar-rule` back the contextual actions with a solid contrasting surface.
- **Content and rules:** `ink`, `muted`, and `subtle` form the text hierarchy;
  `rule` and `rule-strong` separate panels and outline controls.

**The Shared Token Rule.** Tailwind utilities and BoardStudio controls consume the same `--wb-*` roles. Keep workspace state and document edits out of visual primitives.

**The Paired Foreground Rule.** Use each filled action with its designated foreground; ordinary content colors are not interchangeable with on-accent colors.

**The Geometry Role Rule.** Keep canvas, board, key, pad, and selection roles separate from shell surfaces and status roles.

[The shared viewer](web/crates/case/src/shared_viewer.rs) uses a transparent WebGL background
over the shared canvas token, with board, key, and part geometry colors on
its materials. Lighting remains specific to the 3D scene, so shaded materials
are not flat swatches of those colors.

### Known implementation exceptions

These are recorded defects or limitations, not patterns to reproduce:

- Some legacy auxiliary actions still use ordinary ink on the action fill;
  these need a separate paired-foreground audit when changed. The table below
  covers the explicit on-accent pair, not every legacy consumer.
- The existing palette is not a blanket contrast guarantee. Inspect real
  foreground/background pairs, opacity, focus states, and selected surfaces.

Calculated from current opaque CSS pairs (normal text target 4.5:1):

| Pair | Light | Dark |
| --- | --- | --- |
| Primary text | 15.87:1 | 13.89:1 |
| Muted text | 5.87:1 | 8.16:1 |
| Action | 5.94:1 | 8.36:1 |
| Action hover | 8.01:1 | 10.53:1 |
| Selection | 7.63:1 | 7.77:1 |
| Success | 5.44:1 | 5.70:1 |
| Warning | 6.10:1 | 8.10:1 |
| Error | 7.09:1 | 6.41:1 |

These checks do not certify every composited state, geometry layer, or 3D material.

## Typography

Source Sans 3 is the bundled face for headings, panes, menus, and controls.
Its open forms and compact proportions keep dense labels readable without
changing the workbench's palette or hierarchy. Source Code Pro is reserved for
measurements, code, and canvas technical labels. Tabular numerals are enabled
on the workbench root.

Both upright variable WOFF2 fonts are self-hosted, use `font-display: swap`, and
are included in the generated offline cache. The main UI face is preloaded;
monospace loads on demand. Segoe UI/sans-serif and Consolas/monospace remain
fallbacks. Source provenance and checksums live in `web/assets/fonts/README.md`;
SIL OFL licenses ship in `web/assets/licenses/fonts/`.

- Preview titles use 20px, selected-object headings use 21px, and panel headings
  use 18px. Disclosure headings and field values are generally 15px; compact
  labels range from 12px to 13px, with metadata at 10px.
- Measurements use Source Code Pro. The body and numeric value role is 15px on
  desktop and 14px on narrow screens; help copy uses a 1.55 line height.
- The main type roles contract on narrow screens: the page heading becomes 30px,
  body and control text 14px, and the brand 22px. Browser zoom remains available.

**The Measurement Rule.** Keep values and units aligned, use tabular numerals,
and keep precise values available alongside direct manipulation.

## Layout

The application fills the viewport. On desktop, a 64px single-row header and
48px footer frame the independently scrolling Objects dock, canvas, and
Inspector. At 760px and below, the header becomes 56px. The center expands to
absorb available width. At 980px and below, Layout replaces side panels with a
bottom settings sheet and Objects/Inspector tabs. Expand and reduce controls set
ordinary sheet height to 42% and expanded height to 62%, capped to retain 340px
for the canvas area and tabs. The canvas remains visible and interactive above
it. The Objects sheet uses one scroll region for the tree, inline Snap, and Canvas
visibility; other workspaces keep their existing panels.

| Width | Implemented arrangement |
| --- | --- |
| Above 1150px | Resizable Objects and Inspector docks, flexible center; default widths are clamp-based |
| 981–1150px | Dock defaults narrow to 220px Objects and 300px Inspector; persisted widths remain bounded |
| 980px and below, Layout | Bottom settings sheet with Objects and Inspector tabs; interactive canvas remains visible above |
| 980px and below, other workspaces | Existing panel and drawer behavior |

Workspace links use flat tabs in the header, outside the Objects dock, with an
accent underline for the active workspace. The rounded pill design is reserved
for canvas toolbars. The
wide header keeps project identity, workspace navigation, save status, history
actions, and Export. At 1100px and below, workspace links use icons with accessible
names and titles; at 760px and below, navigation occupies a second header row so all
six workspaces remain visible without horizontal scrolling. History buttons move into the footer
at 1100px and below; brand text and save labels hide at 1240px and below, while
the save status itself hides on narrow mobile screens.
Layout uses two fixed horizontal strips above the canvas. The upper strip
selects Matrix, Row, Column, Key, or Part with an icon and short label. The lower
contextual action bar shows Position, Stagger, Offset, and Splay as relevant,
followed by Align and a secondary Transform menu. The strips do not switch density
or horizontally scroll; coarse-pointer controls use 44px targets.

Both desktop panels support pinned, collapsed, and opt-in auto-hide modes.
Widths (navigator 200–420px, inspector 280–480px) and modes persist locally;
Workspace settings owns appearance and restoring this panel layout. The Project
menu owns naming, opening, new projects, and saving portable copies.
Collapsed panels release their full column and reopen from narrow edge controls.
Auto-hide leaves the same reveal control and opens the panel over the canvas;
pointer presence, keyboard focus, and resizing prevent dismissal. Panel reveals
slide over 220ms. Narrow screens use explicit panel controls instead of hover
behavior. Resizing supports pointer dragging and arrow keys. Layout's Add action
sits beside the Objects heading and opens a create view within the dock.

The Objects dock keeps direct Layout, PCB, Keymap, Keycaps, Case, and Parts links
available even when the header is compact. The object tree is contextual to the active view and shows the relevant
selectable objects. Layout also exposes PCB, Case and Keycaps as navigation leaves
alongside Board, using the existing workspace state without changing document
selection. Export remains a dedicated header action and page.
When open, the setup guide occupies the left workspace in place of the object
tree. Project → Setup guide is its entry point, and the guide header's
Back to objects control returns to the contextual tree. The current-step list is
one bounded scrolling region and uses the same panel surface, rules, typography,
and selection tokens as Objects. SetupGuide uses
`is-current` and `is-ready` step states, with marker and accent treatment rather
than nested cards. On narrow widths it follows the same left drawer and scrim
behavior as Objects.

The guide opens automatically after a new project is saved successfully. Its
four accordion sections are Project, Layout, Wiring, and Case. A new project opens
in the Layout workspace while the guide starts at Project. In Case, Full settings
shows the existing MechanicalSettings in the left setup drawer; its back arrow
returns to the guide without opening the Inspector or dismissing the guide.
Continuing from Case returns to Layout and dismisses the guide. Project → Setup
guide and Back to objects provide explicit entry and exit paths. The guide
open state and selected section are browser preferences scoped to the project;
completion comes from the current document, selected board, wiring, and
geometry. Sections remain freely navigable.
The legacy Review stage exists only in persisted state; saved Review selections
map to Case. Existing projects retain the ordinary workbench and each exporter
remains authoritative. Returning from a temporary Parts or Export task returns to
the prior design view; Back to objects returns to the contextual object tree.

Recurring spacing uses compact 4px gaps, 8px inline separation, 12px field gaps,
16px inspector insets, and 20px section endings. These are extracted repeated
values rather than an enforced global spacing scale. Most compact-panel rules
switch at 760px; the Export workspace uses 24px 18px 36px padding there.

**The Stationary Controls Rule.** Scope and snap controls remain anchored to the workspace while world geometry pans and zooms.

The Case action bar owns geometry readiness and its next action: configure,
generate, wait/cancel, retry, review errors, or export. Export requires valid captured
accepted inputs and matching prepared manufacturing geometry for the configured
board and revision. Applicable mechanical errors block the outputs they affect;
warnings remain reviewable without blocking. Disposable display-preview currency
does not grant or revoke export ownership
([ADR-0016](docs/adr/0016-export-ownership-is-separate-from-preview-currency.md));
changing the accepted source still invalidates stale output. The canvas labels only
the geometry currently displayed. Configuration metadata or a visible preview alone
does not imply generated-solid or manufacturing readiness.

Layout findings and Mechanical findings have separate, explicitly scoped footer
controls. Counts match the grouped lists they open. Mechanical review opens and
focuses diagnostics, including from a closed compact inspector. Model notices
remain separate from mechanical findings.

The mechanical inspector orders active diagnostics, Construction, Inherited part profiles,
Dimensions & clearances, and Resolved stack before optional openings/battery,
mounting/hardware, and manufacturing overrides. Async results can open an
untouched disclosure; ordinary edits preserve a user's open/closed choice.
Disable mechanical stack belongs in Configuration management at the end.
Part fit comes from the key assembly or definition selected in Parts. The inherited
summary links to that definition's fit editor, where standard switch profiles,
custom cutouts and clearances, and imported KiCad geometry can be reviewed before
saving. Opening or cancelling the editor does not change the document. Existing
Case-specific profile overrides remain under Advanced source geometry.

Layout's Canvas visibility and Snap are inline accordions at the bottom of
Objects. PCB, Case, and Parts keep their workspace-specific layer controls.
Expanding a bounded visibility list never changes drawing dimensions or camera.
Availability is separate from visibility preference: generated solids show
Not generated, while unavailable component assets show Missing model. Preferences
remain view-only, survive generation, and assembly preferences are scoped to the
project and board. Long labels wrap; lists scroll inside.

Assembly display and assembled/exploded/section controls occupy a wrapping rail
above the drawing. Camera controls remain with the viewport; assembly visibility
stays in Objects.
Compact Case settings has a visible label, and compact generation, layer, and
settings controls use 44px touch targets. Passive canvas hints do not intercept
geometry selection or gasket dragging.

## Elevation & Depth

Persistent panels are flat and separated by fine rules and surface tone. Menus,
popovers, dialogs, and mobile drawers use offset shadows to establish their
position above the workspace. Floating canvas controls and hints have smaller
shadows. Save/live dots use the success foreground and have no halo in the unified shell.

- Project library and dialogs: `0 12px 40px rgb(0 0 0 / 30%)`.
- Panel options, Add menu, matrix setup, and toolbar popovers: `0 8px 22px rgb(0 0 0 / 20%)`.
- Save-status popover: `0 8px 24px rgb(0 0 0 / 20%)`.
- Command menus and case view settings: `0 8px 24px #0005`.
- Bottom-dock popovers: `0 10px 28px rgb(0 0 0 / 22%)`.
- Mobile drawers: `±8px 0 24px rgb(0 0 0 / 16%)` toward the canvas.
- Canvas toolbar: `0 4px 16px #18233120`; view groups: `0 3px 10px rgb(0 0 0 / 10%)`.
- Canvas hint: `0 4px 14px #24373312`.

**The Structural Shadow Rule.** Use tonal surfaces and fine rules for persistent panels; reserve offset shadows for transient overlays and floating controls.

## Shapes

Shared inspector fields and action buttons use 5px corners; the contextual
toolbar uses 6px corners. Menus and flyouts use 5–8px corners. Persistent
workspace panels remain rectangular. Selection
modes live in a labeled command menu; the earlier rounded floating scope dock
is superseded.

Thin rules organize the dense controls. Selection makes the relevant boundary
stronger rather than changing the object's silhouette. Inline SVG icons use
consistent outline strokes; geometry uses actual editable and compiled shapes.

## Components

### Actions

Shared `BoardButton` primary actions use Action blue with the paired foreground,
8px corners, 12px horizontal padding, and a 40px minimum height (34px compact;
44px for coarse-pointer controls). The Parts placement action fills the
Inspector width. Hover uses Action hover for primary buttons and a soft surface
for ordinary buttons. Secondary buttons use the quiet surface fill without an
outline; quiet actions are transparent.

Keyboard focus is a 2px Action blue ring with 2px offset. Shared buttons dim to
0.58 opacity when disabled; some legacy inspector actions use their own muted
surface or opacity treatment.

### Fields

Fields combine a label, editable value, and adjacent unit. Shared numeric fields
use a 1px outline, 5px corners, and a 40px outer height (44px on coarse pointers).
The inner measurement control uses Source Code Pro. Focus shows a 2px Action
ring with a 2px offset; invalid fields use the status-error border and nearby
error text.

### Workspace navigation and preview switching

Layout, PCB, Keymap, Keycaps, Case, and Parts are text-labeled links in the
Objects dock. The current workspace uses `aria-current="page"` and a soft
surface; workspace navigation is not duplicated in the header. Export remains
a dedicated header action and page with its own artifact, readiness, and review
context. Save status sits beside project identity. Undo and Redo remain in the
header on wide screens and move to the footer at 1100px and below. The Parts
preview uses a compact icon pill with accessible names and a tinted pressed state
to switch between 2D footprint and 3D model. Camera and render controls use the
same pill treatment in a reserved row above the 3D canvas so they do not cover
the model. In Layout, 2D, 3D, and Footprints remain anchored at the canvas
bottom-left, with zoom and fit in the footer.

Add groups Layouts, contextual matrix actions, Parts, and Board geometry into
named sections. Components prioritizes power/reset hardware; Controllers, Displays,
and Encoders have separate disclosures. Search includes keyboard switches and
sockets, while Browse all parts opens the library. Mirror existing half links
one or all unpaired matrices without moving the original keys; independent
hardware stays local. Y-mirrored frames and constrained keys must be resolved
before linking. Mirrored pair opens a focused setup over the canvas,
then previews both halves and a centre axis before a single placement action.
The halves become named layouts beneath the board. Their key geometry is linked
in both editing directions; switch choices and components remain local. The
inspector names the partner and offers Unlink halves. Add provides a layout
destination for extra components, which can also be reassigned in the inspector.
Left half and Right half group their layouts beneath the board. Automatic
perimeters are built separately across linked split axes, including for exports;
PCB and case settings remain shared at board level.

### Catalog rows and metadata tags

Catalog rows use compact full-width buttons with 34px minimum height, 7px 8px
padding, and a name plus muted detail. Selection combines a tinted surface and
Action blue border. Revision tags are small outlined mono labels (10px) with
2px 5px padding; they are metadata rather than action chips.

### Inspector disclosures

Keys, rows, and columns expose width and height sliders in quarter-unit steps
from 1u to 7u. Wide/Tall swaps the dimensions; a row or column applies the change
to all selected keys in one edit. Units use matrix pitch minus the configured
edge gap. Explicit cap dimensions stay linked across paired halves. Key Assembly
names the key definition selector; diode composition belongs to the assembly,
not a separate key checkbox.

The inspector starts with the selected object, description, and primary edit or
action. Optional groups use native disclosures with a drawn chevron, a quiet top
rule, and an optional trailing detail. Summary rows have a 46px minimum height
(48px at narrow widths); open content uses 12px gaps and 20px bottom padding.

**The Disclosure Rule.** Separate inspector sections with quiet rules and reveal optional settings in place, preserving the selected object as the editing context.

### Workspace grouping and layer names

The header owns project actions and flat workspace navigation tabs. The Objects dock holds
the active workspace's object tree; contextual tools sit beside the canvas, and
the Inspector begins with the selected object and its common edits. Group
specialist controls by task and use clearly named, single-level disclosures for
optional settings.

Use distinct names for distinct meanings: Canvas visibility, PCB layers,
Keymap layers, Case assembly visibility, and Parts preview layers. In Keymap, the
Objects dock is titled **Keymap**, its list is **Keymap layers**, and the Inspector
is the **Keymap editor**. Keep visibility controls view-only, and preserve each
workspace's existing state owner and edit path.

### Parts catalog

New placements use one preferred entry per supported key part: Ceoloide MX,
Choc V1/V2, Gateron KS27/KS33, matrix diode, and SK6812 MINI-E. Solder and
hotswap are settings within a switch entry. The eight assembly presets remain;
Choc presets explicitly identify V1 compatibility. Parts, Add component, and
assembly selectors share these choices and searchable labels.

Retired built-in key parts and the duplicate Infused Kim Choc and diode remain
resolvable in saved projects. Existing assignments retain their current selector
option; opening a project never replaces them. User-created and imported parts
remain available, including similarly named parts. Internal placement snapshots
are not separate catalog products. **Update assembly preset** is an explicit,
undoable matrix action that refreshes its recipe and definition snapshots. Added
rows and columns inherit that recipe; linked halves retain their own companion
parts and existing key edits.

### Drawing and footprint workspace

Board outlines and component boundaries remain separate from the slate field.
Pads are filled violet; drill holes expose the canvas. Keycap and courtyard
guides in the Parts preview remain dashed. The Layout canvas offers a separate
keycap overlay with a rounded outer envelope and inset top face, using the
resolved key dimensions and existing geometry/selection tokens. Matrix membership
identifies keys even when their definition is a socket; pitch minus edge gap
supplies missing key dimensions. A Relations reference hover or focus marks its parts with a dashed `accent-secondary` (violet) outline, never the selection surface or accent stroke, and clears on leave, blur, selection change or removal. The Layout canvas-visibility control has independent Keys, Components, Keycaps, Footprints,
and Board switches; all start visible except Footprints. The toolbar Footprints
switch shares that view-only state. Visibility survives workspace changes without
creating document revisions or history entries. The accordion does not change
canvas bounds or camera and retains keyboard focus behavior.
Layer swatches distinguish
Action blue keys, amber components, neutral keycaps, violet footprints, and
teal boards using the existing theme tokens.
Selection types have direct icon shortcuts. Stagger, Offset, Splay, and Origin activate
explicit canvas tools; Ctrl held at pointer-down turns a Stagger drag into a quick
Offset of its column; handles remain hidden outside those tools. Origin snapping
uses component origins, corners, midpoints, and physical edges before the grid. PCB draws pad
shapes and drills at their authored positions and rotations, plus supported
bundled footprint graphics. Its collapsible Layers list groups the available
copper and technical layers separately from object visibility. View toggles do
not modify the document or exports. Command details expand inside the floating
pill’s shared surface; at narrow widths the toolbar docks and the layer list
starts collapsed. Footprint labels and the dimension footer identify
what is shown. The 2D preview has a compact, collapsible Layers legend with
visibility controls for named footprint graphics layers, copper, guides, drill
marks, pad numbers, and each previewed part. These controls affect only the
preview; they do not edit the component or export. Missing or failed model
previews use explicit text and a retry or import action instead of a perpetual
loading state.

Layout canvas chrome keeps fixed screen sizes at every zoom: technical labels
(dimmed-half, spacing note, part names) draw at 11 px mono, and the anchor
ring draws at 7 px violet, each mapped from screen pixels to canvas units for
the current view. The badge label lives outside the canvas in a "Placed from"
readout at the canvas's bottom left, in 11 px mono violet. Violet is only for
anchoring — the ring, the tie, the readout and part outlines, with a selected
part turning accent. Amber is only for keys that move — the field and drag
previews — and the spacing note. Key hover is a 1.5 px accent stroke over a
60% selection fill, and solid muted on the dimmed half.

### Motion and browser surfaces

Motion tokens are `motion-feedback` (120ms), `motion-reveal` (180ms), and
`motion-panel` (220ms), with the shared decelerating curve
(`cubic-bezier(.16, 1, .3, 1)`). Setup steps unfold in place; compact panels
travel from their opening edge; buttons and switches acknowledge input with
short color, checkmark, and press feedback. Canvas geometry remains still.
Reduced motion removes spatial travel and accordion animation while retaining
brief color feedback.

## Do's and Don'ts

### Do

- Do pair light or dark role tokens consistently within a surface.
- Do retain distinct board, key, pad, and selection treatments.
- Do keep measurement units adjacent to editable values.
- Do give each panel its own bounded scrolling region.
- Do preserve native disclosure controls and keyboard focus, and add reduced-motion handling to any new movement.

### Don't

- Don't treat the original (v1) app's design document as BoardStudio's palette.
- Don't substitute shade or hue names for a control's actual role.
- Don't spread geometry colors across unrelated panel backgrounds.
- Don't copy the documented contrast and status-indicator defects into new components.
- Don't present a saved screenshot or this document as proof of current accessibility compliance.

### Workbench typography

Keep the established type roles: 18px panel headings, 21px selected-object
headings, 15px body and editable values, 12–13px compact labels, and 10px mono
metadata. At 760px and below, the page heading, body, and brand roles contract
to 30px, 14px, and 22px. This dense editing interface uses a 1.45 UI line height
and 1.55 for help text. Use the bundled Source Sans 3 and Source Code Pro font
stacks; use monospace for measurements and code, not ordinary labels. Tree
references do not shrink to make room for secondary descriptions; truncated
names expose their full text on hover. Browser zoom remains available.

### Workbench motion

The centered Layout and PCB pill uses the selected anchored-popover study:
scope buttons open their real contextual actions from the clicked control, with
an 8px rise and .96-to-1 scale over 180ms. The selected scope highlight slides
between fixed slots over 240ms; the contextual row reveals below over 220ms.
The Layout command chevron rotates over 180ms;
side panes slide and fade over 220ms, with the compact Inspector entering as a
bottom sheet. Setup steps unfold in place. Shared controls acknowledge presses
and checkbox changes within 120ms. Canvas geometry stays immediate so drag and
placement remain precise. No loops, page-load choreography, or motion dependency
is used.

Reduced motion removes panel travel, drawer entrances, and accordion movement;
brief color feedback remains so state changes are still clear.

### Workbench controls and review

Layout presents Matrix, Row, Column, Key, and Part in a centered horizontal
icon-and-short-label pill above the canvas, with Align and Transform sharing its
borderless surface. A contextual pill below offers Position, Stagger, Offset,
and Splay as relevant. PCB reuses the same shell and anchored control menus.
Narrow canvases retain the scope icons and hide their short labels. There is no compact-density toggle or
horizontal toolbar scrolling. Transform subfunctions use a small
selection-aware popover with visible labels. Snap and Canvas visibility appear
inline in the Objects sheet. Workspace switching uses flat header tabs, separate
from the canvas toolbar pills. This group links Layout, PCB, Keymap, Keycaps, Case and Parts and marks
the current workspace. Domain-specific tree contents and visibility owners stay
with each workspace. Other workspaces retain their existing selection selector. The
2D/3D switch occupies a separate control so view changes are distinct from editing. Rows/Columns tree grouping belongs in
Objects options. Panel menu actions occupy full-width rows, and Escape closes
the menu without changing the canvas selection.

Shared controls live in `web/crates/ui-shared/src/controls.rs`, with token-based
styles in `web/tailwind.css`. Use `BoardButton` variants for form and task actions;
pass `type="submit"` explicitly for submission. Keep bespoke canvas, tree and
navigation affordances in their owning workspace.

`BoardCheckbox` keeps native checkbox and switch form events and label
activation while its shared CSS supplies the themed control rendering. `BoardDisclosure` retains native details/summary keyboard behavior
and keeps child drafts mounted while collapsed; controlled headings keep their
existing handlers. Layout transform groups and key assembly groups collapse
independently, starting open. `BoardAccordionSection` uses the Dioxus inline
accordion primitive for compact workspace sections. Use the mounted native
disclosure for draft-owning fields: the pinned Dioxus AccordionContent currently
unmounts closed content.
`BoardDialog` provides native modal confirmation or an inline,
non-modal task surface. Pending actions control whether dismissal is permitted.
Browser file pickers remain native. Dioxus Components primitives own popovers
and toggles; native form, disclosure and dialog semantics stay intact where they
already provide the required behavior.

Compact headers retain the active board name and a focusable local-save status.
The status follows actual storage writes; a failed write must not show as saved.
Undo, Redo, and Export remain available as labeled Project menu actions when
the compact header hides their desktop buttons.

Findings name their severity and affected target, consolidate matching board and
outline reports, and offer navigation when the target still exists. Corner-fit
warnings describe the resulting smaller corners without making fabrication
claims. Unresolved targets retain the diagnostic without a dead action.

Fit board and Fit selection remain in the footer beside zoom. Fitting includes
transformed part and keycap bounds, reserves room for canvas controls, and uses
empty default bounds only when no geometry exists. Selecting a half in the tree selects its
matrices and components for inspection with Fit selection.


### Wiring and physical assemblies

The PCB inspector identifies the selected part and its named terminal assignments.
The controller chooser uses a horizontal carousel of source-backed, trimmed 2D
MCU footprints to compare package outlines and pads. Search smoothly centers a
matching controller and narrows the carousel; arrow buttons and native scrolling
remain available. Browsing changes preview state only; Place controller retains
the existing placement action. Key switches inherit wiring from the board plan; the Wiring section owns
controller selection, explicit matrix/direct mode, automatic resolution, pin locks and manual
assignments. Mechanical holes never appear as electrical terminals. Peripheral pins
are allocated before scan pins. Missing profiles, diode polarity, capacity conflicts,
and manual-net conflicts remain explicit findings rather than guessed connections.
When existing connections conflict with the automatic plan, Wiring names the
affected nets and requires an explicit replacement action. That action releases
only the conflicting terminals, preserves other manual connections, and supports
Undo.

The firmware keymap starts with unassigned keys and offers an editable binding for
each current key and encoder push button. Layout edits define a new hardware/keymap
revision; deleted-key tombstones and automatic keymap migration are not promised.
PCB downloads include a wiring report and per-part jumper instructions for each
physical population sharing the board. Draft
handoffs retain unresolved findings. Successful PCB handoffs protect valid signal
assignments, including drafts; protection survives Undo. Starting a new PCB revision
requires the explicit remap review in Wiring. Failed exports record no handoff.

Reversible recipes preserve fabricated local nets across open solder gaps and use
part IDs rather than editable references for their names. Instructions identify the
face and bridge state; optional omitted footprint traces are routing obligations.
Firmware resolves the module GPIO behind each selected population, including the
reduced MCU jumper variant. Wired halves use local power, crossed TX/RX through a
straight TRRS cable, sleeve ground, and an unused ring 1.

Physical assembly selection is separate from the PCB design. A split may share one
reversible PCB or use two boards. Turning a half over changes its face and reflects
its physical X/Z placement. Mechanical features stay local to each half by
default. An explicit mechanical
mirror pair links mounts, closure mounts, authored Case openings and gasket
support anchors across both halves by default, shares construction dimensions,
materials and processes within the pair, and settles linked edits from either
half in one Undo step. Per-feature exceptions are explicit: Make local, remove
on one half, or rejoin with a chosen source. Battery space remains local. PCB
thickness remains authoritative. Case geometry
is retained across electrical-only edits only when its exact physical dependency
signature and project/instance identity still match.
The Case preview uses that instance's reflected parts and outline together. Layout
2D and 3D continue to show the canonical PCB design.

ZMK handoffs target v0.3.0 and contain editable source, pin assignments, and build
configuration. Source generation and device-tree syntax checks do not substitute
for a complete target firmware build or hardware verification.
