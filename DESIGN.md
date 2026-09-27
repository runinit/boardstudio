---
name: Board Studio v2 — The Keyboard Lab
description: A compact visual workbench with neutral panels and distinct geometry colors.
colors:
  light-accent: '#3858D6'
  light-accent-dark: '#2945b6'
  light-on-accent: '#FFFFFF'
  light-accent-secondary: '#8E5AA5'
  light-teal: '#3858D6'
  light-paper: '#f7f8fa'
  light-canvas: '#E9EEF2'
  light-panel: '#FFFFFF'
  light-raised: '#e9eef2'
  light-surface-soft: '#edf0f8'
  light-surface-strong: '#dce3ef'
  light-ink: '#182331'
  light-muted: '#576678'
  light-subtle: '#576678'
  light-rule: '#dbe1e8'
  light-rule-strong: '#9aaabc'
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
  dark-accent: '#9AADFF'
  dark-accent-dark: '#b6c4ff'
  dark-on-accent: '#101725'
  dark-accent-secondary: '#B98CD1'
  dark-teal: '#9AADFF'
  dark-paper: '#151c26'
  dark-canvas: '#101a22'
  dark-panel: '#1c2632'
  dark-raised: '#293443'
  dark-surface-soft: '#263246'
  dark-surface-strong: '#344562'
  dark-ink: '#EDF1F7'
  dark-muted: '#AFBCCB'
  dark-subtle: '#AFBCCB'
  dark-rule: '#344151'
  dark-rule-strong: '#61738a'
  dark-selection-surface: '#304373'
  dark-selection-ink: '#CDD6FF'
  dark-topbar: '#16222d'
  dark-topbar-ink: '#EDF1F7'
  dark-status-success: '#6ab99d'
  dark-status-success-surface: '#18352d'
  dark-status-warning: '#F4BA68'
  dark-status-warning-surface: '#332A1D'
  dark-status-error: '#ff8389'
  dark-status-error-surface: '#491217'
  dark-grid-small: '#203040'
  dark-grid-large: '#344b60'
  dark-geometry-outline: '#6F87A3'
  dark-geometry-fill: '#18352d'
  dark-geometry-fill-muted: '#223041'
  dark-geometry-key: '#9AADFF'
  dark-geometry-part: '#B98CD1'
  dark-geometry-label: '#EDF1F7'
  dark-geometry-body: '#223041'
  dark-geometry-board: '#6AB99D'
  dark-geometry-graphic: '#6AB99D'
  dark-geometry-zone: '#18352d'
  dark-geometry-text: '#B98CD1'
typography:
  preview-title:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 20px
    fontWeight: 600
  inspector-title:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 23px
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: -0.015em
  section:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 13px
    fontWeight: 600
    lineHeight: 1.45
  body:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: '''tnum'' 1'
  field:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 12px
    fontWeight: 500
    lineHeight: 1.45
  supporting:
    fontFamily: '''Source Sans 3'', ''Segoe UI'', sans-serif'
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.55
  measurement:
    fontFamily: '''Source Code Pro'', Consolas, monospace'
    fontSize: 12px
    fontWeight: 400
  metadata:
    fontFamily: '''Source Code Pro'', Consolas, monospace'
    fontSize: 10px
    fontWeight: 400
rounded:
  field: 4px
  control: 4px
  menu: 5px
  flyout: 8px
spacing:
  compact: 4px
  inline: 8px
  field-gap: 12px
  panel-inset: 16px
  section-end: 20px
components:
  button-primary:
    backgroundColor: '{colors.light-accent}'
    textColor: '{colors.light-on-accent}'
    rounded: '{rounded.control}'
    padding: 0 13px
  button-primary-hover:
    backgroundColor: '{colors.light-accent-dark}'
    textColor: '{colors.light-on-accent}'
  button-secondary:
    backgroundColor: transparent
    textColor: '{colors.light-ink}'
    rounded: '{rounded.control}'
    padding: 0 12px
  numeric-field:
    backgroundColor: '{colors.light-panel}'
    textColor: '{colors.light-ink}'
    rounded: '{rounded.field}'
    typography: '{typography.measurement}'
  stage-navigation:
    backgroundColor: '{colors.light-panel}'
    textColor: '{colors.light-muted}'
    height: 48px
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
    backgroundColor: '{colors.light-panel}'
    textColor: '{colors.light-muted}'
    rounded: 3px
    padding: 0 12px
    height: 32px
  button-primary-dark:
    backgroundColor: '{colors.dark-accent}'
    textColor: '{colors.dark-on-accent}'
    rounded: '{rounded.control}'
    padding: 0 13px
  button-primary-hover-dark:
    backgroundColor: '{colors.dark-accent-dark}'
    textColor: '{colors.dark-on-accent}'
  button-secondary-dark:
    backgroundColor: transparent
    textColor: '{colors.dark-ink}'
    rounded: '{rounded.control}'
    padding: 0 12px
  numeric-field-dark:
    backgroundColor: '{colors.dark-panel}'
    textColor: '{colors.dark-ink}'
    rounded: '{rounded.field}'
    typography: '{typography.measurement}'
  stage-navigation-dark:
    backgroundColor: '{colors.dark-panel}'
    textColor: '{colors.dark-muted}'
    height: 48px
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
    backgroundColor: '{colors.dark-panel}'
    textColor: '{colors.dark-muted}'
    rounded: 3px
    padding: 0 12px
    height: 32px
---

# Design System: Board Studio v2

## Overview

**Creative North Star: "The Keyboard Lab"**

The Keyboard Lab is an instrument-like workspace for exploring and refining keyboard designs. Compact neutral panels frame a dominant drawing surface. Color identifies actions, geometry, and state; labels and measurements carry the detail.

The working character is calm, precise, and restrained: small controls, aligned values, modest corners, and short state transitions. Catalog, canvas, and inspector share one workspace, with optional properties organized into disclosures. These descriptions record the approved unified workspace now implemented locally.

**Key Characteristics:**

- Neutral shell with a separately colored slate canvas.
- Action blue, Board teal, Pad violet, and dashed key boundaries.
- Compact fields and rule-separated inspector disclosures.
- Fixed canvas controls and independently scrolling side panels.
- Light and dark palettes with a persisted System option.

The September 2026 cleanup preserves the unified workbench shell and its visual
identity. Shell ownership is split by responsibility: [base workbench styles](app/src/ui/workbench.css) own tokens, controls, canvas and geometry; [unified shell styles](app/src/ui/unified-workbench.css) own the page hierarchy and grid; [panel behavior styles](app/src/ui/workspace-panels.css) own dock modes, rails, drawers, and scrims; [inspector styles](app/src/ui/inspector.css) own inspector content rhythm and field roles; and [setup guide styles](app/src/ui/setup-guide.css) own the optional left-panel guide. The [footprint workspace styles](app/src/ui/library-workspace.css) remain scoped to the footprint library.
Exploratory review images are local artifacts; current source supplies the
implemented layout and token values.

## Colors

Role-led names describe the implemented palette. The frontmatter records separate
`light-*` and `dark-*` values; neither theme is a global inversion. Removing the
mode prefix from a color key gives its corresponding `--wb-` CSS token suffix.
Tokens in this document record the implementation; update the document and its
sidecar together when the implementation's palette changes.

### Primary

- **Action blue:** `accent` marks primary actions, active navigation, selected
  objects, and most inspector focus rings. `on-accent` is the paired foreground.
- **Action hover:** `accent-dark` is the existing hover token. Its name is legacy:
  it becomes lighter in the dark theme.
- **Selection surface:** `selection-surface` marks selected catalog and tree rows.
  Selection also has a border, heavier outline, or active navigation rule.

### Secondary

- **Secondary violet:** `accent-secondary` supplies secondary emphasis.
- **Auxiliary action color:** the legacy `teal` token is violet in light mode and
  turquoise in dark mode. Current consumers mix focus, add actions, readiness,
  and outline completion; it is not a safe universal semantic role.

### Tertiary

- **Board teal:** `geometry-board` outlines the board, `geometry-fill` fills it,
  and `geometry-graphic` draws footprint graphics.
- **Key blue:** `geometry-key` marks key boundaries and handles.
- **Pad violet:** `geometry-part` fills pads and draws copper traces and vias.
- **Geometry support:** `geometry-outline`, `geometry-body`, `geometry-label`,
  `geometry-text`, and `geometry-zone` keep boundaries, bodies, labels, and zones
  distinct. `geometry-fill-muted` supports inactive matrix cells.
- **Status colors:** `status-success`, `status-warning`, and `status-error` have
  separate surface tokens for filled backgrounds. Geometry keepouts and cutouts
  currently reuse the error color and retain dashed boundaries.

### Neutral

- **Slate canvas:** `canvas` is the drawing, footprint, and 3D preview field.
  `grid-small` and `grid-large` form its quiet measurement grid.
- **Shell surfaces:** `paper`, `panel`, and `raised` distinguish the application,
  permanent panels, and raised or hovered areas. `surface-soft` and
  `surface-strong` provide additional interaction surfaces.
- **Content and rules:** `ink`, `muted`, and `subtle` form the text hierarchy;
  `rule` and `rule-strong` separate panels and outline controls.

**The Paired Foreground Rule.** Use each filled action with its designated foreground; ordinary content colors are not interchangeable with on-accent colors.

**The Geometry Role Rule.** Keep canvas, board, key, pad, and selection roles separate from shell surfaces and status roles.

[AssemblyScene](app/src/ui/AssemblyScene.tsx) uses a transparent WebGL background
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
| Primary text | 15.87:1 | 13.50:1 |
| Muted text | 5.87:1 | 7.92:1 |
| Action | 5.94:1 | 8.36:1 |
| Action hover | 8.01:1 | 10.53:1 |
| Selection | 7.63:1 | 6.74:1 |
| Success | 6.31:1 | 6.59:1 |
| Warning | 6.80:1 | 8.79:1 |
| Error | 7.79:1 | 6.45:1 |

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
fallbacks. Source provenance and checksums live in `app/src/assets/fonts/README.md`;
SIL OFL licenses ship in `app/public/licenses/fonts/`.

- Preview titles use 20px and selected inspector titles use 23px.
- Section headings, actions, and control values use 13px; field labels and
  supporting text use 12px, with measurements in Source Code Pro.
- Pane and menu type roles stay fixed across viewport widths; browser zoom
  controls the overall scale.

**The Measurement Rule.** Keep values and units aligned, use tabular numerals,
and keep precise values available alongside direct manipulation.

## Layout

The application owns the viewport height (100dvh). The 48px header, 44px
command toolbar, and 34px footer frame independently scrolling navigator,
canvas, and right inspector panels. At 1440px and above these become 60px,
64px, and 60px. The center expands to absorb available width.

| Width | Implemented arrangement |
| --- | --- |
| Above 1150px | Resizable navigator and inspector, flexible center; default widths remain clamp-based |
| 981–1150px | Default 220px navigator and 300px inspector; saved widths are bounded to preserve the canvas |
| 821–980px | Full-width center with navigator drawer and 300px inspector |
| 820px and below | Full-width workspace with navigator and inspector drawers |

The desktop project header is 48px and the command toolbar is 44px. At 1440px
they become 60px and 64px; the footer becomes 60px. At 560px and below the
compact navigation remains label-led. The navigator drawer is capped at 290px
and the inspector at 370px, each bounded by 92vw, with a scrim.

Both desktop panels support pinned, collapsed, and opt-in auto-hide modes.
Widths (navigator 200–420px, inspector 280–480px) and modes persist locally.
Collapsed panels release their full column and reopen from narrow edge controls.
Auto-hide leaves the same reveal control and opens the panel over the canvas;
pointer presence, keyboard focus, and resizing prevent dismissal. Reveals use
a short slide, removed under reduced motion. Narrow screens use explicit drawers
opened from the header instead of hover behavior. Resizing supports pointer
dragging and arrow keys. Add opens a create view within the Objects panel.

The optional setup guide occupies the left panel alongside Objects navigation.
Its Guide and Objects controls are native buttons with a 44px compact target;
the current-step list is one bounded scrolling region and uses the same panel
surface, rules, typography, and selection tokens as Objects. SetupGuide uses
`is-current` and `is-ready` step states, with marker and accent treatment rather
than nested cards. On narrow widths it follows the same left drawer and scrim
behavior as Objects.

The guide opens automatically after a new project is saved successfully. Its
steps are Project & hardware, Layout & assemblies, Controller & wiring, optional
Case, and Review & export. Skip guide, Objects, and the Project menu provide
explicit exit/reentry paths. The open state and selected step are browser
preferences scoped to the project; completion comes from the current document,
selected board, wiring, and geometry. Steps remain freely navigable. Existing
projects retain the ordinary workbench and each exporter remains authoritative.

Recurring spacing uses compact 4px gaps, 8px inline separation, 12px field gaps,
16px inspector insets, and 20px section endings. These are extracted repeated
values rather than an enforced global spacing scale. Inspectors use 18px side
insets at 700px and below. Footprint workspace padding changes from 35px 38px
27px to 24px 18px 20px at that breakpoint.

**The Stationary Controls Rule.** Scope and snap controls remain anchored to the workspace while world geometry pans and zooms.

The Case action bar owns geometry readiness and its next action: configure,
generate, wait/cancel, retry, review errors, or export. Export requires current
generation, preview, committed scene, and resolved assembly revisions on the
configured board. Mechanical errors block export; warnings remain reviewable
without blocking. The canvas labels only the geometry currently displayed.
Configuration metadata does not imply generated-solid or manufacturing readiness.

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

Layout, PCB, Parts, and assembly previews share a floating Layers pill in the
lower-right corner. It starts collapsed. Expanding the bounded list never changes
the drawing dimensions or camera. Below 640px of drawing width it has a Close
control and 44px targets; Escape closes it and returns focus to the trigger.
Availability is separate from visibility preference: generated solids show
Not generated, while unavailable component assets show Missing model. Preferences
remain view-only, survive generation, and assembly preferences are scoped to the
project and board. Long labels wrap; lists scroll inside.

Assembly display and assembled/exploded/section controls occupy a wrapping rail
above the drawing. Camera controls remain outside the Layers pill.
Compact Case settings has a visible label, and compact generation, layer, and
settings controls use 44px touch targets. Passive canvas hints do not intercept
geometry selection or gasket dragging.

## Elevation & Depth

Persistent panels are flat and separated by fine rules and surface tone. Menus,
component flyouts, and mobile drawers use offset shadows to establish their
position above the workspace. Floating canvas hints and 3D controls have smaller
shadows. Save/live dots use the success foreground and have no halo in the unified shell.

- Project menu: `0 8px 24px rgb(0 0 0 / 20%)`.
- Component flyout: `0 8px 24px rgb(0 0 0 / 24%)`.
- Mobile drawer: `0 14px 30px rgb(0 0 0 / 18%)`.
- Canvas hint: `0 4px 14px #24373312`.

**The Structural Shadow Rule.** Use tonal surfaces and fine rules for persistent panels; reserve offset shadows for transient overlays and floating controls.

## Shapes

Unified inspector fields and action buttons use 4px corners; command buttons
use 3px, menus 5px. Persistent workspace panels remain rectangular. Selection
modes live in a labeled command menu; the earlier rounded floating scope dock
is superseded.

Thin rules organize the dense controls. Selection makes the relevant boundary
stronger rather than changing the object's silhouette. Inline SVG icons use
consistent outline strokes; geometry uses actual editable and compiled shapes.

## Components

### Actions

Primary actions use Action blue with the paired foreground, 4px corners,
13px horizontal padding, and a 34px minimum height. The Parts placement action
fills the inspector width and has a 38px minimum height. Hover updates fill and
border over 160ms. Secondary actions are outlined, transparent, and more compact
(33px minimum height, 12px horizontal padding).

Keyboard focus is generally a 2px ring with 2px offset. The global ring uses the
legacy auxiliary token, while inspector controls use Action blue. Disabled
buttons are dimmed, commonly to 0.42 opacity; specific controls vary.

### Fields

Fields combine a label, editable value, and adjacent unit. Numeric fields use a
1px outline, 4px corners, and a 34px inspector minimum height (42px at 1440px). Ordinary name fields are 32px;
generator text fields use 7px 8px padding. Focus changes the field border and/or
adds a visible ring. Error text remains beside the operation that produced it.

### Stage navigation and preview switching

Design and Parts tabs use text labels. Export is a labeled, icon-only action at
the upper right beside Undo/Redo; Project, history, and Export use quiet buttons
with hover and keyboard-focus feedback rather than permanent outlined boxes.
Layout, PCB, and Case are linked object-tree branches. Active tabs use a 2px action-colored
bottom rule, stronger text, and weight 700. The Parts preview uses two small
outlined buttons with pressed state, switching between 2D footprint and 3D model.

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
supplies missing key dimensions. Keys and Components have independent visibility. Layer swatches distinguish
Action blue keys, amber components, neutral keycaps, violet footprints, and
teal boards using the existing theme tokens.
Selection types have direct icon shortcuts. Stagger, Splay, and Origin activate
explicit canvas tools; handles remain hidden outside those tools. Origin snapping
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

### Motion and browser surfaces

Matrix hover changes fill and stroke over 120ms; primary actions transition over
160ms; mobile drawers move over 200ms with ease-out. Reduced-motion preference
shortens transitions and animations to 0.01ms. Inspector carets, scrollbars, text
selection, and focus rings use theme tokens. This is short state feedback, not
an entrance-animation system.

## Do's and Don'ts

### Do

- Do pair light or dark role tokens consistently within a surface.
- Do retain distinct board, key, pad, and selection treatments.
- Do keep measurement units adjacent to editable values.
- Do give each panel its own bounded scrolling region.
- Do preserve native disclosure controls, keyboard focus, and reduced-motion behavior.

### Don't

- Don't treat the original app's design document as v2's palette.
- Don't substitute shade or hue names for a control's actual role.
- Don't spread geometry colors across unrelated panel backgrounds.
- Don't copy the documented contrast and status-indicator defects into new components.
- Don't present a saved screenshot or this document as proof of current accessibility compliance.

### Workbench typography

Keep pane and menu roles stable across viewport widths: 23px inspector titles,
13px semibold section headings and actions, 13px control values, and 12px field
labels and supporting copy. This dense editing interface uses a 1.45 UI line
height and 1.55 for help text in narrow panes. Use the bundled Source Sans 3 and Source Code Pro font
stacks; use monospace for measurements and code, not ordinary labels. Tree
references do not shrink to make room for secondary descriptions; truncated
names expose their full text on hover. Browser zoom remains available.

### Workbench motion

Command options reveal from the pill's attached edge over 180ms. Project and
panel menus use a shorter crop with the same decelerating curve. Disclosure
chevrons turn with their state; hover and selection colors settle over 120ms.
Side panes enter over 220ms and exit over 120ms, remaining inert when hidden.
Menu dismissal and canvas geometry changes remain immediate. No loops,
page-load choreography, layout-size animation, or animation dependency is used.

Reduced motion removes pane travel and chevron transitions, substitutes a brief
80ms opacity change for menu reveals, and retains color feedback.

### Workbench controls and review

Selection uses one labeled selector alongside Transform, Align, and Snap. The
2D/3D switch occupies a separate control so view changes are distinct from editing. Rows/Columns tree grouping belongs in
Objects options. Panel menu actions occupy full-width rows, and Escape closes
the menu without changing the canvas selection.

Compact headers retain the active board name and a focusable local-save status.
The status follows actual storage writes; a failed write must not show as saved.
Undo, Redo, and Export remain available as labeled Project menu actions when
the compact header hides their desktop buttons.

Findings name their severity and affected target, consolidate matching board and
outline reports, and offer navigation when the target still exists. Corner-fit
warnings describe the resulting smaller corners without making fabrication
claims. Unresolved targets retain the diagnostic without a dead action.

Fit board and Fit selection sit beside zoom. Fitting includes transformed part
and keycap bounds, reserves room for canvas controls, and uses empty default
bounds only when no geometry exists. Selecting a half in the tree selects its
matrices and components for inspection with Fit selection.


### Wiring and physical assemblies

The PCB inspector identifies the selected part and its named terminal assignments.
Key switches inherit wiring from the board plan; the Wiring section owns controller
selection, explicit matrix/direct mode, automatic resolution, pin locks and manual
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
its physical X/Z placement. Construction dimensions may be linked; openings, mounts,
and battery space remain local. PCB thickness remains authoritative. Case geometry
is retained across electrical-only edits only when its exact physical dependency
signature and project/instance identity still match.
The Case preview uses that instance's reflected parts and outline together. Layout
2D and 3D continue to show the canonical PCB design.

ZMK handoffs target v0.3.0 and contain editable source, pin assignments, and build
configuration. Source generation and device-tree syntax checks do not substitute
for a complete target firmware build or hardware verification.
