# Consolidated candidate review — frontend-current-actions-20261004

**Verdict: pass for the scoped repairs; recommend accepting F5.2.** No P0/P1/P2/P3 actionable finding was identified. Source is exactly `3b5bb1b3651e2bbd35911e15f698995f02c1abed`, compared with prior accepted `916a40549444e7ac73c144e422f632a129fd2eaa` using `git diff 916a4054...3b5bb1b3`. Later export source, including `76e68748` and the overlapping runtime/handoff edits, is excluded. Only this consolidated review and its [audit](frontend-current-actions-3b5bb1b3-audit.json) were written; tasks/run/RF records remain coordinator-owned.

## Standards

No documented standards violation or actionable judgment-only smell was found. Current scope and generation fencing follow CONSTRAINTS.md; presentation retains local selection/view ownership and normal Session edit/history ownership. Layout Escape uses the existing selection intent, Case layer selection rechecks the live owner, and Parts leases reject obsolete generations. No unrelated public API/schema/backend change was introduced.

## Spec

No clear functional defect, omitted scoped requirement or scope creep was found. Layout relationship Escape returns only the matching component/scope and ignores text-entry/prevented events (`layout_workspace.rs:393–473`). PCB model options explicitly retain the available saved asset or present its unavailable state (`pcb_board_reference.rs:934–960`). Case settings prefer exact-current resolved rows and accept their click before viewport scene completion, with live identity checks (`mechanical_settings_mount.rs:562–603,947–975`). Parts synchronously selects the owner generation and scopes deferred lease invalidation (`parts_preview.rs:93–129`; `parts/preview.rs:131–163`).

The current [Case receipt](../../../dioxus-case-workspace/evidence/public-mechanical-settings/candidate-0cad7577/RESULTS.md) proves five current rows, Plate inspector selection, and removal of Main-board layer context after Board 2 selection on 34807/3b5bb1b3. Its earlier setup blocker remains historical and was resolved by the absolute-path import. The current [Parts receipt](../../../dioxus-parts-catalogue/evidence/parts-03-preview-34759/RESULTS.md) proves Front→Back→Front Apply remains in ready 3D without a manual toggle on that same candidate. Sequential browser settlement proves that journey; the focused lease regression supplies adversarial generation ordering.

The [Layout Escape receipt](../../../dioxus-layout-authoring/evidence/f35-matrix-properties-relations-20261003/source-implementation.md) and [saved-model selector receipt](../../../dioxus-pcb-view/evidence/24-module-source-route-20261003/receipt.md) qualify 6d3e89fc/34806; their production files are unchanged in 3b5bb1b3. Reuse is appropriate. Compact post-return focus, broader routes, directory attachment and other parent criteria remain open.

## F5.2 criterion and join assessment

All four criterion verdicts are supported by the [wiring receipt](../../../dioxus-pcb-view/evidence/pcb-apply-ui-20261002/receipt.md), retained paired archives, and current production source:

| Criterion | Assessment |
| --- | --- |
| C01 | Verified. Left changed Apply and Right populated idempotent Resolve/Apply preserve board scope. Production identity/late-reply admission and two mounted stale-board/revision/controller regressions supply unavailable forced-race evidence. |
| C02 | Verified. Paired controller, mode, 18 assignments, used/free pins, peripherals, diagnostics and no-controller states match. Readiness is conveyed by counts/diagnostics/action admission in both apps. Pending and production-mounted Failed notice/retry are explicit. |
| C03 | Verified. Current plan admission and Core materialization preserve revision/fingerprint checks; one Commit submits one board-targeted edit. Independently reread archives confirm paired board-state equality, one Undo restoring pre-Apply state, one Redo restoring applied state, and unchanged Right board. |
| C04 | Verified. Public Pending→current and board switch preserve documents. Deterministic mounted owner failure→alert→same-action retry/stale rejection and Apply rejected-edit feedback cover lifecycle branches allowed by CONSTRAINTS.md. |

The production Wiring renderer actually uses the tested alert/status notice and keeps Resolve available after failure (`pcb_wiring.rs:932–965`). Resolver production methods and Wiring/Apply presentation are byte-equal to the qualified 924ba282 source; changes in the controller since then are test-only. No public provider failure or forced stale reply race was induced, and neither is claimed.

INT.2’s accepted bounded seam join remains satisfied. Its [accepted review](../int2-acceptance-20261003/REVIEW.md) still hashes to `a18ed232e36c3df7b0460fd5ba3f82762eb54349796b10b4a42e5d7371403dd5`; candidate runtime/CoreWorker source is unchanged from 916a4054. F5.1’s consumed board/panel capability is established by the paired journeys; its broader open parent is a start prerequisite, not F5.2’s final acceptance join. Recommend coordinator acceptance of F5.2 only, with this review/audit pinned.

## Evidence limits and package audit

The new [CAD-worker characterization](../../../dioxus-keycaps-workflow/evidence/keycaps-cad-batch-cancel-20261003/receipt-34769.md) uses a synthetic Blob Worker with the production CadWorker client. It proves failure messages, request correlation, ignored late replies and STEP revision validation, not a genuine packaged CAD-kernel failure. STEP error/UI reconciliation must also use retained public empty-profile STEP error/restored-profile retry and the current Runtime report propagation; mounted preview retry alone does not prove STEP export UI. This citation precision does not block the scoped source or F5.2 recommendation. Full Keycaps and other parents retain their joins.

The [package proof](../frontend-current-actions-20261004/package-proof.json) pins the exact SHA: 1,393 recorded source inputs, zero source mismatches, nine fresh commands and 23 inherited commands, zero release warnings, and recorded HTTP 200/isolation headers for root and subpath. Provenance hash was independently recomputed as `90a71c8f61aea550160547fa9cbc58114ffe0e9576e41cc68dde80113f8df3e7`. All 12 inspected source files match it; all 190 local assets per route match their recorded hashes. Recorded build/test/browser results were inspected and reused; no browser, application test or build was rerun. Broad visual, compatibility, accessibility, performance, release and retirement gates remain with their owning tasks. No new refactoring takeaway beyond existing recorded ownership/lifecycle findings was observed.

Standards: 0 findings. Spec: 0 findings. Worst issue on either axis: none in this scoped candidate.
