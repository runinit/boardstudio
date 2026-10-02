# Case model mapping failure ownership repair

Defect correction on base0c141ab11a22b59661c0c809f6ee129ff9f18538, including adapter9e286d38 and preserved provider tests. Root explicitly assigned isolated behavioral repair; no integration checkout writes.

The production CasePanel version subscriber invokes Runtime delivery whenever the accepted preview exists. The new mapping error branch cleared pending and notified, then returned Err to Runtime::report, which also notifies. Thus a rejected module import or malformed mapping repeatedly admitted the same owner; a stale error escaped to the replacement project's global status.

Extracted that actual Runtime admission/mapping completion branch into private resolve_native_model_paths without correcting behavior first. A real Chromium test drives this exact production function through version notifications (including the caller's report notification); another releases a delayed rejected mapping after its source becomes stale. Both failed as expected: eight starts instead of one, and Err(old module import rejected) instead of no reportable result. This is the actual production admission/await seam, not a copied policy implementation; the test driver supplies its browser-bound resolver/currentness/notification ports. It does not claim a mounted full Case app journey.

Correction: retain the exact failed preview owner in the existing NativeModelDeliveryState before notifying; same-owner pending, published and failed attempts are terminal for automatic version-driven admission. A new preview owner clears the prior failure and may retry (including remount/reopened/new accepted preview). Check active source and pending identity after the mapping await. Stale successes/errors retire only their own pending record, return no mapping/error to the caller and do not notify or overwrite the replacement. Existing byte/provider/decoder/mesh-cache/lease behavior and external APIs remain unchanged. No new retry button or second authority is introduced.

Executed checks:
- Actual browser red:0/2, exact expected loop/stale-report failures.
- Actual browser green:5/5, includes rejected and malformed mapping settling once, stale error suppression, new-owner retry and pending deduplication, old failure preserving replacement pending, and stale success rejection.
- True WASM page all-target strict Clippy (--all-targets -- -D warnings), formatter and diff checks pass. Production path is WASM-only; existing native provider/digest/cache evidence is unaffected and no unrelated broad suite was repeated.

Command: CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=<installed wasm-bindgen0.2.129 runner> CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 WASM_BINDGEN_TEST_WEBDRIVER_JSON=<Inspector desktop webdriver.json> CARGO_TARGET_DIR=<Case shared target> cargo test --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web -- native_model_mapping_tests

Log copies normalize whitespace only; original raw outputs remain /tmp/case-model-mapping-{red,green,clippy}-20261002.log. Independent both-axis review is required because this report is authored by the fixer. Fresh packaged Case root/subpath/offline paths, bytes/decode receipts and 90-row fixture journey remain open. Root source/report owns integration and RF ledger; retain RF-003/RF-009 and failure-owner lifetime observation under RF-006. No parent acceptance closure.

Current CONSTRAINTS authority read at rootd9d4bc3839e0b38b41efeb7621f7961a06747940; narrow private interface extraction and affected supported-target checks are within this task.

Source hashes:
- web/src/runtime.rs: `6b7b1f3b4b387968701c1c3fb007e2cb100d9a3d5f808682eec0b88ddea9dd77`
- web/src/native_model_mapping_tests.rs: `65b6052814fa8f42183c4dd7ed97c1e94a85fe0f4ff5e676565fb04771539ff0`
