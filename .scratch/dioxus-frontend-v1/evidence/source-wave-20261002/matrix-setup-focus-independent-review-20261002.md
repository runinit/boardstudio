# Matrix setup focus independent review — 2026-10-02

Bounded Spec + Standards source CLEAR for4c9acdeb05cd03d7c1eb19664bdeeb7f44e7e6aa in matrix-setup-focus-repair-20261002.

Exact SHA256 verified: setup_guide.rs b6351015153f50c50ec0321aa454d3b1d361dff0cc3b4c9606a6ecd48e23717b; objects/matrix_setup.rs a004535c3c5c5639498c6e0793a203a95366bf53bf8b0d49482abbbd17ab7e35; actual browser composition test7e04ba2c4ef8ea66880b33987a0871541f27a1c8ed06656d7f6d6b5d67b73e35.

Compared React Workbench.tsx guide heading focus effect and Rows autoFocus with the actual Dioxus mount boundaries. Installed dioxus-web0.7.10 MountedData::set_focus targets the supplied element directly and executes focus before returning its resolved future; no selector lookup, stale owner signal or delayed global action is introduced. Negative tabindex makes the heading programmatically focusable without adding a tab stop. Ordinary input/stage rerenders do not remount/focus either target.

Inspected expected red(false,false,true,false)/BODY and final real Chromium production MatrixSetup + ProjectSetupGuide test. Independently reran exact focus test in headless Chromium:1/1 passed, including Rows focus, returned guide heading, bubbling Columns input and stage-button focus preservation. First rerun attempt failed only because a selected home TMPDIR did not exist; created that local temp directory and reran successfully. Independent log /tmp/matrix-setup-focus-independent-green-20261002.log. Reused author's native11/11, compact composition1/1, strict all-target WASM and format evidence after checking exact source hashes.

No blocking finding within this mount/cancel repair. This is not whole-guide focus equivalence for every panel reveal path and does not close New17, MatrixSetup, parent joins or exact packaged public paired acceptance. No API/member visibility widening, suppressions or unrelated changes found. RF-006/RF-009 carry-forward remains applicable.
