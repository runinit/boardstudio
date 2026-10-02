# New17 compact guide composition repair

Isolated branch codex/new17-compact-guide-repair-20261002 from defb6fd448ff5a13494ac92d731595a4c158033e. No root checkout changes. Root's source-stamped candidate at http://127.0.0.1:34722/boardstudio/ reproduced the defect through a fresh browser session: Create new keyboard → Case stage at 640×900 closes Objects, leaves open:true/currentStep:case and makes Open case settings unreachable. Independent public red screenshot/DOM capture are adjacent.

Cause: stage navigation explicitly reveals Objects, but the later workspace-change default effect unconditionally closes it for Case. The fix extracts those actual production transitions, records a one-transition explicit workspace reveal, and consumes it before applying ordinary workspace defaults. Same-workspace reveal still works; later ordinary Case navigation still opens Inspector. Existing selection/drag cancellation remains in its original effect, and no shared/public contract or member visibility changes. The Case body also drops its duplicate optional sentence: accepted stage detail already supplies it; React's construction paragraph remains intact without inventing readiness.

Actual Chromium WASM regression mounts production ProjectSetupGuide, ObjectsPanel and InspectorPanel through dioxus_web into the DOM, then clicks the real Case-stage button and real settings action. It observes compact panel aria-hidden/inert, not a copied policy result. Old defaults fail with Objects hidden; after correcting precedence, the same test fails independently with duplicate Case copy 2 vs 1. Final green checks both fixes plus ordinary workspace navigation and same-workspace guide reopening through test navigation controls. It uses real matchMedia at 640×900 via the adjacent webdriver.json. This is real browser-mounted composition evidence; the complete public root candidate must still be rebuilt and paired by root.

Executed:
- wasm-pack test --headless --chrome web --no-default-features --features page --bin boardstudio-web -- compact_case_stage_keeps_the_real_guide_visible_until_settings_action (expected red then final 1/1 green).
- strict WASM page all-target Clippy: pass.
- native page suite: 19+57+6+7+10+20 pass.
- cargo fmt and git diff --check: pass.

For the focused browser command, set WASM_BINDGEN_TEST_WEBDRIVER_JSON to the absolute adjacent webdriver.json. Build cache reused via explicit CARGO_TARGET_DIR pointing at the Case worktree web/target; no Case source was modified. No public API/config/dependency/suppression change. Existing five policy tests remain, but are not substituted for this composition regression.

Independent Spec/Standards re-ack required (author cannot self-approve). Full current-source paired guide/physical setup/MatrixSetup acceptance remains open. Carry existing composition/accepted-identity RF lessons forward; no shared ledger/parent graph edits.
