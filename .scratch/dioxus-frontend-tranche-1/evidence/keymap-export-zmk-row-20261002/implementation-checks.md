# ZMK Export row implementation checks

The source change started from `bd9b3d761041b10dce324d54d64dd331c0d8bc2c` on `codex/keymap-export-zmk-row-20261002`. The bounded production-verification follow-up is isolated after the original frozen source review at `6a0456397633973b999d450ef9db995473143296`; it does not imply root integration or full F8 acceptance.

## Verification

- `cargo fmt --manifest-path web/Cargo.toml --all -- --check`: pass.
- `git diff --check`: pass.
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --bin boardstudio-web -- -D warnings`: pass.
- Focused headless Chromium WASM tests `zmk_firmware_export`: 5 passed. [row-tests.log](row-tests.log) includes the mounted production `ExportPanel` and action hook, verifies disabled rows dispatch nothing, mounted generation failure is announced through the actual alert role, the same row retries successfully, and accepted Session state remains unchanged.
- Focused headless Chromium WASM tests `firmware_export_tests`: 9 passed. [runtime-tests.log](runtime-tests.log) exercises actual injected Runtime/Core resolve, firmware generation, archive packaging, terminal and delivery paths. It covers separate generation/ZIP failures, same-Runtime retry, accepted Session/history preservation, worker replacement while each actual async boundary is suspended, stale-owner suppression, and an older in-flight export settling after a newer successful export.
- Full web WASM suite: 34 passed, 9 failed in unrelated environment-dependent tests. [full-wasm-chromium-tests.log](full-wasm-chromium-tests.log) records the run. Failures are in packaged-model tests requiring `scripts/web/test-portable-models.mjs` assets, three Inspector-scroll/compact-layout tests, and a compact SetupGuide viewport test; all 14 ZMK-specific production and helper tests passed in that run.

The first mounted alert assertion exposed that the extracted shared report banner did not read the Runtime-update signal in its own component. The banner now tracks that signal, so the production alert/status role updates when Runtime reports an export failure or successful retry.

## Remaining verification and gates

- Root-owned integrated release build with the reviewed Core/static providers and root/subpath routes.
- Paired fresh-profile React/Dioxus Export ready and unavailable journeys, including download filename and generated archive.
- Independent re-review of this follow-up and root serial integration. F8 parents remain open.

## Refactoring ledger handoff

This feature continues RF-001's shared presentation/Runtime lifecycle concern. The mounted action and controlled async executor seams test those production consumers without public API widening or a new provider. This bounded repair makes no broader refactoring or performance claim.
