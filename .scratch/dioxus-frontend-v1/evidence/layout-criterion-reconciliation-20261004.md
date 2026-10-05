# Targeted Layout criterion evidence reconciliation — 2026-10-04

Verdict: **F3.6-C03 and F3.6-C05 have sufficient attributable evidence for `verified`. F3.5-C05 retains a concrete Attached components option-parity gap; its successful edit/history/locality clauses are covered.** This is an evidence recommendation only. No task status or parent verdict was changed.

Reviewed HEAD `7f3165b497d135eafccbe8c3335cb48024fd62c1`; served candidate remains source `4e716156a0692972b1309798cdc883739c7ff73b` at 34822. Compared the actual task requirements/finish conditions, originating `issues/03-layout.md`, retained key Inspector and grouped receipts/review, and current F7.3 consumer reconciliation. No code edits, tests, builds, browser work or status edits were performed; only this report was written.

## Exact clause accounting

| Criterion | Requirement and evidence | Missing clause / recommendation |
| --- | --- | --- |
| F3.5-C05 | Key Properties provides Enabled, Key Assembly, Attached components replace/remove and mirror-target local override. `layout-key-inspector-gap-20261004/RECEIPT.md` records Key Assembly reference option parity on 34819 and catalog-only attachment replacement surviving reload. Grouped paired 34820/5175 evidence records mirrored reset/local behavior, target Choc, canonical attachment removal and explicit target retention. Final published f320e6b8 replay closes the selected-key disable/Undo defect, exercises attachment replacement/removal/Undo/Redo, normal reload and canonical independence. Native operation/locality and actual ordinary Core/Session selection regressions plus independent scoped PASS complement the public mirrored replay. | The finish condition requires matching reference option sets. Attachment Replace options remain broader than the reference, as detailed below. Keep this clause open; preserve completed action evidence. A new hook-mounted/public injected catalogue fault run is not required. |
| F3.6-C03 | Canonical selected PCB board, common F7 viewer, orbit/zoom/preset/fit/picking, with physical-instance reflection owned by Case. Grouped Layout receipt records rendered paired orbit, actual picks, candidate Top/Zoom/Fit actions, repeated return cycles and selected Right PCB replacing the linked Left pair with the correct single board. Published f320e6b8 replay confirms repaired selected-key/edit retention across 3D/2D. Shared ownership/control evidence is retained in the F7 consumer map and earlier paired shared-controls receipts. Current Layout capture selects from the accepted canonical document by `scope.board_id`; it does not switch to the Case physical document. | No missing clause in this bounded consumer criterion. Recommend verified. Broader renderer lifecycle/model/error qualification and F7.3 parent acceptance remain separate joins. |
| F3.6-C05 | Renderer live camera/GPU ownership separate from Session preset/scope; document a missing public adapter before widening visibility/contracts. The grouped independent source review expressly confirms private renderer/live-camera resources and host-owned controls, separate Runtime/Session scope and 2D camera; no missing adapter or unauthorized visibility widening was found. Current source retains the private host in SharedViewer and accepted scope identities. F7.3-C01/C09 reconciliation confirms one viewer/private DTO consumer wiring. | No missing clause. Recommend verified from the retained source review and checked freshness. F7.3 parent acceptance is not a prerequisite to recording this narrower ownership fact. |

The grouped receipt correctly keeps whole-parent Inspector cardinality/nested routes, locked/driven fixtures, fit envelopes, renderer lifecycle/model-delivery and performance work open. Those limitations belong to other criteria or parent joins; they do not add new clauses to the three criteria above. Mobile/compact validation remains deferred by user instruction; historical compact observations were not used as a current gate here.

## Concrete F3.5-C05 option-set gap

The criterion's finish condition explicitly says the controls must "match reference option sets." The earlier parity receipt describes **Key Assembly** options, not the complete Attached components Replace option sets.

Targeted current source inspection shows `matrix_transform_controller.rs:198` builds `component_choices` from every accepted document definition followed by the loaded catalogue, deduplicating IDs without applying the reference choice policy. `matrix_transform_inspector.rs:911` renders that shared list unchanged for each attachment. In contrast, `app/src/ui/MatrixInspector.tsx:89` calls `partChoices(definitions, assembly.definitionId)` for each row. `app/src/ui/partsCatalog.ts:24` excludes definitions with generator source `infused-kim/nice_nano_pretty` and excludes assembly snapshots unless their ID is that attachment's current definition.

The retained Sofle journey explicitly contains switch snapshot `assembly-preset-mx-hotswap-south-left-keys-0/definition/switch`. Rust's unfiltered document union offers that switch snapshot as an attachment replacement. For a diode attachment, the reference omits it because it is a non-current assembly snapshot. The same discrepancy applies to other non-current placed assembly definitions. This is a concrete source-confirmed option-set difference in the existing finish condition, not a new catalogue-fault requirement. Key Assembly already uses its separate filtered `switch_choices`, so its recorded parity is not invalidated.

This reviewer did not run a fresh browser option-list comparison. The source mismatch and receipt fixture identity establish the missing clause; a bounded future repair/qualification should reconcile attachment choices while retaining the currently assigned snapshot. No broad rerun of successful editing/history/locality actions is needed.

## Catalogue fault limit

The grouped source review found a real catalogue-failure defect that could permanently block existing key edits. It was repaired with native policy RED/GREEN, source-confirmed retry wiring, and independent final CLEAR; the published public edit/history/reload replay passed. Its author/reviewer accurately retain the limit that no hook-mounted/public catalogue fault injection ran. Neither F3.5-C05's requirement nor its finish condition requests a catalogue-fault journey. No new failure was reproduced after the repair. Preserve that observation limit with the repair evidence; do not turn it into an additional C05 acceptance test or repeat successful key-edit journeys.

## Source freshness for reuse

Read-only Git comparisons were made from original published repair `f320e6b8` to served `4e716156`, then from served source to HEAD `7f3165b4`. The relevant working-tree paths were clean.

- `presentation.rs`, `matrix_transform_inspector.rs`, `matrix_transform_controller.rs`, `matrix_transform_operation.rs`, `matrix_transform_lifecycle.rs`, `layout_viewer.rs` and `layout_viewer_source.rs` are unchanged across both comparisons. The key editing/history/locality and canonical Layout source/pick owners are therefore exactly the previously qualified source.
- From f320e6b8 to the served candidate, SharedViewer changed only its component-model row source selection: the Layout branch still selects its own leased preview models and never falls back to physical Case models. The old selector was extracted into `case_display.rs::component_models_for_source`, preserving Layout/Physical behavior and adding Parts. Camera controls, mount/currentness, pick and canonical scene ownership were unaffected. The removed inline test was moved into the native-testable policy module, not removed from coverage.
- From served `4e716156` to HEAD, SharedViewer adds only the already reviewed `cfg(test)` mount-error seed and equivalent non-test call/result binding. The non-test mount still uses the same host, inputs, sequence and guards. Runtime, renderer host files and `case_display.rs` are unchanged across this latter comparison. No changed release behavior invalidates these Layout receipts.

Selected current SHA-256 identities:

| Owner | SHA-256 |
| --- | --- |
| `web/src/presentation/layout_viewer.rs` | `d2560df45288646b938d00dd1842d1a65948cc063b3362c19b66f34bc797e7df` |
| `web/src/presentation/layout_viewer_source.rs` | `a38a0ad163c7f874457ac32e1ccd8b6347fd7cd1a89f60f1519a8b0f20e9e974` |
| `web/src/presentation/shared_viewer.rs` | `6d1b6bac802e9b9e8885505a9a4c685212ebecec1612b841e2a52e4bb4c6c56e` |
| `web/src/presentation/objects/matrix_transform_inspector.rs` | `ed4c2506a4cb86eb1656037c5a7d84d8ca2f95dc18370d1d3c77f8360e3fa95c` |
| `web/src/presentation/objects/matrix_transform_controller.rs` | `7e1e99a3d1fdd2d4042f2abcc2c954c6b5a3c653be6d64a9849582f6ae1ed4cb` |
| `web/src/matrix_transform_operation.rs` | `87b15981ae9c24451b97c9ea6d52e0c0f9648f979bab5d08f1dbca47750a5de1` |
| `web/src/matrix_transform_lifecycle.rs` | `a744cf9a9e43abed43a97f9b815f9db2d9d142a8dc1e4987cf6705ff7648cc30` |

## Criterion verification versus parent acceptance

F3.6 retains its explicit `acceptance_after: [F7.3]`. Verifying C03/C05 records their own satisfied scene/control/ownership clauses; it does not accept F3.6, accept F7.3, waive remaining lifecycle/fit criteria, or erase their original dependency rationale. F3.5 likewise remains subject to its other Inspector criteria. The newer F7.3-C01/C09 evidence reconciliation supports shared ownership/wiring without inventing an additional Parts sample-pick requirement. No parent approval is sought by this report.
