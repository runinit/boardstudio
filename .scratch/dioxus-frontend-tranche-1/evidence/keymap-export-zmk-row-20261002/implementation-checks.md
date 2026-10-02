# ZMK Export row implementation checks

Source changes are isolated on `codex/keymap-export-zmk-row-20261002` from planning baseline `bd9b3d761041b10dce324d54d64dd331c0d8bc2c`. This record covers source-level implementation only. The root/subpath release build and paired React/Dioxus product browser journey remain pending root serial integration and candidate build.

## Implemented checks

- `cargo fmt --check`: pass.
- `git diff --check`: pass.
- `cargo check --target wasm32-unknown-unknown --bin boardstudio-web`: pass before the final current-operation guard; strict Clippy and WASM test compilation pass after that guard.
- `cargo clippy --target wasm32-unknown-unknown --bin boardstudio-web -- -D warnings`: pass after the final current-operation guard.
- `cargo test --target wasm32-unknown-unknown --bin boardstudio-web --no-run`: pass after the final current-operation guard.
- Headless Chromium WebAssembly tests for Runtime firmware export failure/currentness/reporting: 5 passed. Captured in [runtime-tests.log](runtime-tests.log). Coverage includes generation and ZIP failure reports, no-byte failure results, worker replacement at each await boundary, stale owner suppression, and suppression of a late older export report after a newer export starts.
- Headless Chromium WebAssembly tests for row copy, disabled/ready state, accessibility name, ready dot, plan readiness, and action admission: 3 passed. Captured in [row-tests.log](row-tests.log).

The row-level browser test initially queried before Dioxus mounted the virtual DOM and observed zero rows. Adding a short mount wait made the actual rendered-control assertions pass. This was a test timing defect, not a product behavior change.

## Remaining verification

- Root-owned integrated release build with the reviewed Core/static providers and root/subpath routes.
- Paired fresh-profile React/Dioxus Export ready and unavailable journeys, including download filename and generated archive.
- A real browser-induced generation/ZIP worker failure. Production Runtime seams are covered by separate injected generation and packaging errors, worker-currentness replacement at all three awaited boundaries, no-delivery results, and stale-report suppression tests; provider failure was not induced in the retained product browser session.
- Independent source review and root serial integration. F8 parents remain open.

## Refactoring ledger handoff

This feature continues the existing RF-001 shared presentation/Runtime hotspot: one bounded Export row crosses the shared Editor/Runtime composition, so the current private leaf and root-owned serial mount are kept explicit. No broader refactor or measured performance claim is proposed.
