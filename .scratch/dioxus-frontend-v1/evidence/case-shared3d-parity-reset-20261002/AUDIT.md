# CASE + shared 3D parity reset audit

**Read-only source audit and public-UI exploration. No repository source, ticket, config, build, or commit was changed.** Browser actions changed only the isolated task-owned test profile described below.

## Provenance and limits

- React reference checkout: `/home/chris/01_Projects/ts-boardstudio2`, HEAD `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5175/`.
- Dioxus integration checkout: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, HEAD `89b1de8a28fdf02db91d972c90a69235bfbbbffb`, served at `http://127.0.0.1:34687/`. `web/src/presentation/**` was clean at inspection. The server has no `/provenance.json` endpoint (404), so candidate identity is pinned to the supplied integration HEAD plus local source hashes below, not a served build manifest.
- Agent-browser named sessions: `case-parity-react-2318ea1a1376` and `case-parity-dioxus-2318ea1a1376`; both at 1280×577. Browser page-error checks returned no entries.
- This was not a same-archive pair: React entered the public REVIUNG41 demo; Dioxus opened its public saved “REVIUNG41 copy”. Both exposed the same 85-part board count, but no exact document equality was checked. Treat the pair as UI/source exploration, not acceptance-grade same-fixture proof.
- Candidate actions `Generate case`, `Disable mechanical stack`, `+ New case body`, and `+ Add mount` used visible app controls in the isolated Dioxus profile. It advanced that test profile to revision 6; no provider or storage was injected. Do not use this changed profile as a future untouched baseline.

## Observed behavior

| Area | React reference | Dioxus candidate | Assessment |
|---|---|---|---|
| Case object tree | Separate `Main case assembly` with body children (`Plate`, `Plate foam`, `Bottom foam`, `Bottom case`) and a separate expandable `PCB` component group. Selecting a body changes the right Inspector to body fields. | The Case tab retains the Layout-oriented tree (`Board`, physical instance, `Group objects` Columns/Rows, `Keyboard PCB 85 parts`, layout groups such as right/left/thumbs). There is no case assembly/body branch. | Direct, visible hierarchy/context mismatch; the common Objects component always calls `tree::build_tree` for document layout objects. Case mode currently changes instance navigation only. |
| Mechanical settings | “Configure mechanical stack”/selected Case context leads to the right Inspector, beside body-specific fields and preview. | “Case settings” disclosure is in the Case panel over the viewer. Its visible controls include construction/mount options, dimensions/clearances, profile state, and resolved stack rows. The right Inspector remains the authored-body status/editor. | Settings and controls exist; their mounting/context is wrong. This is UI composition, not evidence of a missing Core operation. |
| Authored bodies and mounts | Body hierarchy selects Plate and exposes `Plate thickness mm` and appearance controls in the contextual Inspector. React also exposes mount-edit affordances when preview is ready. | While a mechanical stack is active, Inspector says “0 authored case bodies remain saved” and directs the user to disable the stack. Through the public `Disable mechanical stack` action the Inspector changed to `Case stack` / `+ New case body`. Adding one produced an editable Plate with Body type, Thickness, Clearance, Z offset, Mounting and Gasket channel; `+ Add mount` produced Type (Hole/Boss), X/Y, Hole diameter, Remove. Saved feedback/revisions advanced to 5 and 6. | The authoring port is present and visibly usable, but authored bodies are neither in the tree nor selected from it. Generated-stack and authored-body states are distinct. The fixture began with no authored bodies. |
| Generation/setup | React showed live preview/update/cancel and `Export geometry` (ready after CAD settled); current fixture’s mechanical findings were visible. | Public Generate case on the REVIUNG copy completed; status became `Exact case geometry ready`, with Fit case and a rendered CAD case. Settings describes the PCB reference as unpopulated. | Generation and case CAD mesh preview are green for this demo. Dioxus’s Case panel has no local Export geometry action in the captured ready state; only the global Export control was visible, and export was not exercised. |
| 3D controls | Shaded, Wireframe, Hybrid, hidden lines; Assembled/Exploded/Section, Fit/Top/Bottom/Isometric; edit mounts, layers, camera manipulation. | Expanded View controls exposed Fit case, Top/Bottom/Isometric, Rotate left/right, Zoom in/out, Shaded/Wireframe/Hybrid, Assembled/Exploded/Section, section-plane controls, hidden lines. Layer accordion exposes select, visibility, color and Reset color for PCB and generated meshes/gaskets. | Basic control set is largely present in Case; I did not run each control for visual/output parity. Rendered model inventory and selection mappings remain a separate gap. |
| Layers, selection and Inspector | Tree click on real `Plate` gave Plate Inspector. Tree click on PCB component gave PCB display controls. | Generated layer list offered `Select plate`; activating it did not switch the Inspector away from “Generated assembly preview … Disable the mechanical stack…”. The generic Layout group remained selected in the tree. | Layer selection is wired as viewer-local layer selection, not a demonstrated contextual body/part Inspector mapping. A canvas body pick was not exercised. |
| Board/component models | React reference after stack generation displayed board component models alongside the case assembly. | Dioxus showed generated CAD meshes and generated layers; the Case projection source supplies empty `surfaces`, `holes`, and `models` arrays. | Current Case projection does not deliver PCB/component model meshes. This is an adapter/producer integration gap, not a geometry-generation failure. |
| Export | Case generation panel included an `Export geometry` control; this exploration did not execute it. | No Case-local geometry export control appeared in the ready candidate snapshot. General app Export was not used. | F7.6 export parity remains open and should be proved with current exact geometry and downloaded bytes. |

## Paired action trail and captures

1. In React, enter the public REVIUNG41 demo and open Case. Saved initial capture: [react-case.png](react-case.png), [react-case-snapshot.json](react-case-snapshot.json). Expand the Case tree: [react-case-tree-expanded.txt](react-case-tree-expanded.txt). It includes the Case assembly/body branch and PCB group.
2. In React, configure the mechanical stack through the displayed Case workflow, wait for the preview and inspect the settings in the right Inspector. Captures: [react-mechanical-open.png](react-mechanical-open.png), [react-mechanical-open.json](react-mechanical-open.json). Select the Plate tree item: [react-case-body-selected.png](react-case-body-selected.png), [react-case-body-selected.json](react-case-body-selected.json). Select a PCB component: [react-component-selected.png](react-component-selected.png), [react-component-selected.json](react-component-selected.json).
3. In Dioxus, open the public “REVIUNG41 copy” and Case. Initial tree and Case layout: [dioxus-case.png](dioxus-case.png), [dioxus-case-snapshot.json](dioxus-case-snapshot.json). Select the visible `right keys Independent` tree item: [dioxus-case-tree-selection.txt](dioxus-case-tree-selection.txt).
4. Open Case settings: [dioxus-case-settings-open.png](dioxus-case-settings-open.png), later full snapshot [dioxus-settings-expanded.txt](dioxus-settings-expanded.txt). Invoke Generate case via the UI and wait until exact: [dioxus-case-after-generation-wait.png](dioxus-case-after-generation-wait.png), [dioxus-case-after-generation-wait.json](dioxus-case-after-generation-wait.json), [dioxus-case-generated-ready.png](dioxus-case-generated-ready.png), [dioxus-case-generated-ready.txt](dioxus-case-generated-ready.txt).
5. Open Case layers and colors; capture [dioxus-case-layers-open.png](dioxus-case-layers-open.png), [dioxus-case-layers-open.json](dioxus-case-layers-open.json). Select generated `plate`: [dioxus-generated-layer-selected.png](dioxus-generated-layer-selected.png), [dioxus-generated-layer-selected.txt](dioxus-generated-layer-selected.txt). The right Inspector remained on the generated-stack/authored-body notice.
6. Open View controls: [dioxus-view-controls-open.png](dioxus-view-controls-open.png). Disable the mechanical stack through the visible button and wait for the saved state; then add a real authored body and mount through the Inspector. Captures: [dioxus-stack-disabled.png](dioxus-stack-disabled.png), [dioxus-authored-body-added.png](dioxus-authored-body-added.png), [dioxus-authored-body-added.txt](dioxus-authored-body-added.txt), [dioxus-authored-mount-added.png](dioxus-authored-mount-added.png), [dioxus-authored-mount-added.txt](dioxus-authored-mount-added.txt).

## Source-backed capability map

Dioxus has useful existing seams:

- `web/src/presentation/objects.rs` obtains a Case-mode flag but still builds the common layout tree (`tree::build_tree`); Case mode alters the physical-instance selector only. `web/src/presentation/objects/tree.rs` has Board/Layout/Matrix/Row/Column/Key/Component kinds and no Case assembly/body kinds.
- `web/src/cad_presentation.rs` owns the Case panel and places `MechanicalSettings` inside the central `Case settings` disclosure. `web/src/presentation.rs` separately mounts `CaseBodyInspector` in the Inspector slot. This explains the visible split: the authored editor exists in the Inspector while configuration is in the canvas panel.
- `web/src/presentation/case_controller.rs` projects the current scoped Case document and submits body edits through Runtime; `case_bodies.rs` includes AddBody, body fields, mount add/remove/type/position/diameter and gasket controls. Generated-stack mode intentionally replaces the authored editor with the “remain saved” message.
- `web/src/presentation/shared_viewer.rs` has private scene/view state, lifecycle guards, camera, picking, layer selection, display colors/visibility, render modes, assembled/exploded/section, orbit/zoom, and empty `handles`. The Case projection currently packages the selected board with empty `surfaces`, `holes`, and both board/global `models`; only `CadScene.result.bodies` becomes renderer body meshes/layers.
- `web/src/presentation/case_viewer.rs` guards interactions against scope/token/current scene. It accepts only PCB or generated body layer IDs. It maps a picked/layer ID to an authored Case body only when that ID is an exact current-document CaseBody ID for the selected board; otherwise it clears body selection. A generated CAD id not in the authored document therefore cannot select an authored editor row.
- `web/src/presentation/model_delivery.rs` contains private asset selection/decode/row-merge logic, but `web/src/presentation.rs` does not declare or call `model_delivery`. Its presence is not model-delivery proof. F7.3f/3g tickets explicitly retain the root-owned worker/Runtime/page-host/provider and public integration work.
- React source was pinned and inspected through CodeGraph in the original checkout. Relevant files are `app/src/ui/useCaseWorkspace.tsx`, `CaseInspectorPanel.tsx`, `MechanicalAssemblyPanel.tsx`, `AssemblyViewer.tsx`, `AssemblyScene.tsx`, `ModelPreviewBoundary.tsx`, `assemblyPreview.ts`, and `useAssemblyPreview.ts`.

Selected SHA-256 values (full manifest is next to this report):

| Source | SHA-256 |
|---|---|
| React `useCaseWorkspace.tsx` | `80cecb584fb2c6639fb5b7b7a11cd0b82aad13641e19139d5590fbca991c047e` |
| React `CaseInspectorPanel.tsx` | `051396a0e97254886fc9853d5d922f5e7409ee41a34876b57472fb02869d3eca` |
| React `MechanicalAssemblyPanel.tsx` | `89ef0de8e114fc4098df40ce175c31783850be27fcba4def0d0fd01a8d446ee4` |
| React `AssemblyScene.tsx` | `f0cb88c22e26c9788cfd9b10452ff40e88ff89d6944a6bba618068d247340416` |
| Dioxus `presentation/objects.rs` | `39954b6247b2f0529bd26a3206ad199afea060597242da2feb35a4011176d9d5` |
| Dioxus `presentation/objects/tree.rs` | `84cb7cbae6d37c24d90ffbd26db745b646197cdb0adf57a33f90cd3557563ea0` |
| Dioxus `presentation/case_bodies.rs` | `f848ddaf68c1e745440bafd34206a40c386940eb83ce7b1a773cdaa8252ae68f` |
| Dioxus `presentation/case_controller.rs` | `bc5ce15ce817090f46801c69057fd698fda741cef520a694475db44e852f7601` |
| Dioxus `presentation/case_viewer.rs` | `9648af96b0b68ce9cd669937eef3f791f6257ffa2e1f0aa4d5e6d2c18066a3ed` |
| Dioxus `presentation/shared_viewer.rs` | `d0dd3ba2e818ba1f730ce301bf5d356c06a0d9bceaaf8141b5cd1909b68e688b` |
| Dioxus `presentation/model_delivery.rs` (unregistered) | `303099456dd9c52195dfb0a476c1f407b821559b8280cd6c9154b6f49e6d7eae` |

## Ticket/dependency map and actionable slices

Current `.scratch/dioxus-frontend-v1/tasks.json` marks all F7.1–F7.8 planned. That metadata is not a source inventory: the current tree/body/mechanical/viewer modules and the captured UI demonstrate partial implementation. Keep parent completion and acceptance open.

- **F7.1** is the early private renderer reachability/contract decision. F7.3 and the shared-viewer Case child 02 are fixture-actionable after it. F7.1 is the only canonical start prerequisite for those slices; its review is not evidence that all viewer behaviors are wired.
- **F7.2 authored Case stack** starts after **INT.1** and can be demoed with saved fixtures. The body editor now has a public add/edit/mount path; the missing part in this candidate is the hierarchy/selection experience and complete parity proof, not lack of a Core body edit.
- **F7.4 mechanical configuration** also starts after **INT.1**; **INT.2** is its acceptance join. A high-value independent slice is to mount the existing `MechanicalSettingsMount` in the Case Inspector context and exercise scopes/history there, preserving the existing operations.
- **F7.3 shared viewer/model adapter** starts after F7.1. **INT.2 and BND.1 are acceptance joins, not start blockers**. Its public mesh inventory, model providers, selection mapping, and controls must be completed before claiming the shared contract.
- **F7.5** direct manipulation is explicitly after F7.3/F7.4. Do not front-load it: the current common projection has no handles and current Case projection does not expose all mapped model identities.
- **F7.6** generation/readiness/STEP starts after F7.2/F7.4, independent of F3/F4/F5 completion. Exact CAD generation is already visibly working in this candidate fixture; readiness edge cases, retry/scope staleness and Case export remain separate acceptance work.
- **F7.7** physical Case board/instance projection starts after F7.2/F7.3/F7.6; only its named F5 hardware/instance handoff (**F5.6**) is the acceptance join. Existing one-instance demo is insufficient to prove multi-instance behavior.
- **F7.8** is the downstream paired acceptance join after F7.3/F7.5/F7.6/F7.7, with acceptance joins F3.6, F4.4, F6C.5, and F2.3. Shared-viewer issues 03–06 split Layout, Keymap, Keycaps and Parts sample consumers; issues 07–08 split imported/generated board-model delivery. They preserve F7.3's INT.2/BND.1 joins and do not create new canonical blockers.
- The user's six workbench stream expectation is **Layout, PCB, Keymap, Keycaps, Case and Parts**. Current F7.8 task text enumerates only Layout, Parts, Keymap, Keycaps and Case. PCB cross-workspace acceptance appears instead under F5.8, which also joins F7.3/F7.7. Make the PCB consumer explicit in the reset acceptance matrix; this is an acceptance inventory gap, not a reason to delay fixture-backed F7 work.

Recommended independently demoable order, preserving current edges:

1. Case hierarchy + contextual body selection/Inspector (F7.2/UI; current edit port already exists), using a saved authored-body fixture and one generated-stack fixture.
2. Mechanical settings placement + supported configuration edits (F7.4/UI), with public save/undo/reopen and correct current physical-scope evidence; keep disable-stack behavior explicit.
3. Case-first shared viewer projection and model delivery (F7.3 / issues 02, 07, 08): board surfaces/holes, real `PcbModel` rows/assets, generated meshes, exact pick mapping, renderer lifecycle/retry. Reuse private host/Runtime boundaries and providers; no public API/visibility expansion is supported by this audit.
4. Case generation/readiness/STEP export (F7.6), paired status and exact export bytes. Current mesh rendering is a useful partial, not closure.
5. Supported mount/gasket direct manipulation (F7.5) once handles and exact map-back signals exist.
6. Physical board/instance Case source (F7.7), with a genuine multi-instance fixture and F5.6 join.
7. Paired common viewer acceptance across all six streams (F7.8 + F3.6/F4.4/F6C.5/F5.8/F2.3 joins), including canonical-vs-physical source identity and lifecycle/display/pick behavior. Keep export-to-project journeys for F8.6 separate.

## Verification boundary

This packet verifies source composition and a handful of visible public actions. It does **not** establish exact same-document parity, full mechanical stack parity, exports/STEP byte correctness, canvas mesh-pick selection, direct manipulation, board model delivery, multiple-instance switching, compact/mobile parity, dark-mode parity, keyboard/focus conformance, axe results, failure/retry, scope supersession, or cross-workspace acceptance. Those remain explicit downstream checks; do not report them as green based on the existing native tests or the one successful exact-CAD scene.
