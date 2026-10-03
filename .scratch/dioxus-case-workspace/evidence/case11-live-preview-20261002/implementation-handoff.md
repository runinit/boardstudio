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

## Independent review repair

The review of frozen implementation `590e1eac56430b0ad6dea42edc11df9d142a4bc3` identified three lifecycle gaps. The isolated repair now restricts fingerprint rebinding to exact completed output, keeps a previous completed same-scope result and its selected contextual layer visible with explicit previous-revision labels, and preserves a terminal automatic-generation attempt across temporary ineligibility. Selection and settings mutation still require the current accepted owner; a scope/instance change does not retain prior geometry. These changes do not close Issue 11 or the parent gates.

The terminal-retry regression was run against the prior reset-on-ineligible behavior and failed at the expected final assertion when the same owner dispatched again. Red log: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/case11-review-repair/terminal-retry-expected-red.log`, SHA-256 `e1d1451a4ec8c7953b253356e853d11bf54a57cb7fb964ab2e1cfe4655a99452`.

After the repair:

- Native lifecycle tests: 6/6 passed. Log: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/case11-review-repair/lifecycle-green.log`, SHA-256 `4fe6cbaadf5093e6bd4cba1946a37073fbc972c1cb7594aac4d55db6e82441a6`.
- Mounted headless Chrome stale-layer/Inspector test: 1/1 passed. It verifies the previous geometry notice and resolved thickness, continued access to current settings, and previous-stack labels when returning to the resolved stack. Log: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/case11-review-repair/previous-inspector-mounted-green.log`, SHA-256 `686246afe38df4959a63e2cebaa9d51db1b2e8bf366a3ca77a56ad46890d4ea3`.
- Strict WASM page Clippy passed with `-D warnings`; formatting and `git diff --check` passed.

The two private predicates intentionally separate completed-result reuse from old-result display. This leaves a useful RF-003 refactoring takeaway: freshness for display continuity and authority for mutation should remain separate concepts. The shared architecture/refactor ledger remains coordinator-owned and was not edited in this repair.
