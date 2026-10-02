# F7 Case and shared-viewer parity reset addendum

**Purpose:** Refine the existing F7 work into observable, independently demoable slices after the user’s parity reset. This addendum does not close F7, replace the existing specification, approve API/visibility changes, or change the canonical parent graph by itself.

## Problem Statement

The current public Case route still presents Layout’s Board/instance/group tree instead of React’s Case assembly/body tree. Mechanical controls exist but live in a central “Case settings” disclosure, while authored Case editing lives in the right Inspector. Generated Case CAD can render, but the scene projection contains no board surfaces, holes or model rows. A private model-delivery helper exists in source but is not registered or called by the page; that is a wiring gap, and the public absence of model meshes is a distinct observable result.

The user’s parity target covers all six workbench streams: Layout, PCB, Keymap, Keycaps, Case and Parts. Current shared-viewer task text names only Layout, Parts, Keymap, Keycaps and Case; the six-stream matrix needs PCB represented through the existing F5 cross-workspace acceptance path.

## Solution

First provide a real Case-context object hierarchy and contextual Inspector using the existing accepted document/session projections, authored Case editor, mechanical settings/controller, and scoped viewer state. The hierarchy must represent actual authored bodies, current generated stack entries and real current-board components, with exact producer identities and mode-specific selection. It must not create a decorative/partial tree with fabricated nodes. In the same vertical slice, place the existing mechanical settings and authored-body editor in the Case Inspector context while preserving the generated-stack/authored-body exclusive state.

Then complete and accept the shared scene/model path through the existing private renderer/runtime seams, followed by each workflow’s own correctly scoped viewer projection. Keep parent acceptance joins intact; start fixture-backed capabilities as soon as their exact reviewed seam is available.

## User Stories

1. As a designer, I want the Case Objects tree to show the selected physical assembly, so that Case navigation is distinct from Layout grouping.
2. As a designer, I want authored bodies and generated mechanical stack rows to come from their real current sources, so that the tree never invents project objects.
3. As a designer, I want actual PCB components represented beneath the Case assembly, so that tree selection identifies the same component the project contains.
4. As a designer, I want selecting an authored body or supported mechanical row to open its current Inspector context, so that the visible tree and controls describe one object.
5. As a designer, I want mechanical settings and authored body controls in the contextual Inspector, so that configuration stays beside the selected Case object.
6. As a designer, I want the authored-body editor hidden while a matching generated stack is active, but saved bodies retained, so that editing modes remain exclusive without data loss.
7. As a designer, I want disabling the stack to restore the authored body list and its controls, so that I can resume authored Case editing.
8. As a designer, I want Case layer visibility/color preferences to remain display state, so that inspection does not accidentally edit the saved document.
9. As a designer, I want viewer picks to map only to current, known project identities, so that stale or renderer-only IDs cannot select unrelated objects.
10. As a designer, I want paired journeys through Layout, PCB, Keymap, Keycaps, Case and Parts, so that shared viewer behavior and each workflow’s ownership are checked together.
11. As a designer, I want the viewer to use real model assets, board transforms and scope-specific IDs, so that 3D views show the current accepted assembly.
12. As a designer, I want saved documents, undo/redo, board/instance switches and pending results to retain their existing authority, so that a viewer/context change cannot cross scopes.

## Implementation Decisions

- Case’s first visible contextual Objects/Inspector move reuses existing Session/ReadModel, Case-body, mechanical-settings and renderer-facing projection state. It changes page composition and selection routing only; no duplicate document/session/history owner is introduced.
- Case tree contents come from the selected board and active Case mode. Authored mode uses only current-board `case_bodies`; generated mode uses only current accepted resolved-stack/output identities; PCB nodes use actual board membership and live project parts. A Main case assembly row may act as a display-group control only while backed by the current resolved scene; it is not a fabricated editable document object. Other groups organize real rows but do not invent selectable domain identities. Empty, unavailable and unmapped states stay explicit.
- Selection carries the actual current scope and source identity. Authored body selection routes to the existing Case body editor; mechanical selection routes to the existing stack context; part selection routes only where an existing mapped inspector exists. Unknown, stale and display-only IDs do not fabricate a domain selection.
- Keep generated and authored Case modes exclusive. Generated geometry does not overwrite or masquerade as authored body data. “Disable mechanical stack” remains governed by its current canonical/physical-scope semantics.
- Move the existing MechanicalSettings component/controller into the Case Inspector composition without changing its accepted event/edit path. Keep the existing authored Case editor and its drafts/outcomes in the Inspector; do not regress either owner while changing layout.
- The shared viewer stays one private implementation. Case consumes a physical-instance scene; Layout/PCB/Keymap/Keycaps use their existing canonical or workflow-specific scene; Parts uses its isolated sample. Exact model/part/body pick mapping remains consumer-owned.
- The existing private model-delivery source is not proof of delivery. Complete actual page registration, verified asset provider/decode, scene projection and public model display/pick behavior through reviewed private seams. The current empty `models` input and empty board surfaces/holes are direct source evidence of the public mesh gap. Do not widen public APIs or member visibility on this evidence.
- Keep F7.3 `INT.2` and `BND.1` as acceptance joins, F7.4 `INT.2`, and F7.7 `F5.6`. They are not blanket start blockers for independently fixtureable Case tree/editor or viewer capabilities. F7.8 remains downstream acceptance; its six-stream suite should include existing F5.8 PCB acceptance as an acceptance join, not as an implementation start blocker. The coordinator should reconcile that edge in the canonical 62-task graph; this addendum does not edit `tasks.json`.
- Preserve the approved private boundaries and feature ownership. No Core/CAD/renderer algorithm rewrite, public contract expansion, document schema change or parallel viewer is part of the first contextual tree slice.

## Testing Decisions

- Compare public React and Dioxus behavior on the exact same saved project/archive, including accepted document identity and revision. Use genuine source fixtures; don’t infer parity from two similarly named demo copies.
- Test Case tree membership against the accepted document and current `CadScene`/resolved stack: no stale, cross-board, renderer-only or synthetic selectable IDs. Check authored mode, generated mode, empty state, board/instance change and mismatched configuration.
- Select real body, stack and PCB component rows and verify the right Inspector’s context and field source. Verify display toggles/colors do not modify document/history.
- Exercise configure, edit, disable, undo/redo, save/reopen and board/instance switching through public controls; the existing Runtime/session remains the only write authority.
- For viewer model acceptance, verify real model row IDs, document Asset/SHA byte source, transforms, missing/retry behavior and exact current pick mapping. Existing decoder/native projection tests alone are supporting evidence.
- Run a paired six-workspace journey across Layout, PCB, Keymap, Keycaps, Case and Parts. For each, test the existing public route and viewer only where that workflow has one; preserve each workflow’s canonical, physical-instance or isolated-sample identity. Record the source/build/fixture and actual outcomes.
- Preserve compact/desktop keyboard and focus behavior, display modes, layer state, camera, errors, retries, cancellation, stale-scope rejection and unmount/lifecycle behavior. Keep actual assistive-technology and accessibility acceptance as separate required checks.

## Out of Scope

- Changing parent completion, task status or canonical acceptance joins by assertion.
- Adding fake rows, synthetic IDs, placeholder models or a parallel Case/3D viewer.
- Changing Core/application public APIs, schemas, visibility, renderer exports, geometry authority, file format, durable storage or session history.
- Claiming imported/generated model delivery from the existence of an unregistered helper.
- Treating a generated CAD preview as proof of board model delivery, exact STEP export, multi-instance correctness, all six consumer integrations or full F7 acceptance.

## Further Notes

The paired audit and public action evidence are retained in `../evidence/case-shared3d-parity-reset-20261002/`; source, build, profile, hash and proof limits are stated there. That exploratory pair is not a same-archive acceptance run. The integrated current source is `89b1de8a28fdf02db91d972c90a69235bfbbbffb`, and the pinned React reference is `5a472a9426e6e38993361da402cd4ec730feb369`.

Reconciled vertical tickets:

- Case hierarchy and contextual Inspector: `../../dioxus-case-workspace/issues/09-case-contextual-objects-inspector.md`.
- Six-stream shared-viewer paired acceptance: `../../dioxus-shared-viewer/issues/09-six-workspace-paired-viewer-acceptance.md`.
- Imported and generated model delivery remain the existing reviewed children `../../dioxus-shared-viewer/issues/07-case-imported-board-model-delivery.md` and `../../dioxus-shared-viewer/issues/08-case-generated-board-model-delivery.md`; their implementation/wiring and public acceptance stay open.

**Refactoring ledger:** this audit adds source-backed evidence to RF-001 (shared composition/Inspector reachability hotspot) and RF-009 (acceptance accounting/six-stream mismatch). Required parity remains current work; this observation grants no refactor or API change.
