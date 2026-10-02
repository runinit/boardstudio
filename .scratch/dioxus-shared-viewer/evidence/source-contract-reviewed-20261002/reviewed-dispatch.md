# Dispatch contract — F7.1 common viewer

## Ownership and seam

No canonical start blockers. Before any implementation work, map the pinned React viewer and current Dioxus renderer host to the existing Rust WASM renderer exports. Prove the actual consumer crate call path. The page binary and `boardstudio_web` library are separate crates; a `pub(crate)` library method is unreachable from the page binary. Select a page-binary private wrapper placement or cite an already sufficient public call path. Do not widen API or member visibility.

Define the minimum private inputs/outputs and lifecycle/error behavior for Layout, Parts, Keymap, Keycaps, and Case. Canonical-board scenes are used by Layout/Keymap/Keycaps; Case may supply its selected physical-instance scene. Keep one renderer/viewer implementation. F7.1 is a contract and feasibility decision, not an alternate consumer implementation or CAD task.

## Source and behavior

Use `.scratch/dioxus-frontend-v1/tasks.json`, `workflows/F7.json`, and `issues/07-case-3d.md` as parent authorities. Compare React `AssemblyViewer.tsx`, `Workbench.tsx`, `useCaseWorkspace.tsx`, and `CaseInspectorPanel.tsx` with `web/src/renderer_host.rs` and existing `renderer/src/wasm.rs` exports. Account for scene replacement, state/display, camera presets/orbit/zoom/fit, handles, picking, model decoding, lifecycle, context loss, and error recovery. Classify each item as an existing operation, private host mapping/adapter work, or a precisely demonstrated public-contract gap.

Record the chosen reachable wrapper location, operation map, scope identity and scene invariants, supported consumer interactions, lifecycle/failure handling, and exact evidence that F7.3 and later consumer integrations need. Do not prescribe a new engine function, public facade, format, or visibility change. If a current exported operation is insufficient, record the smallest concrete gap and affected consumer; do not assume it.

## Prepared source decision (2026-10-02)

The contract is recorded in [`issues/01-common-viewer-contract.md`](issues/01-common-viewer-contract.md).

- **Decision:** use one common viewer and a private page-binary adapter. Existing renderer WASM exports cover the requested operation set. The gap is host mapping: `RendererHost` currently exposes mount/full scene update/camera/disposal but not display state, handles, picking, plane coordinates, or model decode/prepare; its full-scene method also discards the renderer's accepted/stale boolean.
- **Reachability:** the current Case caller reaches the existing public library host through `boardstudio_web::renderer_host::RendererHost`. The page binary and library remain separate crate roots, so `pub(crate)` library additions cannot serve the binary. F7.3's private page host is planned as a private binary module sourced from the existing `web/src/renderer_host.rs`; the shared adapter/view belongs under `web/src/presentation/`. F7.3 must build the actual binary and demonstrate the module/call path before it claims integration.
- **Scope and ordering:** guard callbacks/results with `application::Scope` plus a private viewer-instance/projection generation. This extra generation is necessary for Parts: `sampleAssembly` reuses `sample-pcb`, `sample-board`, `sample-N` IDs, and copies source revision even when definition, companion set/order, placement, side, or rotation changes. Allocate a separate strictly increasing renderer scene sequence; do not reuse document, provider-scene, generation, or projection revisions for the renderer's stale-scene guard. Gate same-scope stale async/status/pick/gesture completions too.
- **Consumer projection:** Layout, Keymap, and Keycaps use the canonical board projection and preserve React's conditional authored/generated Case overlays when `caseDocument === document`; generated mechanical assembly is included only when `generatedCase` is true. They must not attach a different physical Case document's overlays to the canonical scene. Parts uses `sampleAssembly`'s isolated sample project; Case uses the selected physical-instance document/projection when available and never aliases it to the canonical scene.
- **No widening:** renderer DTOs remain private; no renderer/application API, file format, provider, CAD implementation, or build entrypoint changed as part of F7.1 documentation.
- **Evidence boundary:** source mapping only. No compiler, browser, public UI, or assistive-technology result is claimed. F7.3's exact downstream verification list is in the issue. No new refactoring takeaway was observed; existing RF-002/RF-012 cover crate placement and the partial reflective host mapping.
- **Review:** prepared for dedicated Astra review. This authored decision is not self-approved and does not close F7.1 slice acceptance.

## Graph and handoff

F7.1 `start_after: []`; `acceptance_after: []`. F7.3 starts after F7.1 and retains INT.2 and BND.1 as acceptance joins. The fixture-backed F7.3 implementation remains actionable before those joins; all other F7 dependencies remain unchanged. This planning publication does not close F7.1 or F7.3 acceptance.

Use shared acceptance at `../dioxus-frontend-tranche-1/ACCEPTANCE.md`, including relevant public-route, source, lifecycle, accessibility, and independent review gates. Record “No new refactoring takeaway observed”; existing RF-002/RF-012 already capture the renderer host and crate boundary unless new source evidence changes that conclusion.
