# Layout to-spec addendum — source/browsing reconciliation (2026-10-02)

This is an outside-repository audit note, not a change to the 62-parent task graph, issue acceptance criteria, run ledger, or RF register. It records actual route observations against TypeScript source `5a472a9426e6e38993361da402cd4ec730feb369` and Dioxus source `89b1de8a28fdf02db91d972c90a69235bfbbbffb`. Full evidence and screenshots are in [audit.md](audit.md).

## Corrected capability picture

F3.1 is not a blank-tree implementation gap. The current Dioxus route has board and physical-instance navigation, stable board/matrix/row/column/key/component entries, independent disclosures, row/column grouping, real-cell component children, semantic empty-cell context, a private scope-aware tree/canvas selection adapter, selected-context summaries and arrow nudging. The browser confirmed those public entries and contexts. The remaining T1-10 gate is end-to-end proof around all specified public fixture/scope/lifecycle/focus states and completion of F3.1 acceptance; it is not a reason to recreate the hierarchy or duplicate Session selection state.

The user-visible gap is the context-to-editor join. A selected matrix/row/column/key reaches the page as a valid private `TreeContext` and is shown in a summary, but it does not reach a matching property form. The right panel renders the generic real-part X/Y inspector only when Session contains real selected part IDs. Empty cells correctly have semantic context but no fabricated part ID, so they need a form bound to their matrix/cell context rather than a fake “selected part.” The type and Core edit contract already exist privately/in Core; the adapter/UI is missing.

Other capability gaps remain separated by existing parents: F3.2/F3.2d matrix/cell structural edits; F3.3 transform, alignment, constraints and configurable snapping; F3.4 outline versions and authored geometry; F3.5 contextual property/relations/findings navigation; F3.6 fit/camera/3D assembly. The browser comparison does not justify combining them into one Layout rewrite.

## First demoable implementation slice to dispatch

**Matrix structure inspector: select one existing matrix, edit its name/rows/columns/pitch, and see the accepted matrix and tree/canvas update with one normal Undo/Redo path.** This is the smallest real end-user value adjacent to the reported blank inspector, and it exercises an already-present semantic selection and authoritative edit path without creating new entities.

Use the current private `selected_context: Signal<Option<ScopedTreeContext>>`, `TreeContext::Matrix { matrix_id }`, active `Scope`, and fresh accepted `ReadModel`. Resolve the matrix by its real matrix ID on each callback/admission; stale board/session/revision targets reject instead of retargeting. Emit the existing Runtime/Core `Event::Edit` with `EditOperation::SetMatrix` and accepted base revision/transaction. Let Core own validation, matrix resize/member preservation, history, and the resulting accepted document/scene. Render values from the accepted projection after completion; local drafts are only text-entry state and never the document source of truth. Existing application selection continues to contain only real part IDs. No schema/public API/Session field change, cloned mutable ProjectDoc, fabricated IDs, or mock-only controls are needed.

This is a bounded sub-slice of published F3.2d (`.scratch/dioxus-layout-authoring/issues/04-matrix-cell-inspector.md`), which already owns matrix dimensions/pitch/name and edit/history integration. The child remains open for assembly/switch/diode/edge-gap controls, Add row/column, Delete, Duplicate Design variant, key enabled/assembly/attached-component edits, mirrored-member behavior, and its full validation/save/reopen/theme/focus review matrix. Do not claim F3.2d acceptance from the first demo.

An appropriate seam is private to `web/src/presentation`: validated `ScopedTreeContext` + current scope/read projection in, typed matrix-edit intent out through the existing root-owned Runtime adapter. The panel should not own tree selection, session-selected IDs, a mutable document copy, or edit/history submission policy. Root’s shared composition author should own inspector mount/selection routing; a feature author should own a new private matrix form module only after the exact field intent is reviewed. The existing `context_summary` remains the honest fallback for unsupported context types until their owner ships.

## Reconciled existing slice recommendations

| Sequence | Existing owner | Small reviewable demonstration | Start/acceptance note |
| --- | --- | --- | --- |
| Baseline public context | F3.1 / T1-10 | Matrix/row/column/key/component/standalone tree and canvas selection; stale scope must not select into another board/project | F3.1 is started/partially delivered after accepted INT.1. Finish its existing public gates. T1-11 and T1-12 stay separate. |
| Matrix structure and cell setup | F3.2d issue 04 (under F3.2) | Matrix name/dimensions/pitch, resize, accepted projection, Undo/Redo | Published child has F3.1 start gate. Full F3.2d acceptance is broader than the first demonstration. |
| Transform and snap | F3.3 | Column splay/origin plus one configured canvas snap path; Core is authoritative | Existing dependency requires F3.1 and F3.2; not blocked by F3.4/F3.5 whole-parent completion. |
| Outline navigation | F3.1 / T1-11 | Generated/saved version entries, activate version, bridge selection/navigation with scope checks | Named ready ticket, navigation only. |
| Outline editing | F3.4 | Copy generated outline, activate copy, move one perimeter point, save/history, show accepted findings | F3.4 starts after F3.1; no need to wait for all F3.2/F3.3 unless the exact shared snap interface is required. |
| Contextual inspector routes, relations and findings | F3.5 | From a selected real part or semantic matrix/key context, open correct property/relationship/finding target and return with focus | F3.5 starts after F3.1. Keep explicit property forms owned by F3.2d/F3.3/F3.4 to avoid duplicating controls. |
| Fit/camera/3D Layout | F3.6 | Fit board/selection and a current Layout assembly view | F3.1 + F7.1 start, F7.3 acceptance join. Keep shared viewer owned by F7. |
| Modifiers/range | F3.1 / T1-12 | Rectangular anchored shift range plus Ctrl/Cmd toggle and cancellation scenarios | Exact behavior stays in its separately reviewed ticket; do not silently fold it into the first matrix form. |

## Dependency reality

INT.1 is recorded accepted and F3.1 is the only start gate for the existing T1 children. There is no new cross-parent blocker revealed here. F3.2d and F3.2 require F3.1; F3.3 requires F3.1 and F3.2; F3.4/F3.5 require F3.1; F3.6 requires F3.1 and F7.1, with F7.3 as acceptance join. F3.7 is later integrated parity acceptance with its recorded F2.3/F6C.3 joins. The reported empty property surface is a concrete capability gap, not approval to bypass those canonical gates or to call a parent accepted early.

Keep all 62 parent IDs, dependency edges, acceptance joins, existing issue bodies and RF history unchanged. Preserve the current evidence/check limits from audit.md; do not infer full parity from these screenshots.
