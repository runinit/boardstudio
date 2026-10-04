# Consolidated wasm harness follow-up review — 2026-10-04

Status: **Standards CLEAR / Spec CLEAR on the settled source diff at HEAD `6b549657`; actual combined browser gate GREEN, mandatory guarded commit checks still required.** No blocking source finding remains. User directed **“Remove mobile checks completely for now.”** This supersedes the originally considered compact lane: active mobile checks are removed/deferred, desktop and responsive application behavior retained, and mobile acceptance is not claimed. This report is separate from the frozen `astra-consolidation-review-20261004.md`, which remains unchanged. Reviewer performed read-only inspection and `git diff --check`, no application/tooling tests, builds, commits or source edits. Coordinator owns real execution and exclusion removal.

## Findings that determine the change

1. **The five generator exclusions share one missing prerequisite, not five redundant tests.** All read `option_env!("BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL")`, so the URL is captured at Rust compilation. Start one temporary generator asset server before test listing/compilation, set the URL once, and retain that server and URL until all selected modules finish. A new port per module would change the compile-time input and defeat batching. Build the existing `scripts/web/build-layout-generators.mjs` output once; do not create a parallel generator implementation or borrow an unrelated published candidate's assets. The generated entry script imports the generated catalogue; both require JavaScript MIME and CORS for the browser runner's separate origin. Stop the server and remove temporary files on success and every failure path.

2. **Remove mobile checks by explicit scope deferral, not by calling them redundant or passing.** Four tests are removed: the two panel compact tests, separate compact Parts browse test and native compact guide-reveal test. Their desktop companions remain. The fifth, compact guide test is converted into a useful desktop mounted journey rather than deleted; its mobile-specific assertions are removed. Only the three mobile CSS assertions are removed from the shared mirrored-overlay test. The initial compact-lane proposal is withdrawn; no such viewport lane is needed now.

3. **The initial `--all` result-validation gap is fixed.** Previously `--files` retained listed expected names while `--all` passed `expected_tests=None`; its old unit test listed three names, emitted two different names and nevertheless expected success. A crashed/partial module with a tolerated failure could omit listed tests without failing `--all`. Both modes now use the immutable listed inventory and exact expected sets. Zero-execution, incomplete invocation, nonzero listing, unlisted substring matches, harness/build failure and actual failure checks remain. Known assertion failures cannot exempt missing/incomplete outcomes. The previously permissive test now requires all three exact terminal outcomes; separate regressions reject missing tests in `--all` and unexpected/incomplete outcomes under grouped `--depth`.

4. **The component draft failure remains distinct.** `mounted_component_drafts_survive_unrelated_acceptance_and_blur_uses_latest_owner` tests retained drafts/tab/disclosure state across an unrelated accepted revision and commits against the latest accepted document. Its observed `4.00` versus `7.25` failure is not explained by generator assets or viewport setup. Retain that test and its explicit unresolved status; do not remove it as a duplicate, weaken the expected value or imply that fixing prerequisites makes it green.

## Reductions justified by preserved assertions

No Rust test deletion beyond the authorized mobile scope is justified. All five generator tests remain: source membership/unknown-source rejection; reversible Gateron and matrix normalization; parameter-schema fields/types; accepted part model override identity; and four exact portable path mappings are different contracts.

The duplicate work is harness ownership:

| Current entrypoint | Repeated work | Safe replacement and retained coverage |
| --- | --- | --- |
| `scripts/web/test-portable-models.mjs` | Builds assets, owns another HTTP server, loops two direct wasm-pack invocations | Thin compatibility wrapper to strict runner selecting `web/src/bundled_models.rs`. Its four Node path expectations exactly match `native_preview_paths_use_the_packaged_ergogen_asset_identity_helper`: KiSwitch, THQWGD001, infused-kim and untrusted Windows path→None. That Rust test additionally covers the Rust/JS bridge. The packaging builder already imports `layout-generators.js` and checks its exported identity helper, so the wrapper entrypoint is still checked even though the browser test imports the underlying module. |
| `scripts/web/test-physical-setup-proposal.mjs` | Builds assets, owns another HTTP server, hard-codes chromedriver and runs broad `pcb_part_` substring | Thin compatibility wrapper to strict runner selecting **both** `web/src/presentation/parts/catalogue.rs` and `web/src/presentation/pcb_wiring/part_input_settings.rs`. The old filter covers two catalogue tests and four part-input tests. Selecting only catalogue would silently drop four tests. Selecting both also includes `generator_recognition_uses_packaged_source_membership`, which the old filter missed. Preserve `test:web:physical-setup` and both script paths as callable entrypoints. |

Both wrappers are implemented as reviewed above, preserving entrypoints and exit-status/error propagation. Their separate asset builders, HTTP servers and three direct wasm-pack invocation paths are removed. The unified runner owns one fresh package and one server lifetime per CLI invocation; default test modules remain isolated. No separate standalone harness executions immediately after the combined module gate are warranted.

## Required regression and execution evidence

Author reports the `--all` inventory regression RED, then the full fake-runner suite **26/26 GREEN**. Source inspection confirms exact valid listing and missing/unexpected/incomplete cases, generator URL available before listing and stable through both module invocations, one build/server lifetime, socket closure, caller environment preservation and temporary desktop-config removal. Existing file-mapping, nonzero/partial listing, zero-execution, incomplete-known-failure and substring-collision cases remain. These tooling tests mock asset builds and test commands; they are not browser qualification. The coordinator also reports an independent delivery Python suite **26/26 GREEN**. Reviewer did not execute either suite or independently observe their command outputs; the separately executed browser receipt below was directly inspected.

Coordinator's one real batch selected **five source files yielding seven isolated modules**: `bundled_models.rs`, `presentation/parts/catalogue.rs`, `presentation/pcb_wiring/part_input_settings.rs`, `presentation/panels_scroll_tests.rs` and `presentation/setup_guide/tests.rs`. The completed `consolidation-followup-20261004/headless.log` was directly inspected: **21 executed, 0 failed, all seven selected modules**, with the coordinator confirming exit 0. All five generator names below are explicitly reported as now passing; the sole other known failure is outside this selection. Thus these are actual successful outcomes, not tolerated failures. Desktop panels-scroll and the desktop guide also completed under strict inventory validation.

- `bundled_models::wasm_tests::generated_models_use_packaged_generator_part_overrides`
- `bundled_models::wasm_tests::native_preview_paths_use_the_packaged_ergogen_asset_identity_helper`
- `presentation::parts::catalogue::wasm_tests::generator_recognition_uses_packaged_source_membership`
- `presentation::parts::catalogue::wasm_tests::pcb_part_packaged_binding_schema_uses_generator_parameters`
- `presentation::parts::catalogue::wasm_tests::pcb_part_packaged_reversible_proposal_uses_gateron_normalizer`

The coordinator then removed exactly those five generator exclusions. The two mobile exclusions were separately removed under the authorized scope deferral, not repaired. Final allowlist contains only the unresolved component-draft entry, directly inspected at the hash below. Allowlist count therefore changes **8 → 6 by mobile deferral → 1 by demonstrated generator passes**.

Deleting the guide's entire test module would conflict with the unchanged commit gate: `setup_guide.rs` is wasm-only, so its selected prefix would have zero tests. The final diff instead preserves useful mounting/wiring coverage with `desktop_case_stage_opens_settings_and_keeps_the_guide_available`. At effective desktop width >=981px it starts Inspector collapsed, clicks the real Case guide stage, asserts the Case workspace and one readiness detail, proves the stage click leaves Inspector collapsed, uses Configure to pin Inspector, returns through Layout/Case, reopens the guide and checks Objects/Inspector availability and pinned modes. Root cleanup is added. Native `panel_reveal` booleans do not subsume this mounted callback/panel join. The zero-test guard remains strict; no dummy test or gate exception was introduced.

Targeted residual-check search found the five mobile-specific Rust tests and three mobile CSS clauses above; four tests are deleted and one converted to desktop. Current migration-deliver/build/qualification tools do not introduce another mobile browser lane. Synthetic `compact_case_inspector` names in runner unit tests are filter-mapping fixtures, not mobile checks. Many retained reference `app/e2e` suites contain narrow viewport branches, but they are not invoked by the migration native/page/wasm gate; do not expand this Dioxus harness batch into deleting historical reference/CI behavior. `CONSTRAINTS.md` and the canonical run's explicit `validation_scope` now record the user's mobile-check deferral, preserve responsive production behavior and desktop checks, and prevent historical mobile evidence being counted as current acceptance. No desktop scroll, actual overlay-interaction, native Core/Session, error/retry or scope-lifecycle assertion is removed by this batch.

The cached test runner matching Cargo.lock is `wasm-bindgen 0.2.129`; its read-only `--help` confirms `--exact` and repeatable `--skip` filters. Browser capability configuration uses `WASM_BINDGEN_TEST_WEBDRIVER_JSON`, confirmed by the official [headless-browser guide](https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/browsers.html) and [runner source](https://raw.githubusercontent.com/wasm-bindgen/wasm-bindgen/main/crates/cli/src/wasm_bindgen_test_runner/headless.rs), consulted 2026-10-04. The guide/source are current upstream rather than a verified 0.2.129 tag; the installed runner and actual execution remain the version-specific authority. The final runner supplies temporary 1280x900 desktop capabilities when the caller supplies none, while preserving an explicit environment setting or existing crate/root webdriver.json. The retained guide test checks the effective CSS viewport; no compact lane or mobile setup remains.

## Final implementation review

The settled diff has no blocking Standards or Spec finding. The duplicate `nullcontext` import identified during final inspection was removed; the source hashes below are after that cleanup. Fresh generator output uses the existing builder, serves only its two required JavaScript paths with CORS, starts before compilation/listing, remains alive with one URL for the full batch, and closes through context-manager cleanup. Relative `--root` is normalized. Fake-runner override bypasses real browser/assets. Module isolation and strict inventory validation are preserved rather than broadened into a shared page.

The executable remainder is bounded: run the mandatory guarded commit's native/page/reachability/affected Rust wasm checks once for the settled source. The prerequisite batch and exact five exclusion removals are complete; no additional wrapper execution or review chain is needed. Keep the one unresolved component-draft exclusion. Acceptance of tooling repairs or desktop criteria does not accept mobile, the unresolved component-draft behavior, a whole parent or the release. No new parent joins are recommended by this follow-up.

Reviewed source identity (SHA-256):

| File | Hash |
| --- | --- |
| `scripts/run-wasm-tests.py` | `da65de9ef32cabb9e54b762ff12f865738cc44edbaadb26b0abd0f376f1d7e8e` |
| `scripts/test-run-wasm-tests.py` | `6a55837e9792f42a05b0bbb6d33bf729489bc1b2992e4a0d003f8146bcf6415e` |
| `scripts/web/test-portable-models.mjs` | `97a851c46f5e9e41b1638cde7b9c844f6d50a5c1b251c207b5e6bad73740253f` |
| `scripts/web/test-physical-setup-proposal.mjs` | `0d155602202d6b21909d3fa0447080cdf423306ef635fb2d5b33b92585613dfe` |
| `scripts/wasm-known-failures.json` (one unresolved entry) | `d8abf875f191d0d812d052961e5ae33cbbc349f9d402a994574d85ad4f0a0e5a` |
| `web/src/presentation/setup_guide/tests.rs` | `59057e85004da5107cbfc22d5af645fa2202c5ffe2235e006e28e26b270ce01f` |
| `web/src/presentation/panels_scroll_tests.rs` | `069f51a30f8a1d8b2039d5bc844e9eb6f816ac76603cb439b5fda09aacd3dec3` |
| `web/src/presentation/parts.rs` | `052635eba0febec8c26a6646fa50e3cf88f9edf2704d34d39e2357908cd781df` |
| `web/src/setup_guide_state.rs` | `76374af8edde816cb01d8272446e6731596542aa3bda1d3edae2ae721c4f3013` |
| `web/src/presentation/objects/mirrored_pair/canvas_overlay_tests.rs` | `65cf209c78d124314f76c6b4ddd711ef3b789e844602af4ff510ccae55a3edd5` |
| `CONSTRAINTS.md` | `b62aa339e313a1e5c00e144ad04eaf965d421fef0bf656d875be30aeceef93f6` |

The earlier frozen review remains byte-for-byte unchanged at SHA-256 `514ba7891dc078610f07c23cd46ae1d36f09c6cae38c8f0c8b85cb56b046f34b`. This follow-up is frozen after final source inspection and the successful combined browser receipt; coordinator guarded-commit receipts establish the remaining checks.
