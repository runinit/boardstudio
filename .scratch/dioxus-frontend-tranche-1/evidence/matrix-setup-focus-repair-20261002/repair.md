# Matrix Setup / guide focus repair — 2026-10-02

Defect correction in isolated `codex/matrix-setup-focus-repair-20261002`, base `119c9d059c002057e73b3fd3a36648a9681c9cd6`. Root build/source freeze remains untouched.

Reference evidence from New17 author: source-stamped Dioxus34723 restores the Layout guide preference after MatrixSetup Cancel, but activeElement remains BODY after 1.6 seconds; fresh pinned React5173 focuses Rows on form open and the Setup guide heading on return. Source confirms React Workbench.tsx uses Rows autoFocus and an effect on guideVisible to focus its tabindex=-1 heading. The existing Dioxus leaves had neither form initial focus nor a focusable/mount-focused guide heading.

The correction attaches focus to the actual mounted Rows element and guide heading with Dioxus 0.7.10 MountedData::set_focus. Installed dioxus-web source implements that request on the supplied HtmlElement; it does not perform a global selector lookup or defer a callback that could resolve a newer owner. The heading gets tabindex=-1. Focus runs on mounting, not on draft or stage rerenders. Labels, guide preferences, MatrixSetup admission, pending ownership, operation/history and cancellation are unchanged. No API/member visibility, configuration or suppression change.

The Chromium regression mounts the actual production MatrixSetup and ProjectSetupGuide components, follows their real Add key matrix / Cancel DOM callbacks, and inspects document.activeElement. Its fixture only supplies the parent visibility state and accepted form projection; it does not claim to test Runtime persistence or duplicate focus logic. Original production source fails with (initial heading, Rows, Columns preserved, returned heading) = (false,false,true,false), activeElement BODY. Fixed source passes all four and preserves focus on the clicked stage button after a guide rerender. A real bubbling Columns input exercises the production draft update without focus theft. The original red used a nonbubbling input; the form/cancel focus assertions and failure are identical, and the final bubbling event strengthens the preservation case.

Checks:
- Actual headless Chromium regression: expected red, then green1/1.
- Existing compact Case guide/panel regression: green1/1.
- Native actual MatrixSetup lifecycle harness:11/11 green; seven pre-existing native unused provider/archive warnings are recorded, with no suppressions added.
- WASM page Clippy with --all-targets and -D warnings: green.
- cargo fmt --check and git diff --check: green.

Focused browser command: CARGO_TARGET_DIR=<existing Case web/target> WASM_BINDGEN_TEST_WEBDRIVER_JSON=<this checkout>/.scratch/dioxus-frontend-tranche-1/evidence/new17-compact-guide-repair-20261002/webdriver.json wasm-pack test --headless --chrome web --no-default-features --features page --bin boardstudio-web -- matrix_cancel_restores_guide_focus_and_new_form_focuses_rows. Existing compact test uses its own exact test filter. Native: cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --test matrix_setup_lifecycle. Strict: cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings.

Source SHA-256:
- setup_guide.rs: b6351015153f50c50ec0321aa454d3b1d361dff0cc3b4c9606a6ecd48e23717b
- objects/matrix_setup.rs: a004535c3c5c5639498c6e0793a203a95366bf53bf8b0d49482abbbd17ab7e35
- objects/matrix_setup_focus_tests.rs: 7e04ba2c4ef8ea66880b33987a0871541f27a1c8ed06656d7f6d6b5d67b73e35

Independent Spec/Standards source reack and integrated packaged browser pairing remain required; author cannot self-clear. Current34723 remains the pre-fix public candidate. This does not close New17/MatrixSetup/parent acceptance. Carry RF-006 mounted focus lifetime and RF-009 exact browser/source evidence forward; no canonical ledger edits.
