# Case 11 implementation handoff

The isolated Dioxus Case panel now defaults Live preview on, schedules at most one automatic generation for each current eligible accepted owner, and routes Cancel through the existing Session event while pausing automatic work. The app-root transient signal survives conditional Case workspace unmount/remount. In-flight jobs keep exact accepted token/revision authority.

Completed output reuse is also bounded by a private physical-input fingerprint. It includes the same source-owned geometry inputs as the pinned React `mechanicalFingerprint`: document/board/instance/flipped identity, board thickness, effective mechanical configuration, selected-board part poses/outlines/keycaps and geometry parameters excluding terminal-net values, selected part definitions and geometry-relevant generator/profile fields, board contours/transforms, authored case bodies, and modules. The Runtime rebinds only a completed same-scope result whose fingerprint matches, updating wrapper/IR revisions for the current accepted snapshot. A physical fingerprint change leaves same-scope geometry visible as stale; owner changes continue to retire it. No in-flight request uses fingerprint relaxation.

The React source inputs were read from `app/src/useCaseGeneration.ts` (SHA-256 `f9efd448eb5b4d0fb0c3f99dcd8153cb45d0d6c37fe0f18865a6f3daffa7e479`), `app/src/casePreviewContext.ts` (`300fccfd9733ca3aa9a2daa6875fb10f100bbfc9bb7664972772a1900681dfb6`), and `app/src/hardwareInstances.ts` (`3d79515245989616c3b542875c1afc5a40ca5f771a345abbbcca91b627208b83`). This is a private Rust-side adapter for the existing TS physical fingerprint semantics; the TS/Rust duplication is a refactoring-ledger observation for the coordinator’s existing RF-003, not a new RF ID.

## Verification

- `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web case_generation_lifecycle::tests -- --nocapture`: 3 passed, including revision/name invariance and board-thickness invalidation.
- `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/chris/.cache/.wasm-pack/wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen-test-runner CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 cargo test --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --bin boardstudio-web -- 'cad_presentation::mounted_tests::mounted_case_panel_starts_once_pauses_and_resumes_for_current_context' --nocapture`: 1 mounted headless-Chrome test passed. It covers one-shot auto-start, cancellation/pause, app-root persistence across workspace remount, re-enable, and turning Live preview off during a busy request.
- `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page --bin boardstudio-web --tests`: passed.
- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` and `git diff --check`: passed.

## Still open

The paired public TypeScript/Dioxus journey is still required for saved-project reopen, accepted nonmechanical edit reuse, physical-input invalidation, stale result visibility through blocked edit/Undo, and exact one-current-job behavior. This source slice does not close Issue 11 or F7.6. Existing parent criteria and the Case 12 manual regeneration/context-loss observation remain open until that browser evidence is captured on the integrated candidate.
