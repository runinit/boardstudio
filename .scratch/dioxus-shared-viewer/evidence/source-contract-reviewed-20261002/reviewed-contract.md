# 01: Prove the private shared viewer contract and scope inputs

**Parent:** F7.1 — Common viewer contract and renderer feasibility.

**What to build:** Establish one source-backed private contract for the shared 3D viewer so Layout, Parts, Keymap, Keycaps and Case can use the same scene, camera and picking behavior. The handoff identifies callable renderer operations, required scene/state inputs, lifecycle/error handling and the distinction between canonical-board scenes and Case's selected physical-instance scene. It chooses a reachable private wrapper placement in the page binary or an already sufficient public facade, with no API or visibility expansion.

**Blocked by:** None (F7.1 has no canonical start blockers).

**Status:** source contract prepared; independent F7.1 review and slice acceptance remain open.

- [x] Compare the pinned React viewer behavior with the current Dioxus renderer host and the existing Rust WASM renderer exports for scene replacement, display state, camera presets/orbit/zoom/fit, handles, picking, model decoding, lifecycle, context loss and error recovery. Record each supported operation and any adapter-only gap; identify any true contract gap precisely.
- [x] Demonstrate the actual crate call path. The page binary and `boardstudio_web` library are separate crates, so `pub(crate)` library methods are unreachable from the binary. Record the selected same-crate private wrapper location or cite the sufficient existing public call path; do not widen visibility.
- [x] Define the minimum private input/output contract for all five consumers, including canonical-board inputs for Layout/Keymap/Keycaps and the selected physical-instance projection for Case. Keep CAD generation, workspace feature behavior and cloned viewer implementations out of this slice.
- [x] Publish a concrete decision and acceptance evidence for F7.3 and consumer integration. Reuse existing sufficient evidence and identify exactly what must be exercised by those later tasks; do not claim the shared viewer itself is implemented.
- [x] Record “No new refactoring takeaway observed” for the inspected renderer/host boundary, or add source-backed evidence to existing RF-002/RF-012 without proposing an unapproved API change.

## Source-grounded contract decision

### Decision

Proceed with F7.3 as private page-binary adapter and one shared viewer. The current
WASM renderer already exports the required scene, display, camera, picking,
handle, and STL/WRL decode operations. The demonstrated gap is that the current
`RendererHost` exposes only part of those operations, accepts untyped `JsValue`
scene input, and currently routes full updates through `setScene`; this is
private adapter/host mapping work. No missing public Rust/application operation
or approved visibility expansion has been demonstrated.

The page binary is the consumer boundary. `web/src/main.rs` declares the
page-owned `presentation`, `cad_presentation`, and `runtime` modules. Its Case
caller currently imports the already public
`boardstudio_web::renderer_host::RendererHost`, whose library module is exported
from `web/src/lib.rs`. This is a reachable existing public path for the host's
current `mount`, full-scene update, camera, and disposal methods. The binary and
library remain separate crates: a future `pub(crate)` method on the library
version is not callable by the binary.

For the missing operations, F7.3 should use one source-owned host implementation
in the page binary rather than widen the library surface or clone the renderer.
The concrete placement is a private `renderer_host` module included from
`web/src/main.rs` using the existing `web/src/renderer_host.rs` source; its
page-binary-only operations can be visible to the private page modules while
remaining non-public in the library crate. The shared view/consumer adapter
belongs under `web/src/presentation/` and calls that private host. Existing
library exports remain unchanged. F7.3 must route page-binary callers to this
private module when using its added operations; the existing external-library
path remains sufficient only for its current methods. This placement is a
source-level decision; F7.3 must prove it with the actual page-binary build and
must not infer reachability from a library-only build.

### Operation map

| Behavior | Existing source operation | F7.3 work / limit |
| --- | --- | --- |
| Mount, scene replacement, stale update result | WASM `Renderer::setScene`, `setPreparedScene`, and `setPreparedScenePatch` parse scene/revision data and return `false` for stale revisions (`renderer/src/wasm.rs`, exports `setScene`, `setPreparedScene`, `setPreparedScenePatch`). `RendererHost::mount` initializes the JS module, creates the renderer, sets the initial scene and fits; `update_scene` calls `setScene` (`web/src/renderer_host.rs`). | Build one private typed consumer-to-scene projection and allocate a strictly increasing renderer scene sequence independent of source document/provider/generation revisions and viewer-instance/projection identity. Route accepted/false-stale/error results back only to the matching current owner. Existing host `update_scene` neither exposes prepared-scene/patch calls nor preserves the renderer's boolean result (it returns `Result<(), String>`). The React worker/prepared-scene path is prior art, not a requirement to copy its transport implementation. |
| Display state and object/layer visibility | WASM `setState` accepts hidden groups/IDs, selected layer, view, mode, theme, explosion amount, section plane/position, plane visibility, hidden-line display, and color overrides (`DisplayState` and `apply_state` in `renderer/src/wasm.rs`). | `RendererHost` has no state method. Add private mapping; keep view-preference persistence with the consumer that owns it. Do not invent user lighting controls. |
| Camera fit, presets, orbit, zoom | WASM exports `fit`, `view` (`top`, `bottom`, `isometric`, fallback fit), `orbit`, and `zoom`; `RendererHost` already maps and schedules these operations. | Reuse one host mapping. Keep camera interaction and camera-preservation policy scoped to the current scene. |
| Picking and world point | WASM `pick(x, y)` returns an optional rendered object ID; `pointOnPlane(x, y, z)` returns a 3D point or an empty buffer when the ray is parallel. | Host currently exposes neither. Add a private input/output mapping. Convert canvas coordinates to renderer pixel coordinates once; consumers map object IDs to domain selections. Empty pick is valid and does not mutate selection. |
| Direct manipulation handles | WASM `setHandles` accepts stable handle IDs, position, tangent/normal, length, z and optional invalid state; it returns the number of newly uploaded meshes. `pick` considers visible handles before scene objects. | Host currently exposes neither. Add private handle and pick/plane adapters. Case owns gesture constraints, draft/commit/cancel and history through existing operations; the renderer only displays and identifies handles. |
| Model decode and scene preparation | WASM module exports `decodeStl`, `decodeWrl`, and `prepareScene`; the renderer also accepts prepared scenes/patches. | Host currently exposes no module decode/prepare operation. Reuse the shipped renderer artifact and existing model/asset providers. Keep asset lookup, retry, cancellation and scope/revision acceptance in their existing provider/application owners; do not add an asset format or duplicate CAD/model providers. |
| Resize, DPR, frame scheduling, teardown | `RendererHost::mount` installs `ResizeObserver`, window resize and DPR listeners; schedules one-shot animation frames; `dispose` removes listeners, cancels the frame, calls renderer `dispose`/`free`, and releases WebGL context. `Drop` also disposes. | Reuse this owner. Each mounted viewer has one host lifetime; all callbacks/results must be rejected after its captured consumer scope is obsolete. |
| Initialization/update error, context loss | Mount returns a string error and disposes partial initialization; `status` reports lifecycle/render/cleanup failures. `webglcontextlost` stops frames and reports failure; render submission failure also stops the loop. | Preserve the caller's visible failure/status and surrounding form controls, and offer the existing 2D route. This host does not implement context restoration or a renderer retry button; context loss requires remount through the owning UI. Provider/model retry remains the provider's existing action. |

The renderer API is dynamically invoked by `RendererHost` through reflective
method names. F7.3 should check the private mapping against the exported names
and supported inputs above. Do not treat a type/string adapter gap as proof that
the renderer engine or public application contract is missing a capability.

### Minimum private consumer contract

The adapter keeps renderer DTOs private. Its input is an immutable scene
projection plus the existing `application::Scope` (session epoch, document ID,
board ID, optional physical-instance ID), a private viewer-instance/projection
generation, and a renderer scene sequence. These identities answer different
questions and must not be substituted for one another:

- **Application scope** prevents work crossing sessions, documents, boards, or
  physical instances.
- **Viewer-instance/projection generation** prevents callbacks from an obsolete
  mounted consumer/projection being accepted when application scope is
  unchanged. Increment/change it when a semantic projection input changes;
  Parts must include the selected definition, companion set/order, placements,
  side, and rotation. Remounting also gives callbacks a fresh instance token.
- **Renderer scene sequence** is strictly increasing for calls to the
  renderer's full/prepared/patch scene operations. It is independent of source
  document revision, provider scene revision, and generation revision. A copied
  or reused source revision must not make an older renderer scene appear newer;
  the current renderer stale check rejects only `incoming < current`
  (`renderer/src/lib.rs::is_stale_scene_revision`).

Each async completion, event handler, pick result, and handle gesture captures
the identities relevant to its request and is accepted only if they still
match the active viewer. Source/document/provider/generation revisions are
validated independently where the existing producer exposes them. In
particular, `sampleAssembly` fixes the sample project/board IDs and copies the
source document revision while changing sample content with definition,
companions, placements, side, or rotation. Therefore `Scope` plus document
revision cannot identify a Parts preview; never use its sample IDs or copied
revision as a live domain identity or renderer sequence (`app/src/ui/sampleAssembly.ts:16–28,55–64`).

The five projections are:

| Consumer | Scene source and scope | Interaction mapping |
| --- | --- | --- |
| Layout | The accepted canonical document and selected board's board/model/contour preview. Preserve the React conditional Case overlays too: when `caseDocument === document`, pass the available authored/generated Case bodies, prepared Case scene, generation state, and (when `generatedCase`) mechanical assembly (`app/src/ui/Workbench.tsx:420–425,1386`, `showCaseGeometry`/`assemblyDocument` projection); when a separate physical Case document is selected, do not attach that other document's overlays to the canonical scene. | Map picked render IDs to existing board parts, modules, or supported Case objects; updates remain in Layout's current session. |
| Parts | The isolated sample project produced by `sampleAssembly` with `sample-board`, not the live project document. Scope plus copied document revision is insufficient here; the private projection generation changes with selected definition, companion set/order, placement, side, or rotation. | Camera navigation and preview status are useful; a sample ID must never select/edit a live-project part. Keep 2D/3D choice and footprint visibility with Parts. |
| Keymap | The same accepted canonical document/selected-board scene as Layout, including those conditional Case overlays only when `caseDocument === document` (`app/src/ui/Workbench.tsx:420–425,1386`). | Keep Keymap selection and keycap/layout metadata owned by Keymap; only current scoped IDs can return from picking. |
| Keycaps | The same accepted canonical document/selected-board scene as Layout, including only keycap meshes available from the current preview result and the conditional Case overlays only when `caseDocument === document` (`app/src/ui/Workbench.tsx:420–425,1386`). | Keep keycap placement/selection owned by Keycaps; missing or stale preview geometry is a preview status, not a fabricated renderer object. |
| Case | `physicalCaseDocument` when supplied (otherwise the current document), with its matching selected board/scene and selected physical instance. The display identity is instance-scoped, falling back to board ID when there is no selected instance. | Map IDs to current project parts/modules/case bodies/findings only where a current mapping exists. Supported handle gestures call existing Case draft/edit/session paths; they never edit the canonical Layout scene by alias. |

The common output is limited to accepted/stale/error status, renderer lifecycle
status, optional picked object ID, optional world point for a handle gesture,
and gesture start/move/end/cancel signals tagged with the captured application
scope and viewer-instance/projection generation. The consumer performs domain
mapping and submits existing operations. Parts sample IDs remain sample-local.
A missing mapping is an empty optional result, not a generated placeholder
domain ID.

### Evidence and acceptance boundary

This contract is based on source inspection at continuation HEAD
`1574d15766d6abe9c02ba1efa89b3ec09ebd5bc6` and pinned React reference
`5a472a9426e6e38993361da402cd4ec730feb369`. Inspected sources: React `AssemblyViewer.tsx`,
`AssemblyScene.tsx`, `renderClient.ts`, `useAssemblyPreview.ts`,
`assemblyPreview.ts`, `LibraryWorkspace.tsx`, `Workbench.tsx`, and
`useCaseWorkspace.tsx`; Dioxus `main.rs`, `presentation.rs`,
`cad_presentation.rs`, `renderer_host.rs`; renderer exports in `renderer/src/wasm.rs`;
and `application/src/session.rs` `Scope`. The exact initial Case call is in
`cad_presentation.rs::CaseCanvas`; the React shared view is `AssemblyViewer` →
`AssemblyScene`, called for Design/Keymap/Keycaps/Case by `Workbench` and for
isolated Parts samples by `LibraryWorkspace`.

No renderer code, API visibility, DTO, build entrypoint, or consumer behavior
was changed by this contract task. No compiler, browser, assistive-technology,
or public-UI evidence is claimed. F7.3 must verify the chosen private module by
building the actual `boardstudio-web` page binary, then exercise mount, full and
stale scene updates, all mapped state/camera/pick/handle/decode paths, scope
switch during pending work, resize/DPR, mount/unmount cleanup, initialization
failure, render failure, context loss, 2D fallback, and provider retry through
the integrated public UI. Paired consumer evidence must cover the five rows
above; saved-fixture Case work remains actionable before INT.2 and BND.1.

No new refactoring takeaway was observed in this source review. The wrapper's
partial reflective capability map and crate boundary are already recorded by
RF-012 and RF-002 respectively (`docs/migration/POST-PORT-REFACTOR.md` and
`.scratch/dioxus-frontend-v1/refactor-findings.json`). This decision does not
authorize a post-port redesign.

**Review status:** the source contract is ready for dedicated independent Astra
review; no self-approval or F7.1 slice acceptance is implied.

**Parent graph:** Canonical F7.1 `Start after: none`; `Acceptance joins: none`. F7.3 starts after F7.1. Its canonical `Acceptance joins` are INT.2 and BND.1; they do not block fixture-backed F7.3 implementation. All other F7 joins remain unchanged.

**Suggested routing:** Luna High author and verifier; dedicated Astra review because the task establishes a renderer boundary used by several workflows.

### Current implementation reachability — 2026-10-03

On packaged source `733c1da2abede39a617d2eca2e42d9bd437cea41`, `web/src/main.rs` declares private page modules `presentation` and `renderer_host_page`; `presentation/shared_viewer.rs` imports `crate::renderer_host_page::RendererPageHost`. The latter includes the page-owned base/extensions, so this consumer does not attempt to call a library-only `pub(crate)` method. The existing `frontend-module-attachment-repair-20261003` package proof at `.scratch/dioxus-frontend-v1/evidence/frontend-module-attachment-repair-20261003/package-proof.json` records the successful page compiler/package for these inputs. This confirms F7.1-C06's actual same-crate reachability; it does not qualify other viewer behavior or accept F7.1 as a whole.
