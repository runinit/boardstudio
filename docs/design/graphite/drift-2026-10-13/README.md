# Graphite mockup → implementation drift

Review date: 2026-10-13. **Visual acceptance remains open.**

[Open the ten-screen comparison gallery](comparisons.md) · [Mobile overview](mobile-overview.jpg)

The palette and single-header/docked-panel shell are close to the chosen
Graphite direction. The largest remaining differences are page composition,
contextual commands, information hierarchy, and older controls that bypass the
shared styling. Another blanket increase in typography or spacing would miss
these causes.

This is a solo comparison, following the user's instruction to work solo.
The visual/source assessment was recorded before detector output. It is a
fidelity review, not a new design proposal or a behavioral acceptance run.

## Evidence and comparison conditions

- Authority: [approved Layout](../layout-approved.png) and the
  [nine-page Graphite suite](../../../../.impeccable/mocks/redesign-2026-10-12/graphite-suite/README.md),
  interpreted with that suite's correction notes and subsequent user decisions.
- Current frontend: `http://127.0.0.1:8182/`, dark system appearance;
  1586 × 992 desktop, 1024 × 768 intermediate, and 390 × 844 mobile.
- Isolated Chromium contexts; real Corne demo copies and a new split/wired
  setup project. User browser storage and saved projects were not changed.
- Keycaps was checked both unconfigured and with real Cherry matrix profiles,
  graphite colors, and Fit Keycaps. The real 3D renderer and generated Case stack
  were exercised; no illustrative geometry was substituted.
- The first capture batch contains 36 screenshots across the ten surfaces and
  supplementary states. No horizontal page overflow or uncaught application
  error was recorded in that batch. Mobile has no approved generated counterpart;
  those images verify adaptation, not pixel equivalence to a mobile mockup.
- [Measurements and run metadata](evidence.json). Browser/native tooling was
  unavailable; an isolated headless Chromium fallback was used. Software-rendered
  scene lighting does not establish fidelity to image-generated materials.

## What now matches

| Shared region | Approved Layout, approximate | Current, measured |
| --- | --- | --- |
| Header | 64px | 64px |
| Footer | 52px | 52px |
| Objects dock | 289px | 285.47px |
| Inspector dock | 313px | 317.19px |

These dimensions are already close. The dark surfaces, periwinkle selection,
single header, docked panes and solid toolbar backing are recognizable Graphite.
Layout's frequent transforms are grouped; Keymap has dark key faces; Export's
Current design / Portable project rail is now stacked and visible in the first
desktop viewport. Those earlier gaps should not be reported as still unfixed.

## Highest-impact shared drift

### P1 — Passive borders and unrelated settings compete with the task

Case's mirrored-pair rows retain conspicuous native fieldset boxes. Shared
viewer styles apply borders to buttons, canvas and details indiscriminately,
so several controls receive both a group frame and individual frames. Parts
and Keycaps mix these older treatments with newer shared controls.

Keep the fieldsets, labels, mounted disclosures and native events. Give passive
groups spacing and a restrained separator; reserve a clear edge for editable
fields, selected controls and keyboard focus. Primary transforms/dimensions
should precede mirror-management lists, definition metadata and specialist
settings. This is targeted shared-control adoption, not removal of every border.

Source: [shared viewer rules](../../../../web/assets/m1.css),
`.m1-shared-viewer` around lines 1073–1082; [mirror controls](../../../../web/crates/case/src/mechanical_settings.rs),
`MechanicalPairControls` and `.m1-mechanical-pair-row` around lines 1554 and 1635.
The latter class currently has no matching component styling in the two stylesheets.

### P1 — Contextual command hierarchy still differs materially

Layout's selected-column strip is wider and shallower than the approved strip
(approximately 775 × 52 versus 672 × 64 image pixels). Selection scope, density
controls and a detached numeric field compete with the frequent edit actions.
PCB leads with Snap / Part / overflow rather than making the selected controller's
placement operations immediately recognizable. Keycaps provides three text-only
Inspector focus shortcuts instead of the approved icon/label/selected-state system.

**Keymap's contextual toolbar exists in Rust but is hidden by CSS.**
`web/tailwind.css:861` sets `.m1-design-canvas-toolbar` to `display: none` whenever
the editor contains `.m1-keymap-panel`. It hides Assign / Macros / Encoders as well
as the older toolbar content. Restoring and composing that toolbar needs no new
command API.

Source: [toolbar visibility](../../../../web/tailwind.css),
[contextual actions](../../../../web/src/presentation.rs), and
[shared focus shortcuts](../../../../web/src/presentation/workspace_navigation.rs).
Some desired direct actions go beyond the existing focus shortcuts; distinguish
their interaction work from icon and spacing corrections.

### P1 — Selection identity and geometry labels lose their hierarchy

Layout and PCB draw long, often rotated internal part identifiers across the
geometry. Objects gives secondary descriptions enough space to truncate even
short references. In the Inspector, generic Inspect, workspace headings and
selected-item headings repeat identity. Layout's Offset X shows raw float
precision that is clipped by the field.

Use the selected object as the primary title; demote technical metadata. Show
short, readable references at normal editing zoom and expose full identities
through existing detail affordances. Present measured values with concise display
precision without reducing stored precision or interfering with active drafts.

### P1 — The Objects header action is implemented in the wrong position

The approved Layout puts Add object beside Objects. The live UI still uses a
full-width primary row beneath it. This is not an outstanding API permission:
`ObjectsPanel.header_action` already exists and is wired by Layout. In
`panels.rs`, however, `.m1-panel-header-action` is rendered **after** the closing
`header` element. It therefore occupies a separate row and retains the existing
full-width `.m1-layout-add-object` styling.

Place the existing slot within the header composition and give that instance a
compact action treatment. Preserve its current menu owner and event handlers.

Source: [panel slot](../../../../web/crates/ui-shared/src/panels.rs), around
lines 462–493; [Layout wiring](../../../../web/src/presentation.rs), around
line 7659; [action styling](../../../../web/assets/m1.css), around lines 285–289.

## Page-by-page comparison

Each link opens a full-resolution pair: approved on the left, current on the right.

| Surface / priority | Remaining drift | Evidence |
| --- | --- | --- |
| **Layout — P1** | Full-width Add object; wider action strip; detached numeric editor; dense technical geometry labels; clipped numeric precision; poor hierarchy between tree names and descriptions. Frequent Inspector transforms and dock sizing have improved. | [Comparison](compare-Layout.jpg) · [current](Layout.png) |
| **PCB — P1** | A long flat component list replaces the mock's useful Keys / Diodes / Controllers grouping. Controller identity is subordinate to generic Inspect / Wiring headings. Used/free-pin text and tall assignment rows dominate the Inspector; the toolbar is not presented as controller-specific placement controls. | [Comparison](compare-PCB.jpg) · [current](PCB-settled.png) |
| **Keymap — P1** | Contextual toolbar is completely hidden. Full-width Add layer and persistent rename/help fields consume the left dock. Keys / Macros / Encoders remain in the right pane instead of the mock's compact navigation hierarchy. Dark semantic key faces are now correct. | [Comparison](compare-Keymap.jpg) · [current](Keymap.png) |
| **Keycaps — P1** | Even after real profiles/colors are configured and Fit Keycaps is used, the model occupies a small portion of a shallow viewer. A large lower canvas area is unused. Profile / Legend / Color, fit/view settings, Layers and 2D/3D controls form disconnected strips. Inspector controls remain vertically loose and lack the selected-key hierarchy. Viewer sizing and camera framing need separate verification. | [Comparison](compare-Keycaps.jpg) · [configured](Keycaps-configured.png) · [unconfigured demo](Keycaps.png) |
| **Case — P1** | Multiple control groups and update/export chrome crowd the top of the scene. Boxed mirrored-feature management appears before primary construction dimensions. The real generated stack and exploded mode are available; the screenshot must not be described as a missing CAD model. The initial comparison selects assembly context, not the mock's authored tray, so body-specific Inspector fidelity remains a separate state to verify. | [Comparison](compare-Case.jpg) · [assembled](Case.png) · [exploded](Case-exploded.png) |
| **Parts — P1** | Raw native file chooser is exposed. The generic Inspector leads with saved-assembly management and definition metadata while the selected canvas is an MX Hotswap key assembly. Mechanical-fit chrome occupies the top of the canvas. Dimension callouts and the explanatory layer legend are absent from the reference composition; adding real measured overlays is presentation capability work, not merely CSS. | [Comparison](compare-Parts.jpg) · [current](Parts.png) |
| **Export — P2** | Approximately 48px page insets and a large gap before Design files weaken the compact handoff list. File identities are reduced to tiny markers; available rows all use equally strong primary actions. Current design omits the mock's concise layout/wiring/case summary. The portable-project rail is fixed and visible. Preserve actual formats, provider readiness and truthful downstream-review language. | [Comparison](compare-Export.jpg) · [current](Export.png) |
| **Project browser — P2** | Approximately 48px outer insets, a large Setup guide row and a full-width import row interrupt the preview collection. Rename/action placement and hierarchy differ from the selected-project rail in the mock. The rail and real local previews now exist. One saved project naturally leaves space; that fixture difference is not itself a layout failure. | [Comparison](compare-Browser.jpg) · [current](Browser.png) · [empty state](Browser-empty.png) |
| **Project onboarding — P2** | Three regions exist, but the stage heading uses 20px rather than the mock's roughly 32px role. Guide and form surfaces blend together; preview title and form are vertically misaligned; guide metadata introduces excess empty space. The header retains editor project/save chrome rather than the simpler setup composition. Schematic rectangles before real geometry are an appropriate capability limit. | [Comparison](compare-Setup-project.jpg) · [current](Setup-project.png) |
| **Layout onboarding — P1** | The approved inline Rows / Columns / Pitch / Key assembly form, mirrored preview and Create layout & open editor handoff have not been implemented. The guide offers Add key matrix and continuation buttons beside an empty editor canvas. Add key matrix leaves the guide and opens the existing Objects matrix editor. This is a workflow/composition gap, not a spacing adjustment. The setup canvas toolbar also wraps onto a second line. | [Comparison](compare-Setup-layout.jpg) · [guide](Setup-layout.png) · [after Add key matrix](Setup-layout-matrix.png) |

## Differences that should not be “fixed” literally

- Workspace navigation was intentionally moved from the header to the left dock
  after user feedback. Do not restore the mock's duplicate top navigation.
- Corne's keys, controller pins, revisions, contours and active-board scope differ
  from illustrative Aster 42. A single active PCB view is not proof that split
  rendering is missing. Unassigned key legends are actual document data.
- Browser previews are supported 2D schematics. Model-rendered thumbnails and a
  sorting feature would be additional capabilities, not a palette or CSS fix.
- Generated Parts pad layouts, the conflated Cherry (OEM) label, the Keymap
  layer-wide Transparent checkbox and misleading portable-save copy were already
  rejected in the suite notes. Do not reproduce them.
- Gradients, invented validated geometry, tables, floating Objects/Inspector
  panels and a second header are outside the chosen direction.
- **Case height/thickness drag handles remain explicitly excluded.**

## Recommended correction order

1. **One targeted shared-controls pass:** quiet passive fieldsets and viewer
   group frames; standardize buttons/tabs/disclosure spacing; preserve editable
   field edges and focus; fix the existing Objects header slot placement.
2. **Selection and command pass:** restore Keymap tools; compose the existing
   workspace actions with consistent icons, labels, selected/disabled states;
   prioritize object identity and primary edits; fix numeric and reference display.
3. **Scene composition:** make the actual Keycaps viewer fill its allotted area,
   verify fit/camera behavior, and consolidate Case/viewer controls. Use real
   generated assemblies and compare selected body states separately.
4. **Finish shaped onboarding interactions:** keep the matrix form and real
   preview within the guide, using the existing matrix creation owner, drafts,
   selection and undo. Preserve resumability and existing project topology semantics.
5. **Complete Parts, Export and browser composition:** move specialist metadata
   out of the primary path; compact handoff rows and page spacing; retain supported
   schematic previews unless richer thumbnails are separately scoped.

Acceptance should use these same reference dimensions and representative selected,
expanded and disabled states. Keep style fixes separate from newly implemented
interactions. Passing application tests or an empty style scan cannot establish
visual equivalence.

## Verification and limits

Application source was not changed for this review. Runtime interactions were
confined to isolated browser contexts. No new regression suite was run: this is
an observation/report task. Fresh screenshots and source inspection are the
primary evidence.

The CLI detector returned no findings for the selected RSX source directories.
Browser injection succeeded on Layout, PCB and Export, but detector reporting
failed in Dioxus's patched console because overlay DOM references form a circular
JSON structure. Its partial counts are **not accepted as verified findings**.
[CLI output](detector.json) and [browser output](browser-detector.json) are saved.
No user-visible overlay or expiring feedback link is offered. The temporary
detector server was stopped; the frontend preview is kept running.

The “Your app is being rebuilt” string in the HTML is an off-screen Dioxus toast,
not evidence that these captures show a stalled build. The screenshots demonstrate
the mounted application. This supersedes that particular stale preview note in
the earlier fidelity packet.

Questions skipped: the chosen direction, comparison target and exclusions are
already specified; this review does not require another visual-direction decision.
