# Dispatch contract — F7.1 common viewer

## Ownership and seam

No canonical start blockers. Before any implementation work, map the pinned React viewer and current Dioxus renderer host to the existing Rust WASM renderer exports. Prove the actual consumer crate call path. The page binary and `boardstudio_web` library are separate crates; a `pub(crate)` library method is unreachable from the page binary. Select a page-binary private wrapper placement or cite an already sufficient public call path. Do not widen API or member visibility.

Define the minimum private inputs/outputs and lifecycle/error behavior for Layout, Parts, Keymap, Keycaps, and Case. Canonical-board scenes are used by Layout/Keymap/Keycaps; Case may supply its selected physical-instance scene. Keep one renderer/viewer implementation. F7.1 is a contract and feasibility decision, not an alternate consumer implementation or CAD task.

## Source and behavior

Use `.scratch/dioxus-frontend-v1/tasks.json`, `workflows/F7.json`, and `issues/07-case-3d.md` as parent authorities. Compare React `AssemblyViewer.tsx`, `Workbench.tsx`, `useCaseWorkspace.tsx`, and `CaseInspectorPanel.tsx` with `web/src/renderer_host.rs` and existing `renderer/src/wasm.rs` exports. Account for scene replacement, state/display, camera presets/orbit/zoom/fit, handles, picking, model decoding, lifecycle, context loss, and error recovery. Classify each item as an existing operation, private host mapping/adapter work, or a precisely demonstrated public-contract gap.

Record the chosen reachable wrapper location, operation map, scope identity and scene invariants, supported consumer interactions, lifecycle/failure handling, and exact evidence that F7.3 and later consumer integrations need. Do not prescribe a new engine function, public facade, format, or visibility change. If a current exported operation is insufficient, record the smallest concrete gap and affected consumer; do not assume it.

## Graph and handoff

F7.1 `start_after: []`; `acceptance_after: []`. F7.3 starts after F7.1 and retains INT.2 and BND.1 as acceptance joins. The fixture-backed F7.3 implementation remains actionable before those joins; all other F7 dependencies remain unchanged. This planning publication does not close F7.1 or F7.3 acceptance.

Use shared acceptance at `../dioxus-frontend-tranche-1/ACCEPTANCE.md`, including relevant public-route, source, lifecycle, accessibility, and independent review gates. Record “No new refactoring takeaway observed”; existing RF-002/RF-012 already capture the renderer host and crate boundary unless new source evidence changes that conclusion.
