# Dispatch notes — F5.1 PCB host layers

## Ownership and seam

INT.1 is only the parent start edge; it does not create a PCB mount, canvas, scene projection or layer callback. Coordinator must assign the page-crate PCB mount, shared canvas ownership, accepted document/scene inputs, current board scope and selection callbacks before implementation. F5.1a includes its own private PCB canvas/controller proof and feature module; don't parallel-edit coordinator-owned `web/src/presentation.rs`. No automatic INT.2 join is required for these local read/scene paths unless implementation evidence shows an actual use of that seam. Do not change core/CAD/schema/public APIs or treat `ArtifactRequest::PreviewBoard` as proof it is the reference host rendering path.

## Source and behavior

Parent requirements and dependency table are `.scratch/dioxus-frontend-v1/issues/05-pcb.md`. Reference sources: `app/src/ui/Workbench.tsx` (board-scoped scene and `hiddenLayers`), `WorkbenchLayers.tsx` (`footprintLayers` and controls), `ScenePart.tsx`/the existing PCB scene projection callers, and `ModulePcbOverlay.tsx` (module visibility ownership, excluded from these host tickets). Candidate has only Layout canvas/layers in `web/src/presentation.rs`; `web/src/presentation/footprint_graphics.rs` is Layout-oriented projection and is not complete PCB artwork rendering.

For visibility, React keeps one transient `hiddenLayers` set shared by Design and PCB through board/workspace changes; a separate `hiddenModuleLayers` set owns module overlays. Dynamic footprint layer names map front/back according to part side; the standard PCB groups include Edge.Cuts, Courtyards, Pads, Holes and References. Confirm actual accepted scene/definition provider path for host copper/artwork, pads/drills and coordinates before claiming support. Each group must affect only its real host geometry. Preserve stale scene rejection on board/scope changes.

## Checks and profiles

Run `cargo fmt --manifest-path web/Cargo.toml -- --check`; `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings`; `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web`; plus applicable page build and real public Dioxus browser evidence with current build provenance. Do not count React Vitest tests as Dioxus proof or add new gates. Compare paired boards, geometry, layers, themes/desktop/compact, keyboard/focus/axe, unchanged revision/history/export inputs, empty/error and stale-scope handling.

F5.1a: Luna High author/verifier, Astra High reviewer; M. F5.1b: Luna High while shared visibility lifetime/controller is new; Medium only after existing callback contract is demonstrated; Astra High reviewer; S/M. Parent RF-009 source-accounting reconciliation should be recorded at publication.
