# F8.3a — full and draft KiCad board handoff

**Parent:** [F8.3 and F8.6](08-export.md), [private coordinator](08-export-coordinator-dispatch.md). This is one mounted selected-board provider slice; it does not accept any F8 parent.

**State:** integrated candidate 34769 passed the actual full and draft downloads on the retained fixture; a source-visible assembly heading delta was corrected and the full handoff's `ASSEMBLY.md` was rechecked byte-for-byte against React on 34770. The pinned React baseline, 34769 pair and 34770 correction leg are recorded in the [receipts](../evidence/export-pcb-handoff-20261003/REACT-RECEIPT.md). The corrected draft click was not repeated; full/draft inner board ZIPs and reports matched at 34769.

## Source-backed behavior

Pinned React source is `5a472a9426e6e38993361da402cd4ec730feb369`, principally `app/src/exports/pcb.ts`, `electricalPlanContext.ts`, `electricalHandoff.ts`, `assets.ts` and `export.worker.ts`. The Export route has two selected-board rows. Full KiCad requires board readiness and error-free wiring; draft requires board readiness and retains incomplete wiring findings. Both output `<project-name>[-draft]-pcb-handoff.zip` with a nested `<board-name>-kicad.zip`, `wiring-report.json`, `ASSEMBLY.md`, optional per-population jumper diagrams, and model bytes used by the board.

The workflow resolves selected-board wiring, rejects full-mode errors, commits wiring only when the accepted document does not already contain that exact generated plan, adopts the accepted result into the same operation, and resolves the plan again. It then captures board-used model bindings and contours, asks existing Core `PrepareExport`/`FinishExport` for the board, runs only Core-issued Ergogen jobs through the existing preview-generator, resolves each physical population, and packs the nested board and handoff archives through Core. Electrical handoff protection occurs only after both archives succeed. The final download is guarded by the original export owner, current accepted lineage, scope, Session executor epoch and Core worker identity.

The export uses the existing Session/Core/persistence/history owners. The exact API,
accepted-token transitions and remaining proof are indexed once in
[BND.2 source reconciliation](../evidence/planning/boundary-gaps.md#session-owned-export-commits-implemented-path-and-remaining-proof).


## Acceptance for this child

- Both mounted actions follow the exact selected-board readiness rules and produce real ZIP downloads with the pinned names and `application/zip` media type.
- Full wiring errors stop before mutation or download; draft retains them in the report.
- Accepted wiring materialization precedes serialization; handoff protection follows successful packaging and is persisted through Session.
- The inner KiCad ZIP carries Core output plus only board-used referenced model assets; handoff report and population artifacts retain selected-board wiring data.
- Cancellation, accepted-project/board change, worker replacement, or a late reply suppresses delivery. Failed packaging cannot protect the handoff.
- Verify one actual changed paired full/draft journey on the retained fixture and reuse unchanged footprint/preview-generator/Core/archive evidence. No exhaustive export matrix or new routine UI test suite.

## Limits

The 34769/34770 receipts provide actual Dioxus full/draft downloads and compare ZIP entries, report semantics, nested board bytes and corrected assembly heading against React. The retained fixture already had generated wiring applied, so those downloads do not qualify this child mutation's own-apply branch, accepted history/undo/reopen, failed-package-before-protection, or stale/external-mutation behavior. The route-wide failure/race matrix and every other F8 provider remain outside this bounded child; F8.2, F8.3, F8.6 and F8 remain open until their full criteria maps and joins are satisfied.

The preview-generator has no per-request cancellation. Session owner checks suppress stale results and downloads, but an in-flight synchronous conversion is not promised to stop promptly. Keep this limit under RF-010.
