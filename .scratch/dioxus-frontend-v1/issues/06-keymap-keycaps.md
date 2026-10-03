# F6: Keymap and Keycaps Dioxus workflows

**Triage:** ready-for-agent. Implementation/acceptance dispatch follows the [slice graph](../tasks.json) and [agent execution plan](../EXECUTION.md).

**Purpose:** plan the remaining React-to-Dioxus frontend work for Keymap and Keycaps. This is a presentation migration over existing document, core, CAD, renderer, firmware, and export contracts. It does not authorize backend rewrites, persisted-schema changes, broad public API changes, firmware capability expansion, or a production writer cutover. A dedicated `CadWorker::request_keycaps_preview` page/library entry point is authorized for the current F6C.5 slice because the page binary and `boardstudio_web` are separate crates; its keycap wire stays private and existing generic CAD request/operation contracts remain unchanged.

**Reference:** the pinned React UI at `5a472a9426e6e38993361da402cd4ec730feb369`; F6 source ledger is the `05-keymap-and-keycaps` inventory group. **Starting Dioxus executable:** `f44a3d1b`. **Parent scope:** 100% existing React TSX/UI behavior parity.

## Problem Statement

The Dioxus workbench currently has a recognizable shell and a verified Layout correction, but Keymap and Keycaps are still placeholders. A designer cannot configure layered key behavior, macros, encoder actions, physical keycap profiles and legends, inspect clearance findings, or move between the 2D authoring view and the existing 3D keycap assembly presentation while retaining the project's normal save and history behavior.

The React reference already drives these workflows through existing Rust edit, resolution, firmware, CAD, renderer, persistence, and export services. The remaining work is to port its controls, projections, drafts, focus, visual states, and service orchestration without duplicating domain authority or treating missing private UI adapters as justification for new public APIs.

## Solution

Port Keymap and Keycaps as separate, reviewable Dioxus slices over the accepted project snapshot and existing commands. Keep Keymap responsible for logical bindings and firmware handoff; keep Keycaps responsible for physical profile, legend, color, size, fit, and keycap export presentation. Use stable saved key/layer/macro/encoder IDs, the existing edit/history path, and existing Rust resolution and output providers. Reuse the F7-owned assembly viewer for 3D presentation; F6 supplies keycap inputs and controls but does not build a second viewer.

F6 work may proceed against saved fixtures and existing core/CAD services while F3/F4/F5/F7 integrations are incomplete. Overall acceptance waits only for the specific handoffs listed below: accepted Layout selection/projection, the PCB physical-input handoff for hardware-dependent encoder controls, and the shared F7 assembly viewer for 3D parity. Those handoffs do not block isolated editor slices or saved-fixture tests.

## User Stories

1. As a keyboard designer, I want the Keymap workspace to show the selected board's supported switch and press inputs, so that I can assign behavior to the same stable objects used by Layout and PCB.
2. As a keyboard designer, I want to select an input on the canvas or from a searchable list, so that I can edit it without losing its identity when layers or workspaces change.
3. As a keyboard designer, I want to see and choose saved layers by name, so that I can author behavior without confusing persistent layer IDs with their display order.
4. As a keyboard designer, I want to add, rename, and remove non-base layers within the existing limits, so that I can express alternate keyboard modes safely.
5. As a keyboard designer, I want higher layers to display their transparent fall-through behavior, so that I can understand which binding will be used.
6. As a keyboard designer, I want a searchable keycode chooser and the existing free-form valid keycode entry, so that I can find common keys while retaining supported ZMK expressions.
7. As a keyboard designer, I want to choose key press, mod-tap, layer-tap, momentary, toggle, to-layer, sticky-layer, sticky-key, macro, transparent, or unassigned behavior, so that I can use every behavior accepted by the current core contract.
8. As a keyboard designer, I want modifier and tap fields only when their selected behavior uses them, so that each binding editor presents relevant controls.
9. As a keyboard designer, I want layer and macro references to remain tied to stable IDs, so that renaming or reordering display labels does not silently retarget bindings.
10. As a keyboard designer, I want to see the active layer's binding summary on the 2D keys, so that I can scan a layer without opening each editor.
11. As a keyboard designer, I want to create, rename, edit, and remove macros with tap, press, release, and wait steps, so that I can represent the supported sequences in the current keymap contract.
12. As a keyboard designer, I want macro timing, name, step, count, and nested-call limits to produce clear validation feedback, so that invalid keymaps are not presented as successful exports.
13. As a keyboard designer, I want to assign clockwise and counterclockwise actions to each supported encoder per layer, so that physical rotary inputs can use the same supported binding editor.
14. As a keyboard designer, I want encoder push bindings only when PCB reports a push input, so that the interface follows actual hardware handoff data.
15. As a keyboard designer, I want the PCB wiring panel's legacy firmware-position controls to remain usable, so that a designer can keep using the established quick assignment flow while detailed layered editing is available in Keymap.
16. As a keyboard designer, I want legacy base-layer key bindings to appear and export with their current meaning, so that opening and saving older projects does not erase established behavior.
17. As a keyboard designer, I want unassigned and transparent bindings to remain distinct, so that fall-through behavior is not confused with an empty firmware position.
18. As a keyboard designer, I want every accepted edit to participate in the existing Undo, Redo, save, reload, archive, and workspace-switch flows, so that Keymap does not create a second document history.
19. As a keyboard designer, I want invalid edits and service failures to retain the last accepted value and show an actionable error, so that I can recover without silent data loss.
20. As a keyboard designer, I want to export the same ZMK source package using the existing firmware provider, so that the UI does not generate or rewrite firmware semantics itself.
21. As a keyboard designer, I want the keymap export to explain when controller, wiring, scan, encoder-driver, or other existing qualification is missing, so that a visible export action does not imply unsupported hardware is ready.
22. As a keyboard designer, I want Keycaps to show board-wide keycap and legend colors, so that the keycap presentation has consistent defaults.
23. As a keyboard designer, I want to choose a supported keycap profile and socket per matrix, so that a matrix uses the intended nominal keycap shape and switch fit family.
24. As a keyboard designer, I want to configure matrix profile row and wall thickness, so that the existing Rust resolver can produce the same nominal cap specifications.
25. As a keyboard designer, I want to override a selected key's profile, socket, row, dimensions, color, or legend, so that unusual and standalone inputs can retain individual settings.
26. As a keyboard designer, I want a key's legend to follow its binding by default, so that common key labels stay in sync with Keymap.
27. As a keyboard designer, I want to explicitly restore a binding-derived legend or save a blank legend, so that inheritance and an intentional empty cap remain distinguishable.
28. As a keyboard designer, I want to see each supported key's 2D cap size, color, legend, orientation, and selection, so that physical appearance remains inspectable while editing.
29. As a keyboard designer, I want the shared width/depth controls to support mixed selections, keyboard and pointer commits, and linked halves, so that sizing retains the Layout workflow and a single undoable edit.
30. As a keyboard designer, I want neighboring keys to reflow when resizing would overlap them, so that the existing geometric behavior and board outline remain coherent.
31. As a keyboard designer, I want keycap fit findings to identify the affected keys and current case bodies/features, so that I can navigate from a warning to its target.
32. As a keyboard designer, I want fit status to say when case geometry is stale or when a clearance is only a conservative envelope check, so that the interface does not imply physical certification.
33. As a keyboard designer, I want the 3D assembly view to display generated caps and letter inlays in the shared assembly viewer, so that keycap settings can be reviewed with the existing renderer and assembly layers.
34. As a keyboard designer, I want stale or cancelled keycap preview results discarded and current failures to expose retry, so that an older request cannot replace my current project view.
35. As a keyboard designer, I want to export keycap STEP using existing Rust resolution and CAD construction, so that exported geometry corresponds to the current accepted project revision.
36. As a keyboard designer, I want both workspaces to retain the application theme, compact-panel, focus, and keyboard interaction language, so that they remain usable in the same workbench.
37. As a keyboard designer, I want empty projects and unsupported inputs to explain the next supported action, so that unfinished hardware or absent switches do not look like a broken editor.
38. As a reviewer, I want paired public browser scenarios to compare React and Dioxus edits, saved documents, errors, history, previews, and exported artifacts, so that F6 acceptance demonstrates workflow parity rather than component presence.

## Implementation Decisions

- Dioxus presentation reads accepted immutable project snapshots. Durable keymap and keycap changes use the existing session edit and history flow. Presentation-only state is limited to selected workspace section, active displayed layer, search text, input drafts, focus, and view state.
- The existing keymap edit contract remains authoritative for typed bindings, layer changes, macro changes, encoder actions, and validation. The existing legacy firmware-position edit remains authoritative for its board-scoped binding map and compatibility merge behavior. Do not normalize old documents by inventing a replacement keymap.
- Persist and resolve layer, macro, key, and encoder references by their existing IDs; display indices are derived only for ordering and export formatting. Preserve transparent, none, missing, and legacy values as distinct supported states.
- Use only the currently accepted binding families and macro step forms. Preserve existing core limits and errors; do not add behavior families, firmware device connectivity, unsupported macro nesting, or new keycode parsing rules.
- Keep keymap's active-layer 2D labels distinct from Keycaps' physical legend/color/size view. Both use the same selected key identity and current board projection.
- Matrix profile defaults and per-key overrides continue to use the existing keycap settings contract. Null values mean inherit where the Rust contract currently defines inheritance; an empty legend remains an intentional blank.
- Width/depth controls port the existing TypeScript draft-and-commit and linked-layout reflow policy into a private Rust frontend controller. This policy is not an already callable Rust service. Submit the result once through the current undo path; do not introduce a new resize engine or persist UI slider drafts.
- Keycap findings and specifications come from the existing keycap resolver. The reference CAD service constructs keycap solids/inlays and STEP; current Rust web CAD operations do not expose that keycap request, so the private retained-service bridge must be established before this part can be accepted. F6 presentation must show pending, stale, unsupported, invalid, and error states truthfully.
- Dioxus needs private UI adapters for keymap read projection, keycap resolution/preview, and the firmware export flow. They may compose existing public core/CAD/renderer/host contracts, but must not add or widen public application/core APIs or duplicate engine state. Keep request identity, revision, scope, cancellation, and late-result rejection in those adapters.
- Firmware position controls remain embedded in the F5-owned PCB wiring presentation. F6 owns the migrated control component and its behavior; F5 owns the wiring panel, electrical-plan data, and where the component is mounted. Acceptance of this integrated slice requires the agreed F5 handoff, but the editor can be implemented and tested with a saved fixture before that handoff.
- The 3D assembly viewer, camera, picking, assembly layer controls, and renderer lifecycle remain F7-owned. F6 supplies keycap settings/resolution and the Keycaps workspace's 2D/3D view choice through that shared viewer. Do not duplicate `AssemblyViewer`, scene ownership, worker lifetime, or camera state. The 3D part of F6 acceptance joins after the F7 viewer interface is available.
- Export buttons use existing firmware and keycap STEP providers, with accepted snapshot/revision checks and existing browser delivery semantics. The full Export workspace is F8-owned; F6 only ports these established local actions and their readiness/error behavior.
- Keep visual styling inside the coordinator-owned shell/shared tokens. F6 owners may add only feature-local presentation styles and must hand shell, global CSS tokens, runtime wiring, and shared compact-panel changes to the coordinator rather than editing them in parallel.

## Testing Decisions

- Use the highest agreed seam: public browser workflows against the same saved fixture and action sequence in the pinned React reference and Dioxus candidate. Compare visible states, accepted document/settings, selection, revision/history, reload/archive behavior, resolver/export outputs, failure states, and downloads. Tests should assert user-visible behavior and resulting public data, not Dioxus component internals.
- Preserve existing core characterization as domain-oracle evidence rather than rewriting those tests: keymap edits/history and legacy retention, keymap validation/export workflows, keycap edit/resolution and fit findings, and electrical/encoder firmware behavior. F6 does not change the core algorithms or their accepted output.
- Retain/transcribe the existing React browser scenarios for layer and mod-tap editing, macro steps, encoder rotation/push, Undo/Redo, reload, ZMK archive contents, keycap profile/color/legend inheritance, 2D/3D preview, STEP export, narrow layout, and mirrored key sizing/reflow.
- Add Dioxus browser checks at the public workbench: mouse and keyboard key selection, key/layer navigation, applicable-field switching, draft commit/cancel on blur/selection/workspace change, visible focus and restored focus, compact panel operation, light/dark/system, empty project, validation failure, export readiness/error, and no page errors.
- Exercise async boundaries with current/stale revisions and project/board changes: delayed keycap resolution, cancellation before CAD, late CAD replies, failed previews/retry, export input mutation before delivery, and teardown. Reuse existing preview/export seams; add no renderer test seam solely for F6.
- Validate outputs through existing providers: supported binding source and macro/encoder output remain equivalent; keycap specifications and findings remain equivalent; preview bodies and STEP remain valid and revision-matched. A ZMK package generation check is not a claim that the external ZMK toolchain was installed or a physical keycap fit was proven.
- Acceptance includes paired desktop and compact captures for both themes and populated selected/loading/error states, browser accessibility checks with visible/focusable labels, and affected native/WASM/frontend/build/repo checks. Keep actual assistive-technology limitations as a carried gate rather than treating automated checks as screen-reader acceptance.

## Out of Scope

- Rust core, application-session, document format, generated contract, or CAD algorithm changes; broad public API/schema/member visibility changes; new persistence fields; and a second writable store. The explicitly authorized dedicated `CadWorker` bridge is the narrow F6C.5 exception.
- New ZMK behaviors, keycode language, macro nesting, external-device pairing, live firmware, physical hardware communication, or claims of external firmware compilation/flash success.
- New keycap profiles/dimensions, physical certification, stabilizer generation, CAD geometry changes, or changes to existing fit assumptions and diagnostics.
- A second 3D viewer, new camera/renderer ownership, or F7's Case/assembly workspace and assembly viewer implementation.
- F5's PCB/wiring/electrical-plan UI and hardware planning algorithms; F8's complete Export workspace; and full F9 React retirement, entrypoint switch, production publication, or cutover.
- Blocking all independent F6 work on the completion of F3, F4, F5, or F7 as whole milestones. Only the affected integration acceptance requires each named handoff.

## Further Notes

### Task and ownership map

The private owners below are proposed Rust/Dioxus module boundaries for independent slices; exact internal filenames are implementation choices, not APIs. The coordinator owns root presentation composition, Runtime/CoreWorker changes that cross slices, global CSS tokens, shared shell/compact drawers, and final integration. Feature owners should expose small private component/data functions within `presentation`, not public crate interfaces.

| Task | Dioxus private owner | Exact React responsibility |
| --- | --- | --- |
| F6K.1 Keymap read projection, layer list, selected-key navigation, active-layer 2D view | `web/src/presentation/keymap_view.rs`, `web/src/presentation/keymap_panel.rs` | `app/src/ui/createKeymapWorkspace.tsx`, `app/src/ui/KeymapPanel.tsx` (layer and key sections), `app/src/ui/KeymapLayout.tsx` (Keymap mode) |
| F6K.2 Binding editor and behavior-specific fields | `web/src/presentation/keymap_binding.rs` | `app/src/ui/KeyBindingEditor.tsx`, `app/src/ui/keyBindingChoices.ts` |
| F6K.3 Macro editing and validation presentation | `web/src/presentation/keymap_macros.rs` | `app/src/ui/KeymapPanel.tsx` macro section |
| F6K.4 Encoder editor, firmware-position control, and ZMK handoff | `web/src/presentation/keymap_hardware.rs`, with private calls to existing host/core/export adapters | `app/src/ui/KeymapPanel.tsx` encoder section; `app/src/ui/FirmwareKeymapPanel.tsx`; F5's `app/src/ui/WiringPanel.tsx` mount/handoff |
| F6C.1 Keycaps read projection, key selection, physical 2D canvas | `web/src/presentation/keycaps_view.rs` | `app/src/ui/createKeymapWorkspace.tsx`, `app/src/ui/KeymapLayout.tsx` (Keycaps mode) |
| F6C.2 Board/matrix/per-key profile, legend, color, socket, row and units controls | `web/src/presentation/keycaps_panel.rs` | `app/src/ui/KeycapPanel.tsx`, `app/src/ui/keymap.css` feature rules |
| F6C.3 Shared key-size drafts and linked resize/reflow integration | `web/src/presentation/key_size_controls.rs` plus private Layout callback owned in the F3 integration slice | `app/src/ui/KeySizeControls.tsx`, `app/src/ui/MatrixInspectorPanel.tsx`, `app/src/ui/Workbench.tsx` resize call site, `app/src/ui/planKeycapResize.ts`, `app/src/ui/keycapReflow.ts` |
| F6C.4 Fit resolution, finding list/navigation, stale-case state | `web/src/presentation/keycaps_fit.rs` with existing keycap resolution request and accepted scene/finding navigation | `app/src/ui/KeycapPanel.tsx` findings section and `app/src/assemblyPreview.ts` keycap-resolution state |
| F6C.5 Keycap 3D toggle, preview, retry, and STEP action integration | Keycaps consumer in `web/src/presentation/layout_viewer.rs` plus private `CadWorker`/Runtime bridge; shared renderer remains F7-owned | `app/src/ui/Workbench.tsx` Keycaps view toggle; `app/src/ui/AssemblyViewer.tsx`; `app/src/assemblyPreview.ts`; `app/src/exports/keycaps.ts` |

### Existing contracts and gaps

- Existing edit operations are `EditKeymap`, `SetKeyBinding`, `SetKeycapBoard`, `SetMatrixKeycaps`, and `SetKeycapKey`; they merge fields in the current document, retain unrelated saved fields, validate through existing core logic, and use normal Undo/Redo. Existing read/output calls are `ResolveKeycaps` and `GenerateFirmware`, with firmware requiring the existing electrical/hardware inputs. The existing reference CaseClient.keycaps service is separate from core resolution; the Rust web CAD operation set currently has no equivalent keycap request. ResolveKeycaps alone cannot construct solids or STEP.
- Existing `KeymapConfiguration` stores layer IDs, bindings, sensors and macros. Supported bindings are key-press, mod-tap, layer-tap, momentary-layer, toggle-layer, to-layer, sticky-layer, sticky-key, macro, transparent, and none. Existing macro steps are tap, press, release, and wait; nested macros are rejected. The keycode validator accepts the existing bounded nested modifier expressions. The UI must mirror the current contracts and errors, not broaden them.
- Existing Keycaps profiles are Cherry, OEM, DCS, DSA, SA, Hi-Pro, G20, and Choc; mounts are MX, Choc v1, Choc v2, and Alps. Matrix defaults and key overrides include profile, mount, first row/wall thickness or per-key row/units/color/legend. Board defaults include keycap color, legend color, and clearance. Rust resolution owns nominal specifications, switch-family matching, legend inheritance, geometry validation, conservative rotated swept-envelope clearance against neighboring keys and current prepared case geometry, and warnings/errors.
- Keycaps fit already resolves through the accepted-snapshot Runtime adapter. The shared Keycaps 3D consumer reuses that exact accepted `KeycapResolution` and its specs; it does not issue a duplicate `ResolveKeycaps` query. The preview uses the packaged `build_keycaps` service through a dedicated narrow worker facade and current scope/revision checks. Keymap's shared 3D consumer remains open. Firmware UI/export still needs private adapters around existing electrical and firmware requests plus current artifact delivery. No new Core query, session event, generated contract, persisted schema, or generic CAD request field is introduced by this preview.
- F5 owns and hands off selected-board electrical assignments, resolved supported encoder/push inputs, legacy `keyBindings`, readiness and diagnostics. F6K.4 can be developed against the agreed fixture before the F5 UI is ready, but its integrated behavior cannot be accepted until the F5 handoff is real.
- F7 owns the shared 3D viewer, renderer lifecycle, camera, picking, scene layers and assembly controls. F6C.5 connects the Keycaps route to that owner and supplies accepted `KeycapSpec` preview bodies/inlays. It does not copy assembly renderer/viewer code or camera state. The feature-specific CAD worker is separately owned by Runtime and cannot cancel Case generation or STEP export. Each synchronous keycap kernel batch is non-preemptible, but preview work yields between groups of eight so cancellation can be observed before the next group; source changes, supersession, unmount and worker termination reject or suppress late results. STEP remains one synchronous kernel call and only suppresses late delivery.
- F3 owns Layout matrix selection and direct placement. F6C.3 supplies the existing shared key-size interaction semantics through that path; resize preserves linked halves and neighboring reflow using the established operation/history behavior. A F3 integration hook is a task dependency for the full resize acceptance only.
- `FirmwareKeymapPanel` is currently hosted inside the React PCB Wiring panel and edits the legacy board-scoped binding map. The typed layered Keymap editor is a separate surface. Preserve both surfaces and the current core compatibility merge; do not collapse them into one UI model during migration.
- Existing test seams are agreed: public edit/history/export APIs and end-to-end browser workflows against the pinned reference, shared saved fixtures, current app regression scenarios, existing `AssemblyPreview` stale/cancel seam, and keycap resolver/CAD provider outputs. No question or new public test seam is required for planning.

### Ordered task records

Machine-readable task records: [workflow JSON](../workflows/F6.json); the [combined graph](../tasks.json) is authoritative for exact start and acceptance dependencies. Task acceptance is slice-scoped; the whole F6 ticket stays open until all applicable dependency joins and evidence are complete.

### Coordinator reconciliation: remaining TypeScript interaction policy

`app/src/ui/planKeycapResize.ts` and `app/src/ui/keycapReflow.ts` currently implement selected-cell sizing, canonical linked-half deduplication, axis projection, row/column spacing, and selection-center preservation in TypeScript. A targeted Rust source search found no equivalent callable resize/reflow controller. F6C.3 must port this existing frontend interaction policy into a private Rust controller, reuse the accepted matrix projection/core edit authority, and commit once through the existing replace-document path. It must not retain the TS controller at v1 or claim that calling `SetKeycapKey` alone implements Layout sizing/reflow. Preserve the existing helper regression cases and compare resulting matrices, key envelopes, placement offsets and linked halves through public browser/save/Undo scenarios. Any ownership or public API change beyond this behavior-preserving private controller is an explicit boundary decision, not assumed backend work.

### Independent review reconciliation

F6C.5 keycap preview and STEP reuse the existing packaged keycap CAD provider proven by BND.1. This source slice adds one dedicated public `CadWorker::request_keycaps_preview` method across the actual page/library crate boundary; request wire data remains private, and public `CadRequest`, `CadOperation`, Core contracts, CAD algorithms and renderer exports are unchanged. The preview routes only the Keycaps workspace today; Keymap's 3D consumer and the remaining stale/error/lifecycle browser journeys remain open. Do not mark BND.1, F7.3 or F6C.5 complete from this source slice. F6K.1/2 and F6C.1/2 remain independently actionable in 2D.

### Verified keycap adapter boundary

The follow-up source audit found the already exported Rust WASM `build_keycaps` function in `cad/wasm/src/model/keycaps.rs`, re-exported by the CAD package already copied into the Dioxus build. The gap is the Dioxus host/worker request path, not keycap geometry or STEP engine support. The existing dedicated `CadWorker` facade carries `KeycapSpec` through private wire data and preserves request/job/scope/revision correlation; this one feature method is public only because the page and `boardstudio_web` are separate crates. The initial adapter invoked one kernel preview for the full spec list and therefore did not match React's groups-of-eight yield/cancellation policy. The bounded BND.1 continuation now calls the same packaged export in groups of eight, retains stable cap/legend IDs and checks each returned batch revision before combining bodies. Do not bundle the React UI/client runtime or add a generic CAD request field or engine API. Each batch and the complete STEP call remain synchronous; preview cancellation is observed between batches, while STEP cancellation/scope change suppresses late delivery. Keycap export resolves with `cases: null` as in the reference. The [boundary report](../evidence/planning/boundary-gaps.md) records the exact source trace and implementation limits.


### Authoritative execution dependencies

The milestone-level prerequisites above describe integration context. The refined rows below replace whole-milestone or symbolic dependencies. Preparation/fixture work may start after `Start after`; completion also requires `Acceptance joins`. Existing F1/F3a source and evidence are baseline prerequisites, not tasks to repeat. Full workflow qualification also joins F2 shared panels/controls under F9.

| Slice | Start after | Acceptance joins |
| --- | --- | --- |
| F6K.1 | INT.1 | F3.1 |
| F6K.2 | F6K.1 | Own slice acceptance |
| F6K.3 | F6K.1, F6K.2 | INT.2 |
| F6K.4 | F6K.1, F6K.2 | F5.2, F8.2 |
| F6C.1 | INT.1 | F3.1 |
| F6C.2 | F6C.1 | Own slice acceptance |
| F6C.3 | F6C.1 | F3.2, F3.5 |
| F6C.4 | F6C.2 | INT.2 |
| F6C.5 | F6C.4, F7.1 | F7.3, F8.2, BND.1 |
| F6.6 | F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.3, F6C.4, F6C.5 | F2.3, F5.2, F7.6 |

### F6C.5 source packet — Keycaps 3D preview

The Keycaps route now passes its current `KeycapsFitState` to the existing canonical Layout viewer. It reuses the already accepted Core resolution/specifications, requests cap and legend meshes from the packaged `build_keycaps(export: false)` provider, applies each spec's cap/legend color, and adds the body identities to the existing shared assembly layers. A private Runtime-owned CAD worker is independent of Case generation and STEP export. Accepted scope/token/revision and worker generation are checked before and after awaits; source change, route unmount and supersession close the worker and prevent late mesh publication. `build_keycaps` executes synchronously inside the worker and cannot cooperatively yield during the kernel call. The first served journey on `243aa551` exposed a mounted-effect loop: its local generation signal was read reactively while being incremented, repeatedly cancelling preview work and leaving the shared canvas blank. Reading that effect-owned counter with `.peek()` fixed the loop. The exact pinned fixture replay passed on `e7742d46`: the shared 3D preview rendered, the global Keycaps group and separate SW1 cap/legend rows hid and restored their bodies, and the SW1 legend/color plus matrix and per-key profile inputs were visible as expected. The browser receipt and captures are [here](../evidence/keycaps-3d-preview-20261003/RECEIPT.md). This is one bounded Keycaps-route journey; Keymap consumption and remaining lifecycle and full F6C.5 acceptance remain open.

The isolated source check used strict page WASM all-target Clippy with `-D warnings`. This packet is not an integrated/browser acceptance: Keymap's 3D consumer, paired changed-preview journey, cancellation/error browser branches, and F7.3/F8.2/BND.1 joins remain open. No F6C.5/F7.3/BND.1 completion or dependency waiver is claimed. The only API boundary addition is the dedicated `CadWorker::request_keycaps_preview` method required because the page binary and `boardstudio_web` are separate crates; its wire remains private and generic CAD request/operation types are unchanged. Root authorized this narrow exception for this work.

#### Keymap shared 3D consumer follow-up

The pinned React Keymap→3D journey on the retained c6 fixture showed generated keycaps in the shared assembly viewer: the layer list contains the global Keycaps group, individual cap rows, and a separate SW1 legend row after its base binding is `A`. The Dioxus Keymap 3D mount passes the same accepted `KeycapsFitState` used by Keycaps to `LayoutCanonicalViewer`, reusing accepted specs and the existing independent CAD worker. It does not issue another Core resolution or change the shared viewer, scene, camera, or worker owner. Paired React/Dioxus browser proof is recorded in [`paired-receipt.md`](../evidence/keymap-3d-consumer-20261003/paired-receipt.md) against 34759/source `04c85b88`. The changed consumer is qualified on that candidate; the existing cancellation/error/F7.3/F8.2/BND.1 acceptance joins remain open.

The bounded F6C.2 settings reconciliation now maps the board-color, matrix row/wall/socket, key/board ownership, rejected-value, and null-versus-empty persistence behavior to current paired evidence in [`f6c2-criteria-reconciliation.md`](../evidence/keycaps-3d-preview-20261003/f6c2-criteria-reconciliation.md) and [`f6c2-settings-receipt.md`](../evidence/keycaps-3d-preview-20261003/f6c2-settings-receipt.md). The browser input/change path is qualified for colors, not the host native picker. React's attempted legend Redo showed `History is empty`, but the sequence did not establish that the legend edit entered command history, so this is diagnostic only. Keep F6C.2 and the broader F6 parent criteria open; do not turn the bounded receipt into automatic acceptance.

The current F6C.3 selected-linked-key journey is recorded in [`paired-linked-resize.md`](../evidence/keycaps-size-reflow-20261003/paired-linked-resize.md). It exercises one selected linked key through the public Layout Inspector in both pinned apps, confirms mirrored size/reflow, visible remaining-overlap warning, Undo/Redo and reload. Both-halves-selected behavior and the F3.2/F3.5 acceptance joins remain open; do not mark F6C.3 complete from this bounded route.

### Refactoring observation handoff

Update the [living RF register](../refactor-findings.json) and [post-port takeaways](../../../docs/migration/POST-PORT-REFACTOR.md) for architectural, design, theoretical or quality issues discovered in this slice, or record “No new refactoring takeaway observed” with reviewed scope. Distinguish confirmed findings from hypotheses; include evidence, impact, current mitigation, later proposal and validation. This does not authorize unrelated refactoring or defer required parity fixes.
