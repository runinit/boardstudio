# Next keycaps frontier drafts

Planning only. These drafts are saved outside the repository at the authorized temporary destination. They are not published in `.scratch`, dispatched, or counted as completed parent work. The existing 62-task graph and parent acceptance conditions remain unchanged.

## Source evidence checked

- React Keycaps: `app/src/ui/KeycapPanel.tsx` and `app/src/ui/createKeymapWorkspace.tsx` share a board-scoped `KeymapView`; projection selects supported direct and matrix switch parts, applies generated defaults for display, derives legend from explicit setting or binding, and shares `selectedKeyId`/`selectKey` with Keymap. `keycapsCanvas` renders physical legends while `canvas` renders active-layer binding labels.
- Existing capability: `CoreRequest::ResolveKeycaps` dispatches to the Rust keycap resolver (`core/src/lib.rs`, `core/src/keycaps.rs`). The packaged CAD WASM exposes `build_keycaps` (`cad/wasm/src/model/keycaps.rs`, export from `cad/wasm/src/lib.rs`).
- Concrete private gap: current web CAD wire enum has `Preview`, `Exact`, `ExportStep`, and `ReadStep`, while its request shape carries only case assembly input (`web/src/cad_jobs.rs`). The current worker/runtime paths are case-oriented (`web/src/cad_worker.rs`, `web/src/runtime.rs`). This supports an independent BND.1 proof; it does not imply a need for public API changes.
- Workflow contract: F6C.1 retains `start_after: [INT.1]`, `acceptance_after: [F3.1]`; BND.1 has no start or acceptance blockers. Later F6C.5 retains its existing dependencies including F6C.4, F7.1, F7.3, F8.2, and BND.1. No other parent IDs or graph edges are changed.

## Frontier rationale

F6C.1 can deliver projection, shared selection, and physical 2D independently of CAD/3D. Its parent’s F3.1 acceptance join remains intact, while the implementation start gate stays INT.1. BND.1 independently proves the missing private keycap CAD path before later consumers depend on it. No blanket INT.2 or CAD gate was added to F6C.1.

No RF ID was created: this read-only planning/source mapping produced no new evidence-backed refactoring finding. Each implementation/review handoff must still update the register or explicitly record no new takeaway.
