# Graphite visual fidelity — corrective work

Visual acceptance is reopened following user feedback. The implementation has
working migration behavior and passing application checks; it is not a faithful
completion of the approved visual design. This supersedes the previous statement
that the nine-screen comparison found no material defect. The Impeccable receipt
limitation is separate and does not explain or excuse these design gaps.

## Evidence and comparison conditions

Fresh preview captures use the approved reference size, 1586 × 992, dark system
appearance, and an isolated browser context with the real Corne demo. No saved
user project or appearance preference was changed. Browser control through the
in-app tool was unavailable, so a headless Chromium capture was used. Current
measurements and the capture script are in `.scratch/graphite-style-gaps/`.
Earlier selected-state captures supplement this fresh unselected fixture.
Geometry and bindings vary by fixture; the comparison concerns chrome,
hierarchy, layout and control presentation rather than identical key counts.

Current captures: [Layout](fidelity-current-Layout.png),
[Keymap](fidelity-current-Keymap.png), [Keycaps](fidelity-current-Keycaps.png),
[Export](fidelity-current-Export.png), [browser](fidelity-current-Browser.png).
References: [Layout](layout-approved.png) and [the approved suite](approved-suite/).

## Material gaps

| Priority / area | Current gap | Corrective target |
| --- | --- | --- |
| 1 — shared type and chrome | A 56px header, 40px footer and 13px base type produce substantially smaller chrome and labels than the reference's roughly 64px header and 52px footer. Generic controls do not establish the mockup's hierarchy. | Measure shared type, control, icon and spacing roles against the reference. Extend semantic tokens once, then apply consistently through existing shared controls. Preserve 44px touch targets and narrow-screen behavior. |
| 1 — Layout Inspector | Generic Inspect heading, repeated selection/property headings, small fields and a long stack of reset/origin/destructive controls compete with the selected object's primary transforms. | Give selection identity one strong heading; place frequent transforms first; use compact aligned fields and native named disclosures for specialist settings. Retain mounted drafts and existing edit owners. |
| 1 — toolbars | Layout's long command strip and other workspaces' three small generic buttons do not reproduce the reference's clear icon, label, selected-state and separator system. | Build one shared action-group treatment with existing icons and controls, then expose the actual selection's frequent actions. Preserve existing command routes; distinguish presentation gaps from any new interaction work. |
| 2 — Keymap canvas and navigation | Light key faces, tiny repeated Unassigned labels/technical references, full-width Add layer and rename fields, and duplicated mode controls depart substantially from the intended semantic keymap workbench. | Separate semantic binding presentation from physical keycap appearance. Use readable binding labels and selected state, compact layer rows and correctly placed modes. Keep behavior choices and keyboard navigation available; do not alter saved keycap colors or invent assignments. |
| 2 — Keycaps / Case scene presentation | Existing captures do not demonstrate the reference's physical assembly composition or selected-part context. A PCB-only Case fixture cannot verify an assembled-case design. | Verify the existing renderer with representative authored bodies and keycap settings, and compose its actual supported views. Do not replace real geometry with an illustration or treat a missing fixture body as a styling defect. Case height/thickness handles remain excluded. |
| 2 — Export | The content is inset and vertically loose, status is reduced to small text/dots, actions are generic, and the portable-project section starts at Y=1099 in a 992px desktop viewport. It is not directly below Current design as in the mockup. | Compose a full-width handoff list with clear file identity, readable readiness badges and actions, alongside a stacked Current design / Portable project rail. Retain all actual export providers and blockers; keep portable download visible in the first desktop viewport. |
| 3 — project browser | Small schematic cards, a wide Back to editor button, inline current-project editing, duplicated creation entry points and a raw file input dominate the page. The reference is organized around large project previews and a selected-project rail. | Recompose header actions, project cards and the selected-project handoff. Use real supported previews, existing Open/rename/delete/import ownership and an accessible file-input trigger. Do not add a second storage API or imply unsupported 3D thumbnails already exist. |
| 3 — PCB, Parts and onboarding | Docks and contextual inspectors are in place. Current PCB and Parts captures select different real items than the mockups; the setup capture uses a one-board project rather than the mock's split wired assembly. | Compare composition separately from fixture state. Keep selection and project data real, and record supported-state gaps such as PCB pin grouping explicitly. |

## Corrective batch 1 — shared styling and Layout

The shared visual rhythm follow-up now uses 12 / 14 / 16 / 18 / 20 / 24 / 32px
type roles, a 22px brand mark, and 4 / 8 / 12 / 16 / 24 / 32px spacing tokens.
The desktop shell is 64px high with a 52px footer. Secondary shared controls
use a soft fill without a passive outline, panel dividers and toolbar borders
are subdued, and editable fields retain a higher-contrast edge and focus ring.
Page headings stay at 32px across viewports, and the Setup stage heading uses a
20px context role. CAD geometry, selected/error states and keyboard focus keep
their distinct visual treatment.

Selection identity replaces the generic Inspector heading while native
close/options controls remain accessible. Column Stagger, Offset, Splay and
Origin mode are grouped in Transform. Origin coordinates and scope, and
destructive actions, live in separate native disclosures with mounted children.
The adaptive toolbar presents the selected object's actions directly, with
selection scope and specialist overflow; redundant canvas tool-selection
buttons are removed, while editable dimensions and drag handles remain.

Evidence: [desktop](fidelity-1-Layout.png), [1024px](fidelity-1-Layout-1024.png),
[390px Inspector](fidelity-1-Layout-390.png). The approved selected-column mockup
remains the authority; the live Corne fixture has different real geometry.
These captures predate the shared rhythm follow-up and remain useful for
behavior/layout context only. Fresh comparison is pending: the independent
preview currently serves Dioxus's “Your app is being rebuilt” screen, so no
post-pass screenshot is being treated as visual evidence.
The disclosure regression failed on the original expanded specialist section,
then passed after the change. All 174 Layout browser tests pass, including draft,
selection, undo/redo and menu behavior. The responsive journeys pass across
PCB, Case, Keymap, Keycaps, Parts, Export, browser and setup at 1359/1024/390px.
The preceding batch's screenshots use that batch's emitted Tailwind CSS. This
follow-up regenerated the shipped Tailwind asset with pinned v4.1.5 and left the
existing Dioxus server, Tailwind watcher, and proxy processes running.

Visual acceptance remains open. Layout's technical footprint/reference rendering,
Objects header/action placement and detailed canvas measurement composition still
need closer fidelity. Keymap, scene presentation, Export, browser and onboarding
composition have since been implemented as recorded in the
[continuation packet](batches-3-6.md); they still need a final matched-state visual
review. Passing behavior tests does not close visual gaps, and fixture differences
still limit claims about selected PCB wiring and authored Case geometry.

## Corrective batch 2 — PCB assignments and onboarding composition

The PCB wiring inspector now presents resolved assignments in an expanded,
native `BoardDisclosure` with a visible count. Matrix assignments are grouped
under separate collapsed Row pins, Column pins and, when present, Peripheral
pins disclosures; direct-mode assignments remain flat. Grouping uses the
existing assignment IDs, pin controls and edit owner. Other inspector
disclosures were already using the shared control, so the pass did not add a
second accordion pattern.

Onboarding now uses a dedicated stage navigation and form layout beside the
actual workspace viewer. At desktop, the navigation and stage form share the
left dock while the preview remains on the right. The setup-only Project stage
suppresses the editor's geometry toolbar, and setup hides the unrelated editor
footer. At 1024px and 390px the form becomes one column; the preview remains
below it on narrow screens. The Project and Layout stages were captured at
1586×992, 1024×768 and 390×844 with no horizontal overflow. The page heading
and schematic labels use the new shared `--wb-type-page` and body roles.

The captures use the real `Graphite verification` project, which is configured
as one keyboard with an existing 18-key layout. This is different from the
mock's split, wired Aster42. The comparison therefore covers layout, color and
control hierarchy; the schematic preview correctly describes supported
topology and does not invent board outlines before the layout exists. The app
shell retains its current-project selector and save status in its single header.

Evidence: [Project stage, desktop](fidelity-2-Setup-project.png),
[Project, 1024px](fidelity-2-Setup-project-1024.png),
[Project, 390px](fidelity-2-Setup-project-390.png),
[Layout stage, desktop](fidelity-2-Setup-layout.png),
[Layout, 1024px](fidelity-2-Setup-layout-1024.png), and
[Layout, 390px](fidelity-2-Setup-layout-390.png). The pinned Tailwind 4.1.5
output was regenerated without restarting the Dioxus server or its watcher.

Visual acceptance remains open. PCB's approved controller/pin-group state and
Parts' approved assembly selection are not present in this project's real
fixture; Case geometry is also absent. Those are fixture/content differences,
not permission to synthesize model data. The PCB group behavior is covered by
its assignment projection test, but this fixture cannot provide a matching
controller state for a visual capture. The Case height/thickness handle
exclusion remains in force.

## Corrective sequence and exit criteria

1. Shared visual roles and Layout: retain the chosen Graphite direction, one
   header and docked panels; rebuild the hierarchy through existing components.
2. Keymap, Keycaps, PCB, Case and Parts: apply that shared system, then close
   workspace-specific composition and contextual-control gaps.
3. Export, browser and onboarding: implement their distinct page compositions,
   rather than stretching the editor's generic form styling across them.

Each coherent batch needs matched-viewport before/after region comparisons and
real selected, expanded and disabled states. Use desktop, intermediate and
390px captures together, then one confirmation after batched fixes. Tests prove
behavior preservation; they are not a visual acceptance result. Record visible
deviations explicitly. No new design alternatives, duplicate top navigation,
floating panels, tables or Case height/thickness handles are introduced.
