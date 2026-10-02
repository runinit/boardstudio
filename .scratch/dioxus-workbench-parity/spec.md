# Six-stream Dioxus Workbench parity

**Status:** draft for the confirmed 2026-10-02 execution model; stream-audit reconciliation is in progress.
**Reference:** React `5a472a9426e6e38993361da402cd4ec730feb369`.
**Candidate:** use the exact integration revision and build recorded in each paired audit; the 2026-10-02 screenshot sets span more than one candidate snapshot, so they are evidence inputs, not a single acceptance run.
**Authority:** user-confirmed decisions Q1–Q6 in `/tmp/frontend-parity-reset-20261002/decisions.json`, plus the 62 canonical parents in `.scratch/dioxus-frontend-v1/tasks.json`.

## Problem Statement

The current Dioxus application presents some real workflows, but the user still sees mismatched workspace composition and substantial missing contextual UI. The reference experience is organized as six persistent workbench streams—Layout, PCB, Keymap, Keycaps, Case/shared 3D, and Parts—with project operations exposed through the Project menu. The active workbench and current selection determine what appears in the Objects pane, canvas/central work area, and Inspector. A workspace tab, a mounted module, or a screenshot of an unselected shell does not prove that a designer can complete the corresponding task.

The user requires fidelity to the existing TypeScript application: placement, labels, pane hierarchy, contextual controls, menus, default/empty states, interaction boundaries, history and durable outcomes. Known source defects and platform differences are recorded; parity work should preserve the observable reference behavior and report a real incompatibility instead of silently redesigning it.

The old task graph remains the canonical scope: 62 parent tasks, unchanged criteria, historical rationale and final acceptance joins. It is not to be replaced by these six queues or a shorter checklist.

## Solution

Complete the same user-visible workflows in Dioxus through six persistent domain queues. Begin with the one bounded, private Workbench-composition preparation slice associated with already accepted INT.1. That slice establishes reviewed ownership and private composition slots before implementation. It does not complete a user-visible workflow and cannot close any F2–F9 parent. Implement each stream in disjoint private feature modules and integrate continuously through the approved limited private composition seam.

Use the Project menu as shared shell support owned with project operations and coordinated with Parts. It is not a seventh workspace. It must expose the real reference actions, including creating a keyboard and opening the available demo keyboards, while preserving the existing project/session authority.

For every feature child, name the exact minimum capability that blocks starting, with source and observed evidence. A proven capability-level start can allow a child to start while a parent’s broader acceptance remains open. This is narrow and evidence-backed; it does not waive any parent criterion, canonical completion join, cross-workspace acceptance, provider gate, accessibility requirement, or release check. Keep the historical reason for every start edge. `acceptance_after` remains the gate for final parent completion.

## User Stories

### Shared shell, project operations and contextual workbench

1. As a designer, I want the exact six workbench labels and order—Layout, PCB, Keymap, Keycaps, Case, Parts—with Export in its reference location, so I can recognize and navigate the same application.
2. As a designer, I want project operations in the Project menu, so project discovery and creation do not displace workbench controls.
3. As a designer, I want to create a keyboard and open each available real demo keyboard from the Project menu, so I can start with either a blank or useful example project.
4. As a designer, I want saved-project open and the existing import/recovery routes to keep their reference behavior, so I can return to my work safely.
5. As a designer, I want the Objects pane to show the active workbench’s relevant hierarchy, so I can find the board, matrix, physical assembly, components, layers, models or bodies that belong to this task.
6. As a designer, I want the Inspector to update for the active workbench and current selection, so controls describe the object I actually selected.
7. As a designer, I want empty, single, multiple, stale and unsupported selections to be represented honestly, so I can tell what an action will affect.
8. As a designer, I want selecting an object in the tree or canvas to keep the same stable identity and scope, so context does not drift between workspaces.
9. As a designer, I want workbench navigation, opening panes and disclosure changes not to create document revisions, so presentation state remains separate from edits.
10. As a designer, I want accepted edits, history and session identity preserved when moving between workspaces, so changing context does not silently discard work.
11. As a designer, I want keyboard, pointer, focus, compact-drawer and theme behavior to match the reference, so all workspaces remain reachable across supported layouts.
12. As a designer, I want accurate loading, no-result, unavailable, validation, failure, retry and success feedback at the control that caused the action, so I can recover without guessing.

### Layout

13. As a designer, I want Layout Objects grouped into the same board, matrix, row/column, key, component, outline/version/bridge and refinement contexts as React, so I can navigate the complete design structure.
14. As a designer, I want independent group disclosure, stable selection, board membership and tree/canvas synchronization, so opening a section or selecting an item has the same scope as the reference.
15. As a designer, I want the Select, Transform, Align and Snap command controls in their reference location and with their actual enablement rules, so common layout actions are discoverable.
16. As a designer, I want key or matrix selection to show the matching Inspector with dimensions, origin, splay, transforms, constraints, relations and lock/driven state, so I can edit the selected layout context.
17. As a designer, I want to create preset/custom matrices and mirrored pairs with a non-durable placement preview, so I can position and inspect them before committing.
18. As a designer, I want placement arrows to respect the active snap increment, Enter to commit, and Escape/cancel to leave history and durable geometry unchanged, so I can safely place objects.
19. As a designer, I want standalone and selected-key component insertion to follow the reference entry route and normalization behavior, so the definition, IDs, assets, cell overrides and active board remain correct.
20. As a designer, I want direct position/rotation, stagger/splay/origin, alignment, mirror and offset/constraint controls to match their reference gestures and numeric boundaries, so I can arrange components precisely.
21. As a designer, I want pointer capture, final pointer samples, cancellation, modifiers, snap guides and keyboard nudges to match the reference, so one intended drag is one predictable history action.
22. As a designer, I want generated and fixed outlines, versions, bridges, cutouts, point editing, linked refinements and scripts to use the same disclosure, draft, apply/cancel and history behavior as React, so I can author the enclosure boundary without unintended writes.
23. As a designer, I want findings to navigate to a live target and select/focus/fit it, while missing targets have no dead action, so diagnostic panels remain actionable.

### PCB

24. As a designer, I want a real PCB workspace with its own canvas, Objects hierarchy and contextual Inspector, so I can inspect a board without being sent to Layout’s presentation.
25. As a designer, I want board and host-layer controls with the React defaults, visibility, theme and labels, so toggling layers is predictable.
26. As a designer, I want PCB layer visibility and panel presentation not to mutate document revision, history or export inputs, so visual exploration is reversible.
27. As a designer, I want selected board components, mounted modules and physical instances to show the reference overlays, labels and properties, so hardware state is intelligible in context.
28. As a designer, I want controller, connector, wiring, pin, net, jumper and readiness controls to preserve the source’s entry paths and validation, so I can review electrical connectivity accurately.
29. As a designer, I want electrical findings to focus the associated board/module item through the existing navigation contract, so I can correct a located issue.
30. As a designer, I want physical board scope and routed-board/model references to remain explicit, so PCB-to-Case and export handoffs use the intended board.

### Keymap

31. As a designer, I want Keys, Macros and Encoders presented in the same tab structure as React, so I can choose the control family rather than scan a stacked inspector.
32. As a designer, I want the board outline, key selection, layer list/order and active-layer 2D binding view, so a binding remains associated with its visible physical key and layer.
33. As a designer, I want searchable keys, keycodes and supported behavior fields with the same defaults, editing/blur boundaries, validation and recovery, so I can author bindings without changing their meaning.
34. As a designer, I want tap, hold, modifier, layer and legacy binding behavior represented exactly where supported, so values survive scope changes and project reload.
35. As a designer, I want structured macro rows, order, key selection and validation to match React, so I can compose and recover supported macro actions.
36. As a designer, I want clockwise, counterclockwise and push encoder controls to use the same physical identity and supported values, so encoder actions map to the board’s real hardware.
37. As a designer, I want firmware position controls and any provider output delivered where React exposes it, so keyboard configuration can reach the existing export flow without invented controls.

### Keycaps

38. As a designer, I want a real Keycaps workspace with its 2D keyboard view and React’s board/matrix/profile controls, so I can inspect a physical layout instead of a placeholder.
39. As a designer, I want supported key selection, key finder, per-key and matrix profile controls, effective size, colors and inherited/explicit legends to match the reference, so I can style the intended caps.
40. As a designer, I want empty, standalone, mixed-matrix, override and unsupported-key cases rendered honestly without manufactured persistent defaults, so the view reflects actual project data.
41. As a designer, I want fit/clearance findings and their navigation to the corresponding key or matrix, so I can fix overlaps in context.
42. As a designer, I want the offered Keymap/Keycaps 2D/3D choice, separate caps and legend inlays, layer/visibility behavior and existing keycap STEP action, so previews and exports use the same accepted inputs.

### Case and shared 3D

43. As a designer, I want Case Objects to list physical assemblies and their child parts with selection and visibility, so the object tree reflects the assembly I am configuring.
44. As a designer, I want Case Inspector tabs/sections to show contextual construction, mechanical settings, body, layer and fit controls, so I do not have to leave the selected object to find its settings.
45. As a designer, I want authored bodies to retain their hidden editor state when generated mode temporarily replaces the body controls, so toggling modes does not lose a draft.
46. As a designer, I want canonical-board configuration and selected physical-instance configuration to remain distinct, so a split board does not accidentally change its partner’s hardware or Case.
47. As a designer, I want the same shaded/wireframe/hybrid and supported layer, fit, top/bottom/isometric, orbit/zoom and picking controls as React, so the shared 3D viewer behaves consistently.
48. As a designer, I want supported edits to preview before accepted commit, with exact findings, Undo/Redo and retry behavior, so I can recover from invalid geometry.
49. As a designer, I want generation progress, cancel and retry to match the actual reference affordances, so I know what the current model represents.
50. As a designer, I want imported board, authored/generated model and physical-instance scope to reach the viewer using their existing source/provider paths, so a rendered model corresponds to the selected assembly.

### Parts

51. As a designer, I want a searchable Parts library with the same category, footprint, assembly and module variants as React, so I can find reusable hardware.
52. As a designer, I want selected definitions and assemblies to show the same 2D/3D preview and supported editable details, so I can decide whether a part fits before inserting it.
53. As a designer, I want creation, import, generator parameter edits and save/error feedback to preserve source, license, asset and definition identities, so reusable library data remains correct.
54. As a designer, I want module and assembly relationships and local override controls to use the same selection, add/replace/remove entry routes as the reference, so reuse does not silently modify unrelated library members.
55. As a designer, I want the Project-menu create/demo entry and Parts flows to return to the same project/session with clear selection, so library work remains connected to my keyboard.

### Cross-workspace completion

56. As a designer, I want identical named fixture scenarios in React and Dioxus, so a paired result compares the same project, board, selection and initial state.
57. As a designer, I want applicable preview/cancel, Undo/Redo, save/reopen/archive and offline checks included, so a visual match also preserves real accepted and durable behavior.
58. As a designer using keyboard or assistive technology, I want the actual contextual panes, menus, focus and control relationships tested through public UI, so visible parity is also operable.
59. As a maintainer, I want every source/browser audit row mapped to an existing parent/child, observed gap or explicit out-of-scope finding, so parity gaps are not lost between queues.
60. As a maintainer, I want implementation, wiring, paired browser verification and parent acceptance represented separately, so intermediate source work is not reported as user-visible completion.
61. As a maintainer, I want each author and independent reviewer to update an existing RF entry or record “No new refactoring takeaway observed” with scope reviewed, so architectural observations remain available for the post-port refactor.
62. As a maintainer, I want six domain queues to use disjoint private modules and a small shared composition owner, so parallel authoring does not create competing session/document authorities or serialize every feature through one screen file.

## Paired audit reconciliation and current first slices

The six exploratory browser/source packets are retained in [`evidence/audit-index.md`](evidence/audit-index.md), with copied audit reports and screenshot/action evidence by domain. They share the pinned React source and verified candidate build, but several audits explicitly used similarly named rather than byte-identical projects; those findings are observed gaps, not completion evidence.

| Stream | Current source/browser picture | Smallest real visible slice to prioritize | Canonical work and completion limit |
|---|---|---|---|
| Layout | Board/matrix/row/column/key/component tree, scope-aware canvas/tree selection, semantic context summary, board contour, Layers/Footprints and X/Y edits exist. The selected matrix/row/column/key Inspector forms, Select menu, transform/align/snap controls and Outline tree/editor are missing. | Select an existing matrix, edit its name, rows, columns and pitch, and observe accepted matrix/tree/canvas state with one normal Undo/Redo path. A semantic `TreeContext::Matrix` and existing `SetMatrix` operation already form the capability seam; the blank form is not an engine gap. | Existing F3.2d child under F3.2; retain full controls and its F3.1 start until that child’s specific stable active-board/matrix context proof is reviewed. F3.1’s outline/range/focus criteria stay open; F3.2d broader acceptance remains open after the first matrix fields. |
| PCB | React presents a real PCB canvas, layer chips/host layers, selected part context, VIK overlays, Left/Right boards, wiring/resolver fields and routed reference. Candidate PCB tab is a placeholder at the tested build. | Mount the actual PCB scene for a selected host board and its contextual layer/selection controls; prove visibility toggles do not edit the document. Use the existing accepted scene/read model rather than duplicating Layout. | F5.1 starts after accepted INT.1, with existing F5.2/F5.4–F5.8 joins unchanged. React module overlay was not successfully clicked by automation, so do not assert that action from this audit. |
| Keymap | Same imported layered Sofle showed 2 layers/29 keys, labels, selection and no-match behavior. Candidate stacks Layers/Selected key/Encoders/Macros instead of React’s Keys/Macros/Encoders tabs, and omits the Outline/Generated tree and board contour. Candidate’s public ZMK action is missing; the existing provider owns it. | Make the first exact Keymap route use the reference editor tabs and expose the existing generated board outline in tree/canvas, while preserving accepted layer/key selection. Treat these as scoped shared composition/read-model work; do not regenerate outline data or implement a local firmware generator. | F6K.1 starts after INT.1 and keeps F3.1 acceptance join. ZMK belongs to existing F6K.4c child 08; F5.2/F8.2 remain acceptance joins. Binding/macro/encoder sub-slices retain their own acceptance and history matrices. |
| Keycaps | React has a physical 2D view, board/matrix/profile controls, key finder/selection, inherited/explicit legends, size/color/overrides, fit, 3D modes/layers and keycap STEP. Candidate Keycaps is an unavailable placeholder; Core config/edit/resolution is already present. | Mount the real physical 2D Keycaps projection on an accepted fixture with stable key selection, effective/inherited fields and actual 2D controls before the shared 3D/STEP path. | F6C.1 is an existing bounded projection child after INT.1; its F3.1 acceptance join stays. Settings remain F6C.2; linked resize F6C.3; viewer/CAD F6C.5 with F7.3/F8.2/BND.1 joins. No duplicate issue should replace those. |
| Case/shared 3D | Case generation, viewer display/layers, mechanical settings and authored bodies/mounts are partially live. React has a dedicated physical Case hierarchy and contextual Inspector. Candidate still presents the generic Layout tree; settings are in central Case settings and authored bodies in Inspect, so selecting Case parts/Plate/PCB object does not produce the same contextual route. | Provide Case Objects for physical assemblies/child parts and route selected Plate/PCB/body settings into the matching Inspector context, reusing existing mechanical and body operations. Keep generated mode hiding the authored editor while retaining its saved draft. | F7.2/F7.4 existing starts are INT.1; F7.3 starts after F7.1 and joins INT.2/BND.1; later F7.5–F7.8 scopes and joins remain. Do not claim model-delivery source, generated mesh or native tests prove the public multi-instance/pick/history workflow. |
| Parts and Project-menu support | React Project offers New/Open, saved search, setup guide, portable save-copy and 18 demos; initial requested shortlist is Sofle v2 and REVIUNG41. React Parts combines search/categories, assembly presets, VIK variants, import/create, definition Inspector and a separate preview. Candidate has a working catalogue/search but lacks many rows/actions and a central Parts preview; candidate project start exposes two fixture copies and import only. | Provide the selected definition’s real 2D footprint preview with named layer controls, and the Project start entry with New/Create plus the two real demos. Project menu is shared shell support assigned with Parts, not a seventh queue. | F4.1 and F2.1 are independent starts after INT.1. A 2D-only preview can begin from the existing selected-definition/footprint projector capability without waiting for F7.3; full F4.4 acceptance retains F7.3+INT.2 joins. Project durable lifecycle remains F2.2 after F2.1 with INT.2 acceptance join; two demos do not close the full gallery. |

### Capability-start evidence, not parent waivers

Q5 approves child starts after their specific prerequisite capability is proven. It does not authorize removing parent criteria, rewriting the 62-row task graph, or turning an unproven seam into a blanket waiver. For each child packet, record (1) the exact capability it consumes, (2) the existing source operation/read model and observed public evidence that supplies it, (3) the capability still absent and therefore excluded, and (4) the canonical parent and final acceptance joins that remain. Keep the historical dependency rationale in the parent graph and the new narrower proof in the child packet.

Initial evidence-backed examples are:

- **F3.2d matrix-form work:** current accepted matrix context, active board/scope identity and existing `SetMatrix`/history path are sufficient to develop the named matrix form. The broader F3.1 parent still needs outline navigation, anchored range, invalid-scope and full keyboard/compact acceptance; none of those are claimed complete by a matrix form.
- **F5.1 PCB presentation:** accepted INT.1 and current board/scene identity form the starting capability. The candidate’s current PCB placeholder is the gap, not a reason to wait for electrical planning acceptance; F5.2 remains its existing later workflow.
- **F6K.1/F6C.1 fixture projections:** accepted snapshot and the existing Keymap/keycap Core read contracts are start-capable after INT.1. The existing F3.1 shared-selection handoff is still an explicit acceptance join.
- **F7.2/F7.4 fixture UI:** existing Case body/mechanical operations and accepted fixture are enough to work on the visible contextual route after INT.1. Do not wait for F7.3 model delivery unless a specific child action actually consumes it.
- **F4.4 first 2D preview child:** the selected definition and existing compiled footprint projector are sufficient for a 2D-only public slice. The full parent’s common 3D viewer remains a later dependency/acceptance join; no F7.3 3D capability is required for 2D.
- **F7.3 shared viewer/model delivery:** retain its actual F7.1 private reachability/contract start. INT.2 and BND.1 remain full acceptance joins, not reasons to block fixture-backed private viewer work once the reviewed adapter input is available.

The canonical parent graph and every existing acceptance join are unchanged in this spec. Any operational child gate is captured in its own reviewed ticket and does not complete a parent.

## Implementation Decisions

- Preserve the React interface and behavior as the oracle. A confirmed source bug or platform limitation is recorded as a named difference with evidence and does not silently become a redesign.
- Keep one authoritative application Session and Core document/history. Dioxus owns transient presentation and drafts; do not duplicate accepted document, session state, algorithms or persistence in feature components.
- Use one limited private Workbench-composition extraction approved by Q4. It exists to define stream-owned composition slots and contextual Objects/canvas/Inspector composition; it does not introduce a general UI framework or widen public visibility.
- Six persistent queues have disjoint private feature-module ownership: Layout, PCB, Keymap, Keycaps, Case/shared 3D, and Parts. A thin shared dispatcher owned by the composition merger routes to six separately owned workspace modules; no stream author edits the dispatcher for ordinary feature work. Project-menu and project lifecycle are coordinator-owned shared-shell support scheduled alongside Parts and exercised from each stream’s routes.
- Shared root composition, Runtime, global CSS, route registration and build ownership are integrated in short serialized windows by the named merger. Private feature authors can continue in parallel once the exact composition contract is reviewed.
- The composition contract maps the active workbench, current scoped selection/read model, shared Objects/canvas/Inspector slots and feature-owned panels/intents. Exact per-workspace input/callback shape is captured by the separate prep ticket and its reviewed evidence, not guessed in this master spec. Keep existing feature-local Runtime/context use where present; the extraction must not add new session/lifecycle ownership.
- A child’s start proof names only the capabilities its own user-visible slice consumes and cites the source operation/read-model and current candidate evidence. Parent criteria and canonical `acceptance_after` joins remain unchanged; never replace an acceptance join with “capability exists.”
- No new public API/schema/document format/CAD algorithm, service visibility, broad refactor, unrelated configuration change or production entrypoint/data-writer cutover is implied.
- F1 and the earlier Layout correction remain bounded verified increments; they are not completion of this workbench spec. Do not repeat completed criteria, but preserve them as the baseline against which new contextual gaps are tested.

## Testing Decisions

- The highest seam is the actual public React and Dioxus browser applications, on the same fixture, board/instance, active workbench and selection. Use separate named browser sessions and exact build/source lineage.
- For each ticket record a paired journey: entry route/menu/tab; visible Objects, central work area and Inspector; exact selected identity; action/control; visible feedback; accepted document/history outcome; and the relevant undo/cancel/durable effect. Capture comparable screenshots or DOM snapshots at each state.
- Validate state transitions and outcomes through public UI. Native/WASM unit checks are useful for affected modules but cannot substitute for wiring or a browser journey.
- Include save/reopen/archive, offline, cancellation, error/retry, compact keyboard/focus and actual assistive-technology checks where the underlying parent criteria require them. Retain failures and environment blocks; no accessibility waiver.
- Feature authors and reviewers preserve existing browser/source evidence. Every implementation/review handoff updates the refactor index and JSON register, retaining RF IDs; say “No new refactoring takeaway observed” only with an explicit reviewed scope.
- A visible missing control is a parity gap, not an assumed missing engine capability. Reuse existing domain/session/provider operations and identify the private presentation adapter required.
- Initial audit evidence is not acceptance. Layout, Keycaps and Case used similarly named fixture copies that were not byte-compared; Keymap imported the exact same archive; PCB/Parts flows are exploratory and not full lifecycle acceptance. Source/build lineage and the limits of each packet are listed in the retained evidence index.

## Out of Scope

Backend, engine, generator and CAD/kernel rewrites; new product feature families; public API/schema/format/visibility changes; replacement of Session/Core authority; broad post-port architecture refactoring; permanent React islands for required UI; unrelated configuration changes; and production entrypoint or data-writer cutover without separate approval.

## Further Notes

- Canonical authority remains `.scratch/dioxus-frontend-v1/tasks.json` with exactly 62 parent tasks. Preserve their IDs, original acceptance wording, start rationale and all final joins. This spec adds no canonical parent. The preparatory composition ticket is associated with already accepted INT.1 and does not revise INT.1 status.
- Existing global child-record numbers 48 published / 47 non-superseded are historical. New workbench-parity prep records are tracked separately until the root explicitly reconciles counts; do not silently rewrite the historic baseline.
- Six audit packets under `/tmp/frontend-parity-reset-20261002/{layout,pcb,keymap,keycaps,case,parts-projects}` are being reconciled into an evidence matrix. When each report arrives, add exact browser/build/source lineage, observed UI location/label/action, current candidate status, tested outcome, open gap, and mapped canonical parent/ticket. Do not infer a gap from a mismatched project/tab or infer acceptance from a screenshot.
- The retained [paired audit index](evidence/audit-index.md) corrects the Parts observation: the candidate has both its catalogue and meaningful selected-definition Inspector details; the stable missing surface is the central preview/control path. It also records that audit candidate source `868edfcb` and integration base `89b1de8a` have identical `web/**` content. The root outline spot-check in `evidence/audits/root-outline/` is exploratory and not a paired run.
- Six persistent worktree owners and the evidence-backed first-slice/gate map are in [stream reconciliation](stream-reconciliation.md). The composition preparation is now implementing against its independently cleared v3 contract; the six feature stream implementations remain separate work and retain all canonical joins.
- User-observed gap register: `/tmp/frontend-parity-reset-20261002/user-gap-register.md`. Confirmed execution decisions: `/tmp/frontend-parity-reset-20261002/decisions.json` and `/tmp/frontend-parity-reset-20261002/REVISED-LOOP-DRAFT.md`.
- Every author/reviewer handoff must retain or add an evidence-backed finding to `POST-PORT-REFACTOR.md` and `refactor-findings.json`, or explicitly record “No new refactoring takeaway observed” with scope. Correctness/accessibility work remains current work even when its architectural implications are deferred.
