# PCB08 independent source review

Exact reviewed source: `67dcecbb9883e4048c9e95ef2faf944cb00b56db`, clean isolated worktree `pcb08-input-generator-controls-20261002`. Exact packet base: `c701233ea7bab6a4c794cba38cabf13be3a0b498`; review diff `67dcecbb^..67dcecbb`. Requested independent Sol6.1 High review, both axes; no additional subagents or application edits.

## Standards

**Bounded source integration clear; zero blocking source findings.** The private Editor hook owns preparation, pending state and the retained exact operation outcome slot. Preparation sets its busy flag before spawning, rejects competing edits, checks the hook lifetime before touching Signals after the schema await, and rechecks live workspace, scope generation, selected part, accepted session/token/revision, board, generator source and executor identity before registering/submitting. Settlement stays above the Inspector leaf and consumes only the observed exact outcome. Existing SetInputScanMode and ReplaceDocument paths preserve Core ownership and schema/API boundaries. The packaged schema remains owned by the Parts module loader. No weakened assertions, skipped required tests, suppressions, unrelated provider changes or API widening found.

Whole part/definition/override copies in projection follow the already retained RF-001 repaint observation. Their cost remains unmeasured; no new structural refactoring ticket is required for this bounded source packet.

## Spec

**Bounded source integration clear; zero blocking source findings.** The projection and admission preserve controller/whole-switch routing before generic controls, the exact EC11/EC12 fallback, Matrix membership AND independent-press eligibility, Direct/Unassigned choices, selected-board net discovery/order, nonterminal net parameter filtering, Default removal and finite independent anchor coordinate editing. Ordinary edits use the accepted source and exact observed operation, with no auto-apply of board wiring. The reference guidance is retained. Schema defaults are not written as explicit part overrides, matching the reference Default behavior.

Anchor code is conditional only: the current 36-generator package exposes no anchor-typed entries, so no reachable package-backed anchor/browser acceptance is claimed. The controls' final placement and visual/keyboard behavior still require the integrated paired Inspector journey.

## PCB07 integration dependency

The PCB08 base predates PCB07: merge-base with repaired `c0a0b1c6` is `d425a2dfb56ac2d5576da076bed11272437f1ea2`. Its `SelectionView::PartSettings` replaces the former generic Unsupported branch. Therefore integration must combine PCB08 controls with PCB07's GenericPart connection surface; taking this file wholesale would lose the connection editor. Preserve PCB07's actual generator-source classifier, source/document revalidation, hook lifetime guard and live-workspace check. PCB08's added `scope_generation` must remain passed through the accepted source constructor and equality. This source clearance does not review a conflict-resolved integrated join; that exact join requires checking both mounted surfaces and source checks again.

## Executed checks and evidence limits

Independently on exact source:

- `pnpm run test:web:physical-setup`: actual headless Chrome run, **6 passed / 0 failed / 0 ignored / 5 filtered**. Four projection/eligibility/schema-filter tests and two real packaged-module tests, including Gateron normalization and SMD0805 descriptor/default values. No Editor callback/history test is part of these six.
- `cargo fmt --manifest-path web/Cargo.toml --check`: passed.
- `git diff --check 67dcecbb^..67dcecbb`: passed.
- Worktree clean and HEAD matches frozen SHA after checks.

Author strict locked page/all-target WASM Clippy is retained in `.scratch/dioxus-pcb-view/evidence/pcb08-implementation-20261002/implementation-status.md` and reused for this narrow source review. The prior Node runner false green remains disclosed: it skipped browser-only tests; the new HTTP/CORS Chromium runner executed the six tests above. The broad-filter preview-worker failure also remains disclosed. Neither historical result is presented as passing evidence.

The paired-walkthrough is a plan with open steps, not executed public acceptance. Keep real mounted UI/edit/history/Undo/Redo/save-reopen, delayed-schema teardown/selection/workspace exercise, source-stamped generic binding fixture, no unrelated board/instance changes, keyboard/theme/compact/accessibility and full parent gates open. Root public34730 excludes PCB07/PCB08/Parts12. Do not close F5.1/F5.2/F5.3/F5 or the ticket from this review.

No new refactoring takeaway observed beyond existing RF-001/RF-006/RF-009. Preserve those IDs and exact build/project/test provenance; no deferred correctness waiver.
