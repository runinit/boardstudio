---
name: BoardStudio — The Keyboard Lab
description: A cards-on-a-desk workbench with semantic Theme roles and distinct geometry colors.
colors:
  original-light-background: '#E9EEF2'
  original-light-card: '#FFFFFF'
  original-light-primary: '#3858D6'
  original-light-primary-foreground: '#FFFFFF'
  original-dark-background: '#181B21'
  original-dark-card: '#242831'
  original-dark-primary: '#9AADFF'
  original-dark-primary-foreground: '#101725'
  shadcn-light-background: 'oklch(0.965 0 0)'
  shadcn-light-card: 'oklch(1 0 0)'
  shadcn-light-primary: 'oklch(0.205 0 0)'
  shadcn-light-primary-foreground: 'oklch(0.985 0 0)'
  shadcn-dark-background: 'oklch(0.18 0 0)'
  shadcn-dark-card: 'oklch(0.205 0 0)'
  shadcn-dark-primary: 'oklch(0.922 0 0)'
  shadcn-dark-primary-foreground: 'oklch(0.205 0 0)'
typography:
  original-sans: "'Source Sans 3', 'Segoe UI', sans-serif"
  original-mono: "'Source Code Pro', Consolas, monospace"
  shadcn-sans: "'Inter Variable', system-ui, sans-serif"
  shadcn-mono: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
---

# Design System: BoardStudio

## Overview

**Creative North Star: "The Keyboard Lab"**

The Keyboard Lab is a precise workspace for exploring and refining keyboard
designs. A neutral desk holds rounded cards around a dominant drawing surface;
color distinguishes actions, geometry and state. The working character is calm,
compact and restrained, with independently scrolling dock content and anchored
canvas controls.

Original and shadcn are two Themes, each with light and dark Appearances.
Both use the same shell and control metrics. The shipped Rust/Dioxus components
and [Tailwind source](web/tailwind.css) are the implementation reference;
the [theme contract](docs/plans/rust-ui-migration/theme-contract.md) explains
the design choices. Document defaults and shared finding outlines live in
[the base stylesheet](web/assets/workbench-base.css). Older Graphite
explorations are archived in [the reference packet](docs/design/graphite/approved-suite/).

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

The frontmatter gives a small set of actual CSS role values for both Themes
and Appearances, not an exhaustive palette. The authoritative values are in
[Tailwind source](web/tailwind.css); the
[theme contract](docs/plans/rust-ui-migration/theme-contract.md) records their
meaning and rationale. A Theme changes colors, fonts, radius, shadows and icon
stroke, not the shared shell metrics.

| Theme | Character | Sans / monospace | Radius / icon stroke |
| --- | --- | --- | --- |
| Original (default) | Slate desk, white or charcoal card, blue primary | Source Sans 3 / Source Code Pro | 10px / 1.5 |
| shadcn | Neutral oklch desk and cards, neutral primary | Inter / system monospace | 0.625rem / 2 |

Appearance is light, dark or system; system follows the browser's color-scheme
preference. The Settings/Theme menu offers separate Appearance and Theme
choices. The resolved Appearance and chosen Theme are applied to `<html>` as
`data-appearance` and `data-theme`. Browser preferences use
`boardstudio:v2:appearance` and `boardstudio:v2:workbench-theme`;
the former falls back to the legacy `boardstudio:v2:theme` until saved.
The 3D renderer receives Appearance, not Theme, so its default palettes are
shared between Themes. First-paint timing has not been verified.

### Primary

- **Primary action:** `primary` marks primary actions, paired with
  `primary-foreground`; hover treatments retain that pairing.
- **Selection surface:** `selected` and `selected-foreground` mark the header
  workspace and some catalog rows; shadcn aliases them to its quiet `accent`.
  The default Toggle Group uses a `background` fill for its active item. Focus
  keeps a Theme-colored ring. The separate `selection` and
  `selection-foreground` roles style text highlights (orange in shadcn), but
  some existing tree rows, setup controls and 2D geometry also use them. Input
  and Combobox text highlights use `primary` instead.

### Secondary

- **Secondary emphasis:** use semantic secondary or geometry roles rather than
  deriving an action color from a canvas color.

### Tertiary

- **Board teal:** `geometry-board` outlines the board, `geometry-fill` fills it,
  and `geometry-graphic` draws footprint graphics.
- **Key blue:** `geometry-key` marks key boundaries and handles.
- **Pad violet:** `geometry-part` fills pads and draws copper traces and vias.
- **Geometry support:** `geometry-outline`, `geometry-body`, `geometry-label`,
  `geometry-text`, and `geometry-zone` keep boundaries, bodies, labels, and zones
  distinct. `geometry-fill-muted` supports inactive matrix cells.
- **Keymap keys:** `keycap-face`, `keycap-ink`, and `keycap-rule` keep
  binding labels legible across Themes.
- **Status colors:** `success`, `warning`, and `destructive` have
  paired foreground and light/dark roles. Geometry keepouts and cutouts
  currently reuse the error color and retain dashed boundaries.

### Neutral

- **Neutral canvas:** `canvas` is the drawing, footprint, and 3D preview field.
  `grid-small` and `grid-large` form its quiet measurement grid.
- **Shell surfaces:** `background`, `card`, `muted`, and `accent`
  distinguish the desk, panels, quieter surfaces, and hovered areas.
- **Field and toolbar surfaces:** `field` distinguishes editable controls; `toolbar` and `toolbar-border` back contextual actions.
- **Content and rules:** `foreground` and `muted-foreground` form the text
  hierarchy; `border` and `border-strong` provide rules where needed, while
  `ring` marks focus. The main cards do not rely on outlines.

**The Shared Token Rule.** Tailwind utilities and BoardStudio controls consume the same semantic Theme roles. Keep workspace state and document edits out of visual primitives.

**The Paired Foreground Rule.** Use each filled action with its designated foreground; ordinary content colors are not interchangeable with on-accent colors.

**The Geometry Role Rule.** Keep canvas, board, key, pad, and selection roles separate from shell surfaces and status roles.

2D geometry and grid roles are Theme tokens. The 3D renderer's materials and
lighting are separate from those CSS values.

### Known implementation limitations

Token names alone do not guarantee contrast across composited states, geometry
or 3D materials. Measure actual foreground/background and focus pairs; do not
treat this document as evidence of accessibility compliance.

## Typography

Original uses bundled Source Sans 3 for UI text and Source Code Pro for
measurements and code. shadcn uses bundled Inter Variable and a system monospace
stack. Font files use `font-display: swap`; provenance and licenses are in
[the font record](web/assets/fonts/README.md). Body defaults are 16px / 1.45;
individual components choose their own hierarchy and compact sizes.

**The Measurement Rule.** Keep values and units together, align measurements
where practical, and keep precise values available alongside direct manipulation.

## Layout

The viewport is a desk with a 56px borderless header card, a flexible workspace
card and, in the normal editor, Objects and Inspector docks on either side.
The footer supplies workspace status and camera controls. The workspace center
takes the remaining width; dock bodies scroll independently.

### Header and navigation

The header contains BoardStudio identity, the Project menu, workspace navigation,
Search commands, local-save status, Undo/Redo, Settings/Theme and Export.
Layout, PCB, Keymap, Keycaps, Case and Parts are chosen **in the header**, not
in Objects; the current choice has `aria-current="page"` and a selected fill.
Export is a separate action and page. Project actions include opening, naming
and saving projects. Search opens the Command palette (Ctrl/Cmd+K).

At widths of 1250px and below, the Workbench menu contains workspace navigation,
Search, history, panel controls when applicable, Theme and Export; the Project
menu and save status remain in the header. The save label hides at 520px and
below; the status control remains. The brand text appears from 1600px. The
project browser hides workspace navigation until a project is open.

### Docks and narrow screens

At 980px and wider the Sidenav-based Objects and Inspector are in-flow cards.
Each can be pinned or collapsed to a 44px rail; opening the rail restores the
dock. Auto-hide is not offered: a saved legacy auto-hide mode is read as pinned.
Modes and widths persist per side in browser storage. Objects resizes between
200–420px and Inspector between 280–480px, further limited by available space.
Resize handles accept pointer drag, arrow keys, Shift+arrow, Home and End.

Below 980px the regular editor uses one bottom, non-modal Sheet with Objects
and Inspector tabs and a close control. Header menu controls open the requested
tab; unavailable panels have disabled tabs. The Sheet is `min(45dvh, 400px)`
high, with no backdrop, scroll lock or focus trap: the visible canvas remains
interactive. Inactive tab contents stay mounted but hidden and inert, preserving
their owners. The project library and Export do not display these docks. The
setup guide is a separate left-side overlay in place of the normal docks, not a
tab in the Sheet.

Project → Setup guide opens the guide, whose Project, Layout, Wiring and Case
sections remain freely navigable. Hide guide returns to the normal tree.
In Case, Full settings opens the existing mechanical settings in the guide's
left overlay; Back to guide returns without dismissing it. Guide state is a
project-scoped browser preference, while completion derives from the document.

Objects holds the active workspace's contextual tree and view controls; Layout
places Add beside its heading. Inspector presents the selected object's edits.
Visibility controls are view-only. Layout's Canvas visibility is in Objects,
while Snap stays in the canvas toolbar. PCB, Case and Parts have their own layer
or assembly visibility controls; Keymap layers are in the Keymap Objects list.

Layout keeps two stationary strips above its canvas: a scope strip and a
contextual action strip with Align and Transform. The strips are outside the
transformed drawing and may wrap their controls when space is constrained.
**The Stationary Controls Rule:** scope and snap controls stay anchored while
world geometry pans and zooms.

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

PCB, Case, and Parts keep their workspace-specific layer controls.
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

The header, dock Sidenavs and workspace center are rounded, borderless cards on
the `background` desk, using `card`/`sidenav` fills and soft Theme shadows.
The drawing surface is inset inside the workspace card. Dark Appearance relies
on the lighter card fill as well as shadow. Menus, popovers and the non-modal
Sheet sit above the workspace; specific workspace surfaces can still use rules.

## Shapes

Card corners use `rounded-xl` (`--radius + 4px`); common copied controls use
`rounded-md` (`--radius - 2px`). Secondary buttons and toggle groups use
filled surfaces, while ghost buttons are quiet until hover. Card, Input,
Input Group, Select trigger and Toggle Group copies remove their built-in
borders for the cards style, but retain focus rings.
The Theme supplies the radius and icon stroke; geometry preserves its own shapes.

## Components

### Actions

The copied `Button` offers primary, secondary, ghost and other variants. The
default primary uses `primary` and `primary-foreground`; secondary is filled,
ghost is transparent until hover. Shared sizes are 40px default, 36px compact
and 44px large/touch; coarse-pointer rules increase targets where applied.
Keyboard focus retains a Theme-colored ring and disabled shared buttons dim.

### Fields

Input, Input Group and Select triggers use `field`, Theme-derived corners and
focus rings instead of a built-in outline. A numeric field composes a label,
input, unit and error text; callers own parsing and commit behavior. Field errors
remain adjacent to the field. shadcn's light `field` is transparent and its dark
field is a subtle translucent fill, rather than Original's solid fill.

Short fixed lists use Select and searchable lists use Combobox; Toggle Group,
Toolbar, Dropdown Menu, Slider and Collapsible supply the corresponding shared
interaction patterns. The shell uses Toasts for transient action results;
persistent problems, readiness blockers and field errors remain inline.

### Workspace navigation and preview switching

Workspace navigation belongs to the header or its compact Workbench menu;
Objects remains contextual to the current workspace. Export is separate. Parts
preview and camera controls switch views without taking ownership of document
edits. Layout's view controls and footer zoom/fit remain anchored to the
workspace rather than to drawing geometry.

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

Catalog and tree rows use compact, selectable surfaces with names and
supporting detail. Selected objects use `selected` and `selected-foreground`;
`selection` and `selection-foreground` are reserved for text highlights.
Focus remains independently visible.

### Inspector disclosures

Keys, rows, and columns expose width and height sliders in quarter-unit steps
from 1u to 7u. Wide/Tall swaps the dimensions; a row or column applies the change
to all selected keys in one edit. Units use matrix pitch minus the configured
edge gap. Explicit cap dimensions stay linked across paired halves. Key Assembly
names the key definition selector; diode composition belongs to the assembly,
not a separate key checkbox.

The inspector begins with the selected object and its common edits. Optional
groups use shared Collapsible or Accordion controls as appropriate, revealing
settings in place. Draft-owning contents stay mounted when closed.

**The Disclosure Rule.** Separate inspector sections with quiet rules and reveal optional settings in place, preserving the selected object as the editing context.

### Workspace grouping and layer names

The header owns project actions and workspace navigation buttons. The Objects dock holds
the active workspace's object tree; contextual tools sit beside the canvas, and
the Inspector begins with the selected object and its common edits. Group
specialist controls by task and use clearly named, single-level disclosures for
optional settings.

Use distinct names for distinct meanings: Canvas visibility, PCB layers,
Keymap layers, Case assembly visibility, and Parts preview layers. In Keymap, the
layer list is **Keymap layers** and the Inspector includes the **Keymap editor**.
Keep visibility controls view-only, and preserve each
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
supplies missing key dimensions. A Relations reference hover or focus marks its parts with a dashed violet outline, never the selection surface or accent stroke, and clears on leave, blur, selection change or removal. The Layout canvas-visibility control has independent Keys, Components, Keycaps, Footprints,
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

### Motion and browser surfaces

Shared controls provide brief state feedback, and menus and other transient
surfaces use component-specific transitions. Preserve reduced-motion handling
where present and keep canvas interaction immediate.

## Do's and Don'ts

### Do

- Do pair light or dark role tokens consistently within a surface.
- Do retain distinct board, key, pad, and selection treatments.
- Do keep measurement units adjacent to editable values.
- Do give each panel its own bounded scrolling region.
- Do preserve disclosure behavior and keyboard focus, and add reduced-motion handling to any new movement.

### Don't

- Don't treat the original (v1) app's design document as BoardStudio's palette.
- Don't substitute shade or hue names for a control's actual role.
- Don't spread geometry colors across unrelated panel backgrounds.
- Don't assume semantic tokens alone guarantee contrast in every state.
- Don't present a saved screenshot or this document as proof of current accessibility compliance.

### Workbench typography and motion

Use the active Theme's sans and monospace stacks. Keep values legible, preserve
browser zoom, and avoid making canvas interaction depend on animation.

### Workbench controls and review

Layout's scope and contextual action strips use Toolbar, Toggle Group and
Dropdown Menu patterns. Scope selection, Align, Transform and Snap stay with
the canvas; workspace switching remains in the header. Groups can wrap in
constrained space, and coarse-pointer rules enlarge the strip controls.
Domain-specific trees and visibility owners stay with each workspace.

Shared controls live in `web/crates/ui-shared/src/ui/`, with token-based
styles in `web/tailwind.css`. Use `Button` variants for form and task actions;
pass the submit type explicitly for submission. Keep bespoke canvas, tree and
navigation affordances in their owning workspace.

Copied controls use Dioxus primitives for interactive behavior, with their
keyboard and ARIA requirements recorded in the
[accessibility floor](docs/plans/rust-ui-migration/accessibility-floor.md).
Collapsible sections that own drafts must keep those children mounted while
closed. Pending actions control whether a confirmation can be dismissed.
Browser file pickers remain native.

Compact headers retain the project menu and a focusable local-save status.
The status follows actual storage writes; a failed write must not show as saved.
Undo, Redo and Export are available in the compact Workbench menu.

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
