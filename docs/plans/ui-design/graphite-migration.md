# Graphite frontend migration — shaped brief

Date: 2026-10-12. Status: brief approved by the user on 2026-10-12; Batch 1 implementation and evidence are in docs/design/graphite/batch-1.md.

## Confirmed planning request

The user's revised request fixes the visual direction and implementation stack. This brief incorporates those instructions without reopening design alternatives:

```text
$impeccable shape the Graphite implementation across all mockups in
.impeccable/mocks/redesign-2026-10-12/graphite-suite/.

Keep the chosen visual direction fixed. Produce a staged migration
plan covering shared components, existing behavior, new interactions,
and verification. Start with shared tokens, shell and Layout.
Do not generate new design alternatives.

Use the existing Rust/Dioxus frontend, pinned Dioxus Components
primitives, and shared BoardStudio controls.

Use Tailwind CSS through the existing semantic --wb-* tokens.
Extend that shared token system before styling individual pages.
Check available primitives before introducing custom behavior.
Preserve native semantics, edit ownership, drafts and undo/redo.
```

## Delivery at a glance

| Stage | Deliverable | Evidence needed to finish |
| --- | --- | --- |
| Preparation | Durable comp/provenance packet, real baseline states, exact batch paths | References available outside the ignored mock folder; existing changed hunks inspected |
| 1 | Shared Graphite tokens, controls, shell and Layout presentation | Contrast and states measured; navigation, selection, snapping, drafts and save/recovery still work |
| 2 | Layout direct geometry controls | Pointer/field parity; preview, commit, cancel, scope replacement and undo/redo verified |
| 3 | PCB and Case | Placement/wiring and mechanical settings work with current preview/export ownership |
| 4 | Keymap and Keycaps | Bindings, profiles, legends and overrides survive navigation and save/reopen |
| 5 | Parts and Export | Catalogue/import/apply/place/download paths work; stale results remain blocked |
| 6 | Project browser and onboarding | Saved/demo/import/new-project journeys reach real editable state; setup is optional/resumable |
| Integration | Cross-workspace review, required checks and final design documentation | Actual behavior and visual evidence; unresolved work explicitly listed |

## Implementation status — 2026-10-12

Batches 1 and 2 are implemented and approved in chat. Batches 3–6 presentation
and entry-flow behavior is implemented, but visual acceptance is reopened after
user feedback identified substantial mockup fidelity gaps. The corrective work is
tracked in [the visual gap review](../../design/graphite/visual-fidelity-gaps.md).
The first corrective batch updates shared styling and Layout hierarchy; its
174 Layout browser tests and cross-page responsive checks pass. Remaining visual
work stays open in that review. Functional verification and explicit interaction bounds
are recorded in [the continuation packet](../../design/graphite/batches-3-6.md).
The unchanged broad-suite Align focus failure is resolved by cleaning up the
preceding matrix fixture. Case height/thickness drag handles are excluded from
scope per user direction; existing numeric editors remain available.

## Purpose and success

Operate-mode workbench for keyboard designers who need fast, precise Layout editing and dependable handoffs across PCB, Keymap, Keycaps, Case, Parts and Export. Frequent actions must be visible at the selected object, snapping must be discoverable, and specialist settings must open in context. Project entry should make it easy to resume a local design or reach a real editable layout.

Success means a user can select a key, row, column or matrix; find applicable tools without opening several menus; change geometry with a pointer or exact field; undo the accepted change; and continue to another workspace without losing scope, drafts or save feedback. A new user can open a saved keyboard, start a demo copy, or create a layout through optional setup.

## Fixed visual authority and scope

- The selected Graphite Layout comp is `.impeccable/mocks/redesign-2026-10-12/layout-dark-d/i-graphite.png`. Cross-workspace references, exact prompts, provenance and review notes are in `.impeccable/mocks/redesign-2026-10-12/graphite-suite/`. The browser gallery at port 8184 is a review artifact, not the application.
- Preserve neutral charcoal surfaces, periwinkle selection, violet components, green board geometry, Source Sans 3 / Source Code Pro, one application header, docked navigation and inspectors, and compact opaque contextual toolbars with inline icon and label. No new design alternatives, second top bar, floating inspector/Objects panels or spreadsheet feature.
- Each workspace retains its own navigation job: Layout/PCB objects, Case assembly, Parts catalogue, Keymap layers. Setup remains separate from Objects. Project browser and full-page setup have a simpler header.
- The comps establish hierarchy and composition, not fabricated geometry, example pin assignments, state badges or erroneous labels. Render keyboard/footprint/case content through the existing product renderers, not generated screenshots.
- This brief extends the current implementation. Preserve unrelated uncommitted work. Historical orchestration rules in older planning documents are retired by the current AGENTS.md and are not migration prerequisites.

## Explicit implementation stack

Use the existing Rust/Dioxus frontend, Dioxus Components primitives and Tailwind CSS pipeline. Do not introduce React/shadcn runtime islands, another UI framework, a second document store, or a replacement styling/build pipeline.

Current evidence:

- `web/crates/ui-shared/Cargo.toml` pins Dioxus 0.7.10 and `dioxus-primitives` from DioxusLabs/dioxus-components at revision `9a758255ea26e2b20c8cecf4c1c946feb9e71da7`.
- `web/crates/ui-shared/src/controls.rs` supplies BoardButton, BoardCheckbox, BoardDisclosure, BoardDialog, BoardNumberField, BoardSelectField, BoardToggle and BoardPopover. Popover and Toggle use Dioxus primitives; other controls retain intentional native behavior.
- `web/assets/m1.css` owns current theme variables and base workbench styling. `web/tailwind.css` maps those variables through `@theme inline` and scans the Rust presentation sources. It does not enable Tailwind Preflight.

Build on these shared controls. For new tabs, menus, tooltips, selection widgets or other behavior, inspect the pinned Dioxus Components implementation before choosing a primitive. Prefer a suitable primitive behind a BoardStudio wrapper; retain native elements where they already meet the interaction contract. Availability alone does not prove compatibility with controlled values, focus restoration or draft retention. Add only the dependency surface actually needed, with mounted browser evidence. Existing authorization covers exported shared UI controls; it does not require widening domain APIs.

Tailwind expresses layout and common styling through semantic theme roles. Shared component classes own repeated control states. Pointer geometry, SVG/WebGL styling and canvas interactions stay in their existing rendering/interaction layers. Do not copy raw color values into each workspace or spread duplicate control state rules between Tailwind and legacy CSS.

### Pinned primitive availability and selection gates

Source inventory checked locally on 2026-10-12 against the exact pinned revision above, using upstream `primitives/src/lib.rs` and the relevant component implementations. This establishes available candidates; it is not a mounted BoardStudio compatibility result.

| Need in the comps | Available pinned primitives / existing wrapper | Plan before custom behavior |
| --- | --- | --- |
| Adaptive contextual toolbar | Public `toolbar` module: Toolbar, ToolbarButton, ToolbarSeparator; existing BoardButton/BoardToggle styles | Verify focus order when selection changes the visible actions, disabled items and embedded menu triggers. Reuse control styles without nesting buttons. Keep canvas shortcuts from intercepting focused controls. |
| Inspector and viewer tabs | Public `tabs` module | Verify controlled active tab, keyboard activation and content-mount lifecycle before replacing existing tabs; preserve field draft/settlement ownership. |
| Overflow, action and context menus | Public `dropdown_menu`, `context_menu`, `menubar`; existing BoardPopover | Choose only the required menu type. Verify dismissal, Escape, trigger focus restoration and scope retirement. Common actions retain a visible entry point; right-click is an additional path. |
| Inspector disclosures | Existing native BoardDisclosure; public `collapsible` and `accordion` | Retain the existing wrapper unless richer behavior is required. Collapsible defaults can unmount children; its `keep_mounted` option must be evaluated when retaining drafts is required. |
| Informational tooltips | Public `tooltip` module | Use for supplemental descriptions and shortcut hints, with focus access. Do not hide the only action label or a required field behind hover. |
| Select/search binding inputs | Existing native BoardSelectField; public `select`, `combobox` | Verify controlled values, search, keyboard/pointer interaction, invalid values and selection replacement before adopting a custom picker. Native select stays sufficient for simple finite choices. |
| Checkboxes, switches and segmented modes | Existing BoardCheckbox/BoardToggle; public `checkbox`, `switch`, `toggle_group`, `radio_group` | Match actual semantics: independent boolean, pressed mode or exclusive choice. Keep change-event adapters and controlled state correct. |
| Confirmation and task dialogs | Existing native BoardDialog; public `dialog`, `alert_dialog` | Verify focus trapping/restoration, Escape, pending-action dismissal rules and preservation of current native semantics before changing the wrapper. |
| Trees, project previews, setup steps and geometry handles | Existing workspace/domain components plus the shared controls above | Compose project-specific UI around current owners. The public module inventory does not expose a ready-made BoardStudio object tree or CAD handle; do not substitute generic drag-and-drop state for document gestures. |

At each primitive adoption, record the existing wrapper, candidate API, controlled-state/focus/mount requirements, selected implementation and affected browser checks. Avoid dependency upgrades or importing every available primitive as a side effect of the redesign.

## Shared token foundation

Extend the current `--wb-*` system rather than creating a parallel Graphite token hierarchy. Map Graphite to the existing dark appearance; retain Light/System choices and persisted preferences. Light appearance receives compatible shared component behavior without a second visual redesign.

| Token group | Shared meaning | Graphite reference |
| --- | --- | --- |
| Surfaces | Canvas, panel, field, header, contextual toolbar | Canvas #15171B; panel #202329; field #171A20; header #1A1D22; toolbar #3A3F48 |
| Content and borders | Main/secondary text; ordinary/strong boundary; focus | Text #EDF1F7; muted #AFBCCB; toolbar edge #727C8B; field edge #66717F |
| Interaction | Action, on-action text, selection surface/text, hover, pressed, disabled | Periwinkle #9AADFF; selected surface #303B56; dark ink on filled actions |
| Geometry | Key, part, board, outline, dimension/handle and transient preview | Preserve distinct periwinkle, violet #B98CD1 and green #6AB99D roles |
| Feedback | Save, pending, failure, warning, available, stale | Preserve semantic status roles; selection is not success |
| Type | Body, section, inspector title, labels, measured values | Existing local fonts; monospaced measurements and stable units |
| Density | Control heights, icon sizes, spacing, panel widths, toolbar gaps and radii | Measure reference regions; provide consistent compact/comfortable behavior |
| Motion and layers | Feedback duration, reduced motion, canvas overlays, menus and dialogs | Extend existing duration/easing roles; keep overlays predictable |

The palette values are reference inputs, not a measured accessibility result. Measure text, focus, selection and handle contrast in the implemented interface. Correct generated decorative gradients to flat token surfaces. Keep raw palette values at the token-definition layer; use existing semantic names where sufficient and add roles only when their intent is distinct. Document final implemented tokens in DESIGN.md and the Impeccable design sidecar at the foundation's finish.

## Shared components and ownership

| Surface/pattern | Reuse or extend | Ownership boundary |
| --- | --- | --- |
| Header and workspace frame | Single header, project navigation, save feedback, dock slots and status footer | Shell composition in `web/src/presentation.rs`; workspace containers remain in the bin |
| Buttons and fields | Existing Board controls, shared density, unit adornments, focus and failure placement | `web/crates/ui-shared/src/controls.rs`; consuming workspace owns drafts/intents |
| Context toolbar | One solid backing, labeled actions, active/disabled states, compact overflow | Shared visual composition; each workspace supplies tool availability and callbacks |
| Inspector sections | Reusable section/disclosure, field grouping and Properties/Relations presentation | Shared appearance; workspace inspector mounts retain lifecycle and target identity |
| Docked navigation | Shared row/focus/selection vocabulary for trees, lists and catalogue groups | Domain-specific hierarchy remains with Layout, Case, Parts or Keymap |
| Canvas geometry controls | Handles, dimensions, selection feedback and unobscured placement | Existing canvas coordinate and gesture owners; no document writes from visual primitives |
| Project previews and setup | Shared preview frame, form controls, process navigation and optional hints | Library/setup owners retain creation, import, save and resumable-guide behavior |

Do not build a generic editor framework or generic tree controller to force different workspaces into one state model. Extract repeated appearance/composition when it has the same intent; keep domain behavior in its current owner.

## Behavior to preserve

- Durable edits continue through existing application/runtime interfaces and accepted-document settlement. Keep pending-field drafts, errors, one-shot disabling and stale-owner protection. A transient preview is not an accepted revision or a save.
- Preserve project/board identity, workspace navigation, selection contexts, target IDs, unit precision, undo/redo, matrix and key assembly behavior, outlines, mechanical mirroring and existing constraints.
- Column stagger is inherited by following columns; column offset changes one column independently. Splay preserves its existing origin and affect rules. Inspector and direct controls must agree on these semantics.
- Preserve local browser storage, import/export, save/recovery feedback and current supported project-loading behavior. Do not add cloud/accounts, replace archive formats or change storage migration rules.
- Retain generated-preview/apply distinctions, revision-bound jobs and export freshness checks. Availability, valid output and fabrication readiness remain different concepts.
- Preserve current keyboard commands and native form semantics. A visually collapsed section must not silently discard newer drafts, pending operations or failure messages.

## Staged delivery

### Preparation — reference and baseline packet

Before the first code batch, preserve the selected comps and prompts in a durable handoff location: the current `.impeccable/mocks/` artifacts are ignored and must not be the only copy available after switching checkouts. Capture representative actual app states, measure region sizes and record the generated defects below. Inventory changed hunks before touching overlapping files. Record exact batch paths and relevant tests. This is translation of the selected direction, not another design round.

### Batch 1 — shared tokens, shell and Layout presentation

Deliver in that order within one coherent reference implementation:

1. Map Graphite through existing theme variables and Tailwind roles; establish shared control, type, density and interaction states.
2. Apply the single-header shell, docked navigation/inspector layout and readable opaque contextual toolbar. Preserve save/recovery UI and workspace routes.
3. Move Layout's existing frequent actions, snapping controls and inspector sections into the selected composition. Show only tools applicable to current selection, with an explicit path to specialist actions. Keep existing functional tools working before introducing new handle affordances.

Primary touchpoints: `web/assets/m1.css`, `web/tailwind.css`, `web/crates/ui-shared/src/controls.rs`, `web/crates/ui-shared/src/panels.rs`, `web/src/presentation.rs`, `web/src/presentation/layout_workspace.rs`, and relevant Layout objects/toolbar/inspector modules. Avoid wholesale replacement of the shell's existing event wiring.

Exit: a working Graphite Layout at real data sizes, visible snapping, consistent inspector controls, one header, no hidden common functions, preserved save/selection/undo, and no clipped actions. Unmigrated workspaces remain operable through the shared shell. Record any direct-handle work still pending explicitly.

### Batch 2 — Layout direct geometry interactions

Close gaps between existing tools and the chosen direct controls: move handle, stagger and independent offset handles, splay arc/origin, and editable dimension labels. Reuse existing activation, gesture and typed-edit routes. Use the same value and unit semantics as the inspector; do not duplicate transform math in UI components.

Handle contract: begin captures target/scope and pointer ownership; moves produce transient preview with current snapping; release submits one accepted gesture change; Escape/pointer cancellation makes no accepted change. Selection, project, board, workspace or target changes retire the old interaction. Numeric entry and keyboard equivalents remain available. Toolbars/labels avoid covering selected geometry and remain legible at different zoom levels.

Primary touchpoints: existing Layout transform/inspector adapters, canvas handlers in `web/src/presentation.rs`, Layout workspace state and relevant shared geometry presentation. Verify actual existing capabilities before adding a new route. Any missing domain capability gets a named bounded follow-up instead of a presentation-side document mutation.

Exit: pointer and exact-field edits agree; stagger inheritance differs correctly from independent offset; cancel/scope switch and undo/redo work; previews never become saves by themselves.

### Batch 3 — PCB and Case

PCB reuses the shell and direct placement vocabulary around board objects, layer visibility, controller placement and wiring. Keep placement and wiring tab content coherent, pin review explicit, and Resolve/Apply lifecycle intact. The mockup does not add PCB routing.

Case uses its assembly tree and large existing 3D preview, selected-body construction controls, named advanced sections and explicit pending/stale/failure states. Case height/thickness drag handles are excluded from scope per user direction. Retain the existing numeric mechanical editors. Do not draw fake validated geometry.

Touchpoints: `web/src/presentation/pcb_workspace.rs`, `case_workspace.rs`, workspace crates under `web/crates/pcb/` and `web/crates/case/`, plus existing viewer presentation. Exit with working controller/pin edits and mechanical edits, correct preview currency and unchanged export blockers.

### Batch 4 — Keymap and Keycaps

Keymap gives layers and key behaviors their own navigation and contextual tools. Keep selected key, selected layer, macros, encoders and firmware handoff behavior intact. Transparency belongs to key behavior, not a layer checkbox.

Keycaps reuses the actual keyboard view, profile/row assignment, legends, colors, overrides and clearance findings. Keep global appearance separate from selected-key overrides and preserve model/fit-job identity.

Touchpoints: corresponding workspace containers and `web/crates/keymap/`, `web/crates/keycaps/`. Exit with actual bindings/profile changes surviving save/reopen and selection/workspace transitions; no fake clearance success states.

### Batch 5 — Parts and Export

Parts retains its searchable catalogue, real footprint/model previews, source attribution, fit profiles and explicit generator preview/apply/placement actions. Common settings come first; specialist settings remain findable. Export uses a handoff list with real current availability, blocker explanations, recovery links and a separate portable project download.

Touchpoints: Parts workspace/container, `web/crates/parts/`, Export workspace and existing firmware/export adapters. Exit with import, apply, placement and download paths working; stale results cannot replace current scope; browser autosave is not described as an archive download.

### Batch 6 — project browser and onboarding

Project browser distinguishes current/local saved keyboards from demo copies and imported files. Use actual saved-project data and actual thumbnail geometry; selected preview and Open keyboard act on the selected document. Preserve rename, delete confirmation, search, load/recovery failures and portable-copy behavior. Do not add fictitious cloud/favorite/team functions.

Onboarding moves into a dedicated setup surface with the existing five stages: Project & hardware, Layout & assemblies, Controller & wiring, optional Case, Review & export. Hardware choices show the physical arrangement before keys exist. Matrix setup shows the resulting geometry and opens the real editor; later steps remain resumable and optional where already allowed. Keep guide completion distinct from output validity. Preserve separate meanings of split hardware, mirrored layout, and reversible PCB design.

Touchpoints: `web/crates/library/src/library.rs`, project-menu integration, `web/crates/layout/src/setup_guide.rs`, matrix setup and physical-setup adapters, and shell routing. Moving guide presentation does not require replacing creation/import/storage owners or introducing a second wizard document.

Exit: resume saved project, open demo copy, import a project and create a keyboard all lead to working editor state; skipping/reopening setup preserves work; creating once cannot duplicate on repeated activation while pending.

### Final integration — verification and documentation

Complete the cross-workspace task journeys and the required repository checks once the affected batch checks pass. Resolve new integration findings without reopening unrelated design alternatives. Update DESIGN.md and the machine-readable design sidecar from the implemented system. Keep a short evidence packet per batch with screenshots, affected checks, known limitations and remaining interaction gaps. The frontend watcher remains running.

## States, content ranges and responsive behavior

Include no project, empty board, no selection, key/row/column/matrix selection, multiple selection, long names, larger object catalogues, collapsed/expanded sections, invalid numeric draft, pending/failed edit, save failure/recovery, loading/unavailable model, stale preview and blocked output. Use real representative project data; generated thumbnails do not define content limits.

Desktop keeps docked panels and a dominant canvas. At intermediate widths, collapse secondary sections or use explicit panel switches rather than overlaying fixed panels on the selection. On narrow screens, present Objects/Canvas/Inspector as reachable workspace views, keep one header with accessible overflow navigation, and preserve selection and editing state while switching. Setup keeps its process navigation reachable without forcing the desktop three-column form into a phone width. Touch access requires appropriately sized handles and field alternatives; do not rely on hover or right-click alone.

## Verification plan

Planning changes run documentation checks only. During implementation:

- Use `python3 scripts/check.py quick --files <exact batch files>` for the changed-surface loop. Include relevant WASM compilation and mounted browser coverage; native tests alone do not exercise presentation code. Shared-control changes legitimately affect several dependent crates.
- Retain existing meaningful behavior tests. For a bug fix, demonstrate expected failing coverage before the fix. For new gestures, cover preview/commit/cancel, target and scope replacement, exact geometry, snapping and undo semantics. Do not add tests that merely repeat class strings or token definitions.
- Batch visual inspection at desktop (1600×1000), intermediate (1024×768), narrow (390×844), and the user's actual viewport if different. Match key regions against the selected comp and inspect real selection, expanded, failure and disabled states. Measure contrast; verify keyboard order, focus restoration, Escape, touch access and reduced motion.
- Build the batch completely, inspect the viewport/state matrix together, correct the material findings in one batch, and confirm once. Avoid per-control rebuild/polish loops. Required repository checks still must pass; the visual pass limit does not excuse a failing behavior gate.
- Run the full `python3 scripts/check.py` integration check before calling the migration complete. Test production assets/offline behavior in the production preview when relevant; the development server's absent service worker is not offline evidence.
- Preserve the independent frontend-only loop at port 8182 with private target/cache output. Reuse existing providers; rebuild providers only when their inputs change. Port 8184 remains the mockup gallery. Consult current runtime receipts before managing processes; do not start duplicate watchers.

## Generated defects that must not become requirements

Use suite `review-notes.md` as the detailed correction list. In particular: generated key counts/pin numbers/footprints are illustrative; remove the layer-wide Transparent key checkbox; keep Cherry and OEM distinct; label portable project export as a download; avoid duplicate Export navigation/actions; remove irrelevant browser undo/redo/Saved controls; use vertical stagger handles; starting hardware silhouettes are schematic, not generated board outlines. Slight raster gradients do not introduce a gradient-based design language.

## Open decisions and bounded assumptions

- Direction, audience, stack, no-floating-panel layout and first batch are settled. Do not repeat their discovery questions.
- This sequence prioritizes the requested Layout work, then editor families, then entry flows. Every named screen is in scope; stages are dependency/review boundaries rather than optional omissions.
- Exact responsive thresholds, panel dimensions and component primitive choices are implementation measurements constrained by the selected composition and behavior. They do not require a new concept round.
- New direct handles and mirrored matrix creation must be checked against existing supported operations. Missing semantics remain explicit follow-up work rather than silently changing Core rules.
- The existing dirty component migration is part of the implementation starting point. No source, theme or skill-state files were changed by this planning task.

## Reusable implementation instruction

Use this wording when starting each implementation batch:

```text
$impeccable implement the next confirmed Graphite migration batch in
docs/plans/ui-design/graphite-migration.md.

Keep the selected Graphite mockups as the visual authority. Use the existing
Rust/Dioxus frontend, pinned Dioxus Components primitives, and shared BoardStudio
controls. Use Tailwind CSS through the existing semantic --wb-* theme tokens;
extend the shared token system before adding page-specific styling. Inspect
primitive support before introducing custom behavior. Preserve existing native
semantics where they already satisfy the interaction contract.

Preserve accepted-document edits, selection/scope ownership, draft settlement,
undo/redo, save/recovery and revision-bound previews/exports. Implement the
batch's new interactions explicitly and verify them with mounted browser tests.
Use actual product geometry and data. Preserve overlapping uncommitted work.
Keep the independent frontend preview running; avoid provider rebuilds unless
their inputs change. Do not generate new design alternatives.
```

The `shape` task ends with confirmation of this brief. Implementation, direction-contract persistence and token/code changes belong to the subsequent implementation request.
