# Consolidated functional candidate review — 2026-10-04

Sol 6.1 High reviewed the application delta from previously reviewed `bb4f221687a49098e93a451dc854b74134abdf72` through `9eda1b5d4c194a97b7216a43544036b5709a65f8`, then its targeted repair at **`9dd2a412823b650d5b055733181b59ca724c68f1`**. This is the single Standards + Spec review for this functional milestone, including the retained paired receipts. Final candidate **frontend-case-panels-20261004** is served at `http://127.0.0.1:34802/` and `/boardstudio/`. Earlier actions candidate 34801 remains evidence for unchanged actions. No source, tracker, run or build was changed by the reviewer; no application test or browser journey was repeated.

## Provenance and checks

Final proof: `.scratch/dioxus-frontend-v1/evidence/frontend-case-panels-20261004/package-proof.json`. Its provenance independently rehashes to `8f9c3021317208b5b883b8a24c52e4cfb0a9f548f2da3ca55b949b63bdd83ce2`. All **1,393 current source hashes**, **190 local assets per route**, and **23 inherited command-log hashes** match. All nine fresh and 23 inherited commands exit 0. Inherited providers come from the already reviewed bb4f2216 full-build provenance `ac4d3409f85c532d1d28e0f8ab275c4bae87ce42a934d95ccc45db581cae9334`. The proof records both live routes at HTTP 200 with COOP/COEP, no asset mismatches and zero release warnings. The locked page compiler has three existing dead-code warnings; strict warning-free Clippy is not claimed.

The preceding 9eda proof also independently matched its sources/assets/logs before repair. Final application files are clean against the frozen commit, and `git diff --check bb4f2216...9dd2a412` passes. Unrelated working-tree changes were preserved.

## Standards

CONSTRAINTS.md, the domain/issue-tracker guidance, docs/architecture.md and accepted ADR 0003 govern this review. No remaining new blocking standards breach or actionable smell finding. Core retains geometry/history authority; Parts extraction uses its existing artifact service with current owner admission; model placement matrices come from Core and verified asset delivery. Case geometry remains a disposable same-revision preview, separate from accepted generation/export authority. Panel resizing uses existing persisted presentation settings and bounded private controls.

The Case defect below also violated ADR 0003's preserved gesture completion contract; its bounded correction clears that breach. **RF-025** now records the distinct accepted-gesture versus disposable-render identity coupling and proposed later ownership split. Retain RF-001/009/019 integration debts and **RF-024**'s open finding-overlay affordance correction. No second structural refactor or public API widening is implied.

## Spec

**P1 found and repaired: provisional Case geometry cancelled its own valid drag.** On 9eda, holding a valid live mount/gasket move until CAD published replaced the displayed `Rc<CadScene>` (`case_viewer.rs:352–357`). `ProjectionInputs.source` advanced the full viewer identity; `cancel_superseded_pointer` then removed the Handle and released capture. Later pointer-up returned without End, so the accepted gesture edit could not occur. The invalid-movement browser probe did not exercise this path.

9dd supplies the accepted Case scene separately as `handle_source`. Only an admitted provisional transition sharing that accepted scene, scope/token/revision/viewer and unchanged auxiliary inputs transfers the Handle to the new full identity. Move/End and old-callback guards remain strict; genuine source replacement and unmount still cancel. Coordinator-reported focused WASM **RED→GREEN, 2 executed tests**, covers production identity advancement, pointer admission and Move→End emission, plus 15 invalid transition variants. The reviewer read this production helper/test path; it is more than the earlier preview-state-only test. **The source defect is resolved; public valid drag→provisional geometry→release→Undo remains unqualified.**

## Scoped behavioral disposition

| Action / retained receipt | Supported result and remaining limits |
| --- | --- |
| F3.5 Pick origin — `dioxus-layout-authoring/evidence/f35-matrix-properties-relations-20261003/source-implementation.md` | 34800 Inspector-focused Escape RED is repaired on 34801: Escape prevents the subsequent pick, later live pick works, one Undo restores origin. Other F3.5 clauses remain open. |
| Project copy — `dioxus-frontend-v1/evidence/functional-delivery-20261003/f2.2-c02-c03-paired-qualification.md` | Paired Project-menu download closes the menu; fresh-session import retains both boards, parsed project/metadata and six verified embedded assets. Broader archive/recovery/supersession clauses remain open. |
| Macro no-op — `dioxus-keymap-layers/evidence/macro09-zmk-root-public-qualification/RESULTS.md` | Paired unchanged A blur retains revision/hash; changed C blur makes one revision 29→30 with stable macro identity. Existing broader macro evidence is retained. |
| PCB Remove — `dioxus-pcb-view/evidence/22-mounted-module-inspector-20261003/placement-save-reopen.md` | Candidate selected placement disappears, modules 3→2/findings 97→85. Inspector unmounts; React source also clears selection without a success label, so absent text is not an established parity defect. Paired React removal, pending/error/cancel and Undo/reopen remain unqualified. |
| F4.5 extraction — `dioxus-parts-catalogue/evidence/parts-fit-profile-capability-receipt-20261002.md` | Retained paired mapped cutout/clearance, Save/Undo/Redo/editor reopen; candidate circular-drill validation/retry observed. Successful hole mapping, stale-response races and wider qualification remain open. |
| F5.4 focus — `dioxus-pcb-view/evidence/24-module-source-route-20261003/receipt.md` | Retained keyboard module-focus reveal, Escape/host-focus restoration without preference mutation. Actual finding-list pointer action, board switch and nonempty clearance remain open; decorative React marker clicking is not a parity requirement. |
| F7.3 assets — `dioxus-shared-viewer/evidence/public-case-first/F7.3-controls-20261003.md` | Generic module editor-route repair retained; separate STEP model rendering/visibility observed on 34799. Model pick/error/retry and broader lifecycle branches remain unqualified. |
| F2.3 / F7.5 — compact-shell `f23-public-qualification.md`; functional-controls `case/receipt.md` | Retained compact Escape/focus-return and invalid Case non-commit results. New desktop resizing browser journey and valid Case preview/release/history are pending. |

Receipt paths above are relative to `.scratch/`. They support only the stated branches, including unchanged-source reuse from 34800/34801. Final verdict: **Standards — no remaining new blocker. Spec — one P1 repaired in 9dd; listed functional qualification gates remain open.** Package/source clearance does not accept any open parent or waive its final joins, accessibility, lifecycle or release requirements.
