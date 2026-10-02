# F7 Case-first shared viewer: private root mounting contract

**Purpose.** This is an integration contract for mounting the already source-reviewed page-private shared viewer in the Case panel. It records what the current sources establish and the root-owned joins still needed. It does not claim a completed mount, a successful page build, or five-consumer parity.

## Source set and status

The integration tree inspected was `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001` at `566e180fcffab6ba2a78535a60c1089c2ac0a279`. Relevant content hashes at inspection:

| Source | Blob |
| --- | --- |
| `web/src/presentation/shared_viewer.rs` | `af644e88122009bd23644688261a34562f04dbab` |
| `web/src/runtime.rs` | `7b3758c9a11db84139961022d60b6bbf78686d67` |
| `web/src/cad_presentation.rs` | `fa5bafb3af98d78856167ff1039a815d8fcd85fe` |
| `web/src/presentation.rs` | `e9d5a0a005c4bd0c5b16f29357115cd76d15d7f4` |
| `web/src/presentation/case_controller.rs` | `d2d5be31548e1cf22cad6ba6eb5f85ca9706b1aa` |
| `web/src/renderer_host_page.rs` | `f12f70c20400950c63c439f52571f3d7725a45d6` |
| `renderer/src/wasm.rs` | `8bd994be42610e653534b83f3ce280ae8a963a78` |
| `app/src/ui/caseDisplay.ts` | `fd782612cde5eeb6aab473cfc6f4db0c3365ba03` |
| `app/src/ui/Workbench.tsx` | `1cee2d12656f67f04f61d2ed91d3503ec7bd8c37` |

The shared-viewer worker source is at `cdbac52219666c7547d787ed08c1ba31f2ea649d`. Its Spec and Standards source reviews are clear (`/tmp/frontend-run/f7-case-viewer-final-spec-review.md`, `/tmp/frontend-run/f7-case-viewer-final-standards-review.md`). This means source review only: the integration tree has the viewer source but does not yet declare `presentation::shared_viewer` or mount it from `CasePanel`. The existing Case route still uses `CaseCanvas` and public `RendererHost`.

Root has confirmed an integration `rustc` failure on the private page snapshot path; the author’s private page snapshot correction is pending. This is distinct from the unchanged renderer-host baseline, which remains byte-exact. No compile-green claim is made here, and this contract did not run Cargo. Full Case rendering, browser behavior, and public-layer gates remain open.

## Recommended private mount seam

Register the page-only viewer module under private `web/src/presentation` and the private page renderer host in the web binary, then replace only the Case canvas child in `CasePanel` with `CaseSharedViewer`. Keep the current Case settings form, generation/cancel controls, status text, and Inspector/body-editor route in their existing owners. Do not re-export through a public library facade or change Session, schema, or public member visibility.

The existing private entry point is `CaseSharedViewer(scene: Rc<CadScene>, selected_layer: String, display: CaseDisplay, resolved_theme: String, on_signal: EventHandler<ScopedViewerSignal>, on_display_change: EventHandler<ScopedDisplayChange>)`. It uses the page-only `RendererPageHost`, accepts a typed immutable `Rc<CadScene>`, and keeps renderer/transient camera state inside the viewer. `CaseSharedViewer` already checks scope, accepted snapshot token, and pointer identity against the live Runtime scene before host updates. Its signal owner additionally binds callbacks to `Scope`, snapshot token, viewer instance, projection generation, and renderer sequence.

## Callback admission contract

`ScopedViewerSignal::is_current()` and `ScopedDisplayChange::is_current()` are necessary owner-lifetime checks; they are not sufficient admission checks by themselves. Root callbacks should admit an event only when all of the following still hold at callback time:

1. The scoped wrapper reports current, and its captured `ViewerIdentity` equals the currently mounted viewer identity.
2. `runtime.scope() == Some(identity.scope)` and the accepted snapshot still has `identity.snapshot_token`.
3. `runtime.cad_scene()` returns a scene whose `Rc` is pointer-equal to the captured scene and whose full `Scope` and token still match. `Runtime::cad_scene()` alone filters Scope but does not filter the accepted token; check both token and exact `Rc`.
4. The current `InstanceSelection` preference is resolved/current for the fresh `ReadModel` (`InstanceSelection::is_current`), and any callback-specific body/layer target is still a member of that live board projection.
5. For a picked renderer object, the viewer has accepted/applied the exact corresponding renderer identity. The wrapper’s identity already carries a monotonically advanced viewer-instance/projection-generation/renderer-sequence tuple; do not replace this with document revision alone.

On Scope/token/Rc/instance mismatch, ignore the callback and let the existing root scope reconciliation invalidate private selection context, anchor eligibility, and stale interactions. Do not invent a way to mutate Session’s internal selection anchor. The persistent display key deliberately excludes `SessionEpoch` and viewer generations; callback tokens must include them.

## Domain IDs and owner separation

The current Case projection builds layer options from the literal renderer layer ID `"pcb"` plus each actual `scene.result.bodies[*].id` and name. Renderer Case body mesh objects preserve those IDs. The renderer separately uses the synthetic object ID `"pcb-selection"` only for PCB selection highlighting; it is not a layer or domain ID. The renderer’s `selected_layer` compares body selections to those renderer body IDs and compares PCB selection to `"pcb"`.

`CadScene.result.bodies` is the renderer mesh identity source. `CaseBody.id` is the authored Case body identity in the accepted document. Their equality/mapping must be established against the fresh captured Case document before forwarding a picked ID to Case body UI selection. Do not infer either ID is a `ProjectDoc.parts` ID. `Session::SelectParts` accepts real document part IDs and remains authoritative for those only; the Case scene currently exposes no verified mapping from its generated mesh IDs to `ProjectDoc.parts`.

Root should own two distinct ephemeral UI values, both scoped and revalidated:

- **Mechanical layer selection:** the selected renderer layer ID (`"pcb"` or an ID present in the current renderer layer projection), used for renderer highlight and mechanical-layer controls.
- **Authored Case-body selection:** an optional current-board `CaseBody.id`, used to synchronize with body Inspector/navigation. The current `CaseBodyInspector` owns its selection locally, so viewer-pick synchronization needs an explicit private root signal/prop join; do not create a second durable document or Session selection.

Neither is the persistent display preference nor `ReadModel.selected_part_ids`. Keep edit drafts and edit requests in the existing Case body controller.

## Persistent display and theme

React’s current persistence key is `boardstudio:case-display:${projectId}:${displayKey}`, where `displayKey` is the selected Case instance ID when present, otherwise selected board ID (`Workbench.tsx` and `useCaseWorkspace`). The root-owned Dioxus storage key should preserve that document-plus-effective-instance-or-board policy, without `SessionEpoch`, snapshot token, viewer instance, or projection generation. Persist only `{ hidden, colors }`; retain a usable in-memory value if browser storage is unavailable. Apply the renderer’s existing preference ID expansion consistently (`pcb` expands to PCB/Models/Keycaps/Copper/Mask/Silkscreen, `gaskets` to `Gaskets`, and gasket upper/lower aliases together). This first Case slice currently has no gasket or model geometry, so those preference aliases do not establish rendered feature availability.

`ThemeState` currently stores only the preference (`light`, `dark`, or `system`). `App` separately owns a reactive system-theme signal updated by `matchMedia` and applies the resolved value to document `data-theme`. The viewer needs the actual resolved `"light"` or `"dark"` value, derived reactively from both signals. Do not pass the literal preference `"system"` or sample `data-theme`; the DOM write is an effect, not the authoritative reactive input.

## Narrow first slice and limits

The present Case projection is intentionally narrow and grounded in the typed scene: board contours/thickness, the current CAD result body meshes, and the optional mechanical stack. It explicitly sends empty `surfaces`, `holes`, and `models`, supplies no renderer handles, and uses full-scene updates. Therefore this slice can prove a shared private host mount for generated Case geometry and camera/display controls, but cannot claim populated PCB surfaces, holes, library models, battery/reference overlays, editable handles, or parity with the five React consumers. Do not fabricate renderer handles or substitute inferred geometry.

Preserve the existing Case generation/settings behavior and the stale-scene status. A stale scene may remain visible for camera inspection, but its picks, layer/body selection, display writes, and edit actions must fail the live Scope/token/Rc/instance admission checks above. Existing renderer-host baseline and public layer remain unchanged.

## Open root joins

- Integrate the private page snapshot correction and obtain the root-owned strict page build; this contract contains no compiler result.
- Register/mount the source-reviewed private viewer in the Case route, preserving forms, generation status, and Inspector ownership.
- Add the private reactive resolved-theme and persisted display owners.
- Decide and implement the private mechanical-layer and authored-CaseBody selection bridge, validating IDs against the current scene and captured document before use.
- Verify scope/token/`Rc`/instance admission at every parent callback and retain the viewer’s owner identity checks; test stale callback rejection and actual current-Case mount in the root verification stream.

The Case-first mount does not need to wait for parity across all consumers. It is a narrow architectural integration step, with the above root joins and the reported page snapshot correction remaining explicit prerequisites/evidence gates.
