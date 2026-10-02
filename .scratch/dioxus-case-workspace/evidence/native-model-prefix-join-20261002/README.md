# Native Case model path prefix join repair

Category: bounded defect correction at the Issue 10 model-selection consumer of Issue 11's generated physical preview. Base `f782fc081af9b50d77b8a74f2495a95c7cb31a6b`, isolated branch `codex/case-model-path-prefix-fix-20261002`. Root remains the serial integration owner. This packet has not been independently reviewed or publicly accepted.

## Repro and oracle

The production Core formatter (`core/src/artifact/kicad.rs`) turns an export-relative model path into `${KIPRJMOD}/<path>`. The native preview path table retains the relative path by source asset ID. Runtime inverts that table and passes it into `resolve_preview_assets`, but the selector previously looked up the entire prefixed `PcbModel.path` as an exact key.

The Issue 11 audit's New17 fixture is `/home/chris/.local/share/boardstudio/retained-tmp/20261002/new17-final-oracle-accepted-project.json`, independently verified SHA-256 `658e2488ef3ac2514646f177c99b6c12f9f47561bd9a42211f72a93d1609b77a`. Its reported switch identity is `ergogen:model:kiswitch/SW_Cherry_MX_PCB.stp`, native path `assets/ergogen-models/model-dd931656985824ce.stp`, and Core preview path `${KIPRJMOD}/assets/ergogen-models/model-dd931656985824ce.stp`. The generated catalogue independently provides this exact descriptor with digest `ddf0fb3a776faa6105303efb98e39645e044c43b3ac03cbfe75d8eda28e297f4`.

The original full-chain harness source/output in `/tmp` is no longer present. Its retained executable exists under the retained-tmp directory, but its invocation is unavailable. This packet does not claim a fresh full-chain or browser execution. The focused regression instead exercises the actual production consumer chain `native_model_path_assets` → `resolve_preview_assets` → `select_model_asset`, using the real generated bundled descriptor and exact emitted path. Its Ergogen fallback returns no ID: the accepted provider only recognizes the legacy `${KIPRJMOD}/models/boardstudio/` pattern, so a successful join must come from the authoritative native table.

## Correction

Preserve attached BoardReference mapping first, then exact native lookup. When that exact native key is absent, strip only the literal `${KIPRJMOD}/` prefix and look up the resulting key in the same table. The existing Ergogen fallback remains last. No path parsing, new resolver/provider, digest/catalogue changes, source table changes, public API/schema widening, admission changes or stale-owner changes are introduced.

The regression failed before the repair with actual `NoAssetId` versus the expected exact packaged descriptor/model-row identity. The correction passes it. A second regression preserves native document-asset selection over the Ergogen fallback for prefixed archived-model paths. Existing tests retain attached-reference precedence, relative paths, byte digest/bounds checks, healthy-row/error behavior, cache ownership and stale settlement.

## Executed checks

- Expected red: `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --bin boardstudio-web core_packaged_preview_path_joins_the_native_table_to_its_exact_descriptor` — FAIL for `NoAssetId`, retained in `expected-red.log`.
- Green model consumer suite: `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --bin boardstudio-web model_delivery::tests` — 13 passed (`model-delivery-green.log`).
- Native page binary tests: `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --bin boardstudio-web` — 96 passed (`native-bin-green.log`).
- Supported WASM all-target strict Clippy: `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings` — PASS (`wasm-strict-clippy.log`).
- `cargo fmt --manifest-path web/Cargo.toml -- --check` and `git diff --check` — PASS.
- Full native web suite: `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page` — BLOCKED at existing `tests/keymap_panel_lifecycle/harness.rs:146` missing required `on_export` property (`native-suite-blocked.log`). Root separately owns the reviewed harness prerequisite and a fresh integrated rerun. This repair does not alter that test or claim the full gate passed.

Logs retain all diagnostics; only terminal blank lines were normalized in their committed copies. Raw logs remain in `/tmp/case-prefix-*.log` for this session. Environment: native Linux; target `wasm32-unknown-unknown`; selected locked toolchain/framework dependencies unchanged. No lint suppression, skipped test or weakened assertion was added.

## Remaining gates and RF handoff

A different Sol reviewer must independently review this exact packet before root integration. Root must join the harness prerequisite and Issue 11 source, rebuild, and verify actual Case model rows with the current public browser fixture. Resolution to a correct descriptor alone does not prove fetched bytes, decoded STEP meshes or rendered row counts. Retain Issue 10/11 and F7 acceptance joins; no parent closure or publication follows.

RF-003 already covers physical model projection and RF-001 the adapter boundary. No new distinct refactoring takeaway observed for this literal-prefix consumer correction.
