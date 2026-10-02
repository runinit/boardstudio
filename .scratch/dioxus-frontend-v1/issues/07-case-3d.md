# F7 — Case and shared assembly viewer

**Triage:** ready-for-agent. Implementation/acceptance dispatch follows the [slice graph](../tasks.json) and [agent execution plan](../EXECUTION.md).

## Problem Statement

The Dioxus migration still lacks the React Case workspace and its shared 3D assembly viewer. Users cannot yet configure and inspect authored case bodies or a generated mechanical stack, make and preview supported mechanical edits, understand readiness and generation failures, or inspect the assembled physical board in 3D. Rebuilding only the visible panels would also leave Layout, Parts, Keymap, and Keycaps without the common viewer behavior those workflows use.

The source application has a critical scope distinction: in Case mode, the assembly is derived from the selected physical board instance; in Layout, Keymap, and Keycaps, the displayed assembly is the canonical board document. A migration that treats these documents interchangeably can show case geometry from the wrong instance or leak physical-instance edits into canonical board editing.

## Solution

Migrate Case editing, generation/readiness, and 3D inspection to Dioxus using the existing core, application/session, CAD, export, and renderer capabilities. Build one private common assembly/model-viewer adapter and one viewer implementation, then integrate its consumers from Layout, Parts, Keymap, Keycaps, and Case. Keep Case authored configuration, mechanical configuration, previews, and generation within the existing document/session authority and preserve board/instance scope.

Start immediately on saved-fixture forms and viewer-contract feasibility; these do not wait for the complete F3/F4/F5 workflows. Treat F3/F4/F6 consumer wiring and F5 physical-instance handoff as explicit integration joins. Reuse the existing M1 offline CAD, case preview/exact generation, cancellation, and STEP export foundation. Preserve current behavior through public UI actions and existing service boundaries. No backend rewrite or public API/visibility expansion is authorized by this plan. If F7.1 finds the existing renderer ABI insufficient for an accepted behavior, document the exact missing capability and obtain separate authorization before widening a public contract.

## User Stories

1. As a board designer, I want to see the case for the selected physical board instance, so that the 3D case represents the assembly I am configuring.
2. As a board designer, I want Layout, Keymap, and Keycaps 3D views to continue using the canonical board document, so that physical-instance case data does not alter those views.
3. As a board designer, I want to switch among case bodies in the selected board's authored stack, so that I can edit the intended plate, tray, lid, or other supported body.
4. As a board designer, I want to add and configure an authored case body, so that body type, thickness, clearance, z-offset, wall height, and wall thickness are saved with the board.
5. As a board designer, I want to configure mounting holes and bosses, including their locations and supported dimensions, so that the generated body has the intended attachment geometry.
6. As a board designer, I want to enable or remove a gasket channel and edit its supported inset, width, and depth, so that the authored case reflects the design I intend to generate.
7. As a board designer, I want authored case bodies and generated mechanical parts presented as distinct sources, so that generated geometry does not silently replace my authored design.
8. As a board designer, I want to choose among the supported construction methods, mount styles, shell/frame/plate arrangements, and inherited or custom part profiles, so that the mechanical stack reflects the fabrication approach I selected.
9. As a board designer, I want to edit supported plate, PCB, foam, bottom, wall, and clearance dimensions and see derived gaps and allowances, so that dependent dimensions remain understandable.
10. As a board designer, I want to inspect the resolved stack by layer and select a layer, so that I can understand how the configured parts fit together.
11. As a board designer, I want to define supported openings, battery envelopes and cable exits, mounting hardware, closure hardware, suspension mounts, and internal gasket settings, so that the mechanical configuration captures the assembly needs represented by the existing UI.
12. As a board designer, I want to adopt suggested mount locations and configure supported hardware or critical-fit constraints, so that candidate positions and fit limits are reviewable before generation.
13. As a board designer, I want supported per-part process overrides and stabilizer-fit choices, so that each represented part uses its intended manufacturing process and fit.
14. As a board designer, I want numeric edits to remain drafts until the current field's documented commit action, so that blur/Enter commits and Escape restores the accepted value without accidental intermediate edits.
15. As a board designer, I want committed edits to use normal edit history and undo/redo, so that Case work remains consistent with the application's existing document authority.
16. As a board designer, I want edits to be scoped to the current document, board, project session, and physical instance as applicable, so that switching scope cannot apply a stale draft to another assembly.
17. As a board designer, I want mechanical and authored-case findings summarized and navigable, so that I can locate and correct clearance, outline, and critical-fit problems.
18. As a board designer, I want to switch live preview on or off and request a manual preview update, so that I can balance responsive feedback with deliberate generation.
19. As a board designer, I want generation progress and preparing/running/ready/blocked/failed/cancelled states explained, so that I know whether displayed geometry is usable.
20. As a board designer, I want to cancel an in-progress preview or exact generation, so that obsolete work does not continue to occupy the workspace.
21. As a board designer, I want prior generated geometry retained with an explicit stale indication while a new result is pending or fails, so that I can compare safely without mistaking it for current output.
22. As a board designer, I want preview and exact generation to be revision- and scope-checked, so that delayed results cannot replace geometry for a newer edit, board, or instance.
23. As a board designer, I want export enabled only for current exact geometry and to see applicable warnings before export, so that exported STEP data corresponds to the accepted configuration.
24. As a board designer, I want the shared viewer to display PCB, mechanical parts, cases, available models, and keycaps with the existing shaded, wireframe, hybrid, and hidden-line modes, so that different design layers are inspectable.
25. As a board designer, I want to toggle supported assembly layers and individual available model instances, adjust their colors, and reset display colors, so that I can focus on relevant geometry without changing the design.
26. As a board designer, I want assembled, exploded, and section views with the supported section planes and position controls, so that I can inspect internal fit and assembly order.
27. As a board designer, I want camera fit, top, bottom, isometric, orbit, and zoom controls, so that I can navigate the same assembly from useful viewpoints.
28. As a board designer, I want to select rendered objects and map a selection to its corresponding project part, module, case body, or finding where that mapping exists, so that viewer inspection leads to the relevant editor or diagnostic.
29. As a board designer, I want supported mount/gasket preview handles, edits, and unlink actions to respect constraints and commit or cancel as one coherent operation, so that direct manipulation cannot leave partial invalid edits.
30. As a board designer, I want focus/highlight behavior to reveal the geometry associated with a selected finding, so that diagnostics remain useful when relevant layers are hidden.
31. As a board designer, I want saved and bundled 3D assets loaded with the existing STL/WRL renderer and STEP/CAD import paths, so that available model geometry remains visible without inventing a new asset format.
32. As a board designer, I want loading, stale-asset, unavailable-model, and rendering errors to identify the affected content and offer a retry when supported, so that a missing preview does not obscure the rest of the workspace.
33. As a board designer, I want the preview to report provider/model preview failure with its existing retry action; renderer failure remains an alert with clean stop and surrounding UI preserved and a path back to 2D, so that a failed canvas does not strand the Case workflow.
34. As a board designer, I want the viewer to respond to canvas resize, device pixel ratio, theme, and mount/unmount lifecycle changes, so that it stays legible and does not retain stale event handlers or GPU resources.
35. As a board designer, I want physical-instance case settings and previews to follow the selected instance across board changes, so that switching instances neither displays stale geometry nor loses correctly scoped saved settings.
36. As a board designer, I want absent or unsupported physical-instance data to produce a clear empty/setup state, so that the UI does not fabricate a mechanical assembly.
37. As a board designer, I want the common viewer contract consumed by Layout, Parts, Keymap, Keycaps, and Case, so that camera, layer, selection, error, and lifecycle behavior remains consistent across workflows.
38. As a keyboard designer, I want the Case toolbar's existing construction/case iconography retained, so that supported construction concepts are recognizable without introducing a new case-choice wizard.
39. As a board designer, I want the workspace usable in compact layouts, with keyboard-accessible form controls, visible focus, and readable error/status feedback, so that Case work remains practical at the supported viewport sizes and input modes.

## Implementation Decisions

- F7 owns the Case workspace's private Dioxus modules and exactly one common assembly/model-viewer adapter and viewer implementation. F3 Layout, F4 Parts, and F6 Keymap/Keycaps consume that viewer; they do not create parallel viewers. F7 owns Case consumer integration. The coordinator owns the application shell, shared runtime, global CSS, shared design tokens, and whole-app build integration.
- Preserve the source application's authority split: Case assembly input is the selected physical-instance case document; Layout, Keymap, and Keycaps assembly input is the canonical board document. Every request, result, selection, draft, display preference, and cancellation is associated with its actual document/board/instance/revision scope.
- The common viewer adapter translates typed application/CAD/asset inputs to the renderer's internal scene input and maps supported user interactions back to existing application operations. Keep renderer DTO details private. Prefer the existing wasm renderer capabilities and existing Rust wrapper boundary. Do not widen public interfaces or expose private scene contracts as a shortcut.
- F7.1 explicitly verifies whether the current private boundary can support the required scene updates, state/display controls, camera operations, picking, direct-manipulation handles, asset decoding, and deterministic lifecycle. `RendererHost` mount/update-scene/camera methods are not themselves a complete viewer facade. The underlying wasm Renderer exports additional operations, while the host wrapper does not currently expose all state/interaction/model behavior. Classify adapter implementation versus genuine missing public contract at this seam before implementation. Any genuine public-contract/visibility change is a separately authorized dependency; backend/CAD rewrites are out of scope.
- Case forms use existing core edits (`SetCase`, `SetMechanical`), session preview/commit/history, mechanical profile resolution, CAD generation, and STEP export paths. Keep a single accepted document/session authority; do not create duplicate frontend persistence or history.
- Keep transient numeric edits as scope-keyed drafts. Commit on the source UI's blur/Enter behavior, discard the field draft on Escape, and route a committed edit once through the ordinary session history. Cancel outstanding operations and suppress stale replies when identity, revision, or current scope changes.
- Keep authored case bodies distinct from the generated mechanical stack. Preserve the selected-board guard, mechanical board association, and preview revision checks. `CaseChoice` currently supplies construction/case iconography rather than a choice wizard; migrate the helper's represented visuals only.
- Preserve generation semantics: live preview is optional; manual Update Preview is available; preview and exact work use the existing runtime/CAD job flow; progress, cancellation, blocked/failed/cancelled states and retry affordances are explicit; stale prior geometry is visibly stale; STEP export requires current exact geometry and follows existing warning/readiness rules.
- Preserve the actual supported mechanical configuration represented by the current UI, including construction/mount/stack styles, part profiles and supported KiCad geometry extraction, dimensions and clearances, openings/battery envelope, mounting and closure hardware, suspension/gasket configuration, process overrides, stabilizer fit, resolved layer selection, suggested mounts, diagnostics, and the explicit disable-stack path. Do not add unrepresented settings or imply unsupported profile/hardware behavior.
- Viewer controls preserve existing render modes, section/explosion modes, layer and model visibility, layer colors/reset, camera presets/orbit/zoom/fit, object picking, finding focus, supported gasket/mount edits, light/dark scene palette, and unavailable-model/error feedback. Lighting remains renderer/theme behavior; there are no user lighting controls to add.
- The common viewer handles canvas resize/DPR, context loss, initialization/update errors, retry, event capture, superseded scene work, teardown, and renderer/GPU disposal as one lifecycle. Fallback UI offers the existing route to 2D and does not take down surrounding forms.
- Work can proceed against accepted saved fixtures before F3/F4/F5 are complete: F7.1 feasibility, Case editors, common viewer foundation, and generation against fixture/current services. F3/F4/F6 consumer adoption and F5 physical-instance readiness are separate joins, not blanket blockers for fixture-backed work.
- The common AssemblyViewer/scene/model-preview contract and implementation are owned by F7.3, with F7.1 reserved for early contract/feasibility. The viewer is common implementation; consumer-specific scope projection and UI control placement remain with their owning workflow.

## Testing Decisions

- Test observable public UI behavior using saved fixtures and existing accepted application services; compare the same user action, visible state, and resulting accepted document/generation/export outcome as the React source. Avoid tests coupled only to private adapter structure or internal renderer DTO layout.
- Preserve/port the existing Case and viewer public-UI scenarios: manual and live generation, cancellation, stale/current readiness and export gating, warnings, findings, gasket edits, camera/view controls, mechanical configuration, physical scope switching, model loading/retry, and preview fallback. Reuse prior-art cases represented by manual-generation, mechanical-assembly, assembly-camera, live-case-preview, internal-gasket, case-workbench-readiness, assembly-preview, and Case generation polish workflows.
- Exercise the common viewer through each real consumer (Layout, Parts, Keymap, Keycaps, Case) using public actions and verify shared camera/layer/render/error/lifecycle outcomes. Do not duplicate a viewer-only fake for each feature.
- Include saved-fixture coverage before F3/F4/F5 joins. Add explicit integration coverage for canonical-board versus physical-instance Case input, F5 handoff, scope change during pending generation, and each consumer's assembly projection once those joins are available.
- Verify no stale asynchronous asset/CAD/generation response replaces a newer scope or revision; cancel and unmount during work; resize and context-loss/retry; preserve surrounding Case form interaction on viewer failure.
- Preserve keyboard accessibility and compact layout checks at public UI level: tab/focus order, Escape draft rollback, Enter/blur commit, accessible control names, readable progress/errors, and no canvas-only route to critical actions.

## Out of Scope

- Rewriting Rust core, CAD generation, session authority, offline runtime, or STEP export foundations.
- Widening public Rust/application API visibility or publishing renderer-private scene/interaction DTOs without a separate explicit approval.
- A new renderer, asset format, lighting-control panel, construction-method wizard, or CaseChoice picker; unsupported mechanical options/profiles/settings; new hardware capability assumptions.
- Replacing F3 Layout, F4 Parts, or F6 Keymap/Keycaps workflows. Their consumer-specific integration is in scope only for wiring to the one F7-owned viewer and completing agreed joins.
- Owning the shared shell, global runtime composition, global CSS, shared design tokens, or whole-app build orchestration.
- Making complete F3/F4/F5 feature completion a prerequisite for fixture-backed F7 form or viewer work. Only their explicit consumer/hardware integration joins depend on those workflows.
- Exporting 3D or making physical hardware claims beyond the current STEP and accepted renderer/CAD behavior.

## Further Notes

### Work sequencing and dependency boundaries

- F7.1: early renderer/viewer contract and feasibility. Keep it short and decision-focused. It resolves which required viewer operations can be implemented through existing crate-private/wrapper and wasm bindings. If a public boundary really blocks parity, specify the missing contract separately and stop short of treating its widening as approved.
- F7.2 and F7.4: authored case stack and mechanical configuration forms can proceed in parallel on saved fixtures and existing services. They are not blocked on completed F3/F4/F5.
- F7.3: one shared viewer/model-preview adapter and implementation. F3/F4/F6 consume its agreed contract; F7.3 is the intended integration dependency, not a requirement that they wait before beginning their own fixture-backed work.
- F7.5: supported direct manipulation for case gasket/mount preview, after the viewer interaction seam is established.
- F7.6: generation/readiness/export orchestration uses the existing M1 CAD/offline/STEP foundation and the accepted Case settings/session services.
- F7.7: physical board/instance projection and F5 hardware handoff. Only the hardware/instance-dependent join waits on F5; local form/viewer/generation work remains independently actionable.
- F7.8: paired cross-workflow acceptance and migration completion, including adoption by F3/F4/F6 and stable regression behavior.

### Source evidence and ownership map

- React Case/workspace source: `app/src/ui/CaseChoice.tsx`, `CaseGenerationControls.tsx`, `CaseInspectorPanel.tsx`, `MechanicalAssemblyPanel.tsx`, `useCaseWorkspace.tsx` and its supporting hooks/controllers.
- React common-viewer source: `app/src/ui/AssemblyViewer.tsx`, `AssemblyScene.tsx`, `ModelPreviewBoundary.tsx`, `app/src/assemblyPreview.ts`, `app/src/useAssemblyPreview.ts`, and `Workbench.tsx` for the canonical-versus-physical assembly document selection.
- Existing Dioxus/runtime boundary: `web/src/renderer_host.rs`, `web/src/runtime.rs`, `web/src/cad_jobs.rs`, `web/src/cad_presentation.rs`, `application/src/session.rs`, wasm renderer exports in `renderer/src/wasm.rs`, and existing public core edit/configuration contracts.
- Existing test prior art: public UI/E2E workflows for manual generation, mechanical assembly, camera controls, live case/mount preview, internal gasket, Case readiness, assembly preview, generation polish, and stabilizer/case findings; UI component tests for AssemblyScene mounts/controls, mechanical configuration, model preview fallback, and live generation.
- The first renderer investigation found `RendererHost` currently provides mounting, full scene update, resize/DPR, fit/view/orbit/zoom, teardown and context-loss reporting. It does not itself expose the full state/display, picking, direct-manipulation, or model-preview facade. The underlying wasm `Renderer` exports more operations (including state, picking, handles, and prepared scene entry points). Therefore the immediate known work is a private adapter/wrapper feasibility question, not proof that CAD/core public contracts are missing. F7.1 records any actual public-contract gap precisely.
- M1's offline CAD generation and STEP export foundation is accepted reusable behavior. F7 owns React-parity Dioxus presentation and integration around it, not a second CAD/export implementation.

### Independent review reconciliation

Renderer wasm already exports setState, setHandles, pick, prepared-scene and camera operations; the known work is private host wrapping, not a demonstrated public API gap. New feature-only wrapper methods must stay crate-private because web::renderer_host is publicly exported. F7.1 should map and proceed using these operations, escalating only a concrete proven insufficiency. Parts supplies an isolated sample project. F4 retains reusable PartDefinition mechanical-profile authoring; F7.4 owns case-configuration profile assignment/overrides/extraction. Provider/model retry and ModelPreviewBoundary retry are preserved; renderer initialization/context-loss failure requires alert/status, clean stop/unmount and usable surrounding controls, not a newly invented canvas retry button. Keycap solids/inlays are a separate known missing Rust-host handoff: BND.1 must establish reuse of the existing reference CAD service for that subflow; core ResolveKeycaps and Case STEP alone are insufficient.

### Verified keycap adapter boundary

The follow-up source audit found the already exported Rust WASM `build_keycaps` function in `cad/wasm/src/model/keycaps.rs`, re-exported by the CAD package already copied into the Dioxus build. The gap is the Dioxus private host/worker request path, not keycap geometry or STEP engine support. BND.1 proves a crate-private adapter from existing `KeycapSpec` to this export, with stable cap/legend body IDs, revision/scope validation and preview chunks/yields matching the reference. Do not bundle the React UI/client runtime or add a new CAD engine API. Existing public `web::cad_jobs` enum/request types must not be widened implicitly; prefer private wire/host types. Preview cancellation can be observed between chunks; synchronous STEP kernel calls cannot be interrupted mid-call, so cancel/scope change suppresses late delivery. Keycap export resolves with `cases: null` as in the reference. The [boundary report](../evidence/planning/boundary-gaps.md) records the exact source trace and implementation limits.


### Authoritative execution dependencies

The milestone-level prerequisites above describe integration context. The refined rows below replace whole-milestone or symbolic dependencies. Preparation/fixture work may start after `Start after`; completion also requires `Acceptance joins`. Existing F1/F3a source and evidence are baseline prerequisites, not tasks to repeat. Full workflow qualification also joins F2 shared panels/controls under F9.

| Slice | Start after | Acceptance joins |
| --- | --- | --- |
| F7.1 | Baseline; preparation may start | Own slice acceptance |
| F7.2 | INT.1 | Own slice acceptance |
| F7.3 | F7.1 | INT.2, BND.1 |
| F7.4 | INT.1 | INT.2 |
| F7.5 | F7.3, F7.4 | Own slice acceptance |
| F7.6 | F7.2, F7.4 | Own slice acceptance |
| F7.7 | F7.2, F7.3, F7.6 | F5.6 |
| F7.8 | F7.3, F7.5, F7.6, F7.7 | F3.6, F4.4, F6C.5, F2.3 |

### Refactoring observation handoff

Update the [living RF register](../refactor-findings.json) and [post-port takeaways](../../../docs/migration/POST-PORT-REFACTOR.md) for architectural, design, theoretical or quality issues discovered in this slice, or record “No new refactoring takeaway observed” with reviewed scope. Distinguish confirmed findings from hypotheses; include evidence, impact, current mitigation, later proposal and validation. This does not authorize unrelated refactoring or defer required parity fixes.

### Crate boundary checkpoint

The page binary's Runtime/presentation and the publicly imported `boardstudio_web` library are separate crate boundaries. A `pub(crate)` method added to the library cannot be called by that binary. F7.1/INT.1 must choose private wrapper placement in the consumer's crate or an existing sufficient public facade; prove the call path before declaring the adapter ready. Do not silently make methods public to bypass this constraint. See RF-002 in the post-port register.
