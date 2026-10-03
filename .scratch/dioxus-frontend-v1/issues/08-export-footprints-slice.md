# F8.2a — standalone KiCad footprints Export action

**Parent:** [F8.2 private export coordinator](08-export-coordinator-dispatch.md) and [F8 export](08-export.md). This child is a runnable provider slice, not F8.2/F8.3/F8 acceptance.

**State:** bounded child qualified on candidate source `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd` (34767); this does not accept F8.2/F8.3 or any F8 parent. The durable in-worktree browser receipt and archive identity are recorded in [the receipt](../evidence/export-footprints-20261003/RECEIPT.md); the ZIP remains retained outside Git at its recorded path/hash. No new ordinary-UI test suite.

## Pinned behavior and provider boundary

Pinned React source is `5a472a9426e6e38993361da402cd4ec730feb369`, `Workbench.tsx` row `KiCad footprints`, `exports/documents.ts` and `export.worker.ts`. The row is ready when the current committed project has one or more definitions. It exports every project definition into `<project-name>-footprints.zip` (`application/zip`). The archive contains one `BoardStudio.pretty/<safe-name>.kicad_mod` per exported definition, `fp-lib-table`, `BOARD-UTILITIES.txt`, and referenced STEP/STP/STL/WRL models under content-hash paths. Standalone exports use the existing Core `PrepareExport` / `FinishExport` target `standalone-footprints`; Ergogen conversions use the existing preview-generator worker; final files are packed with the existing Core `PackFiles` archive provider. When multiple definitions sanitize to the same filename, Core now appends a deterministic suffix derived from each definition ID and gives the internal KiCad footprint the same unique stem. Unambiguous legacy filenames stay unchanged.

The Dioxus action captures the accepted document/snapshot and current session/scope when clicked, reuses the existing `Session::StartExport`/`CancelExport`/`RunExport`/`DeliverExport` lifecycle, and checks the captured scope, accepted token/revision/document/session and Core worker identity/epoch after each asynchronous boundary. Cancellation, supersession, project/board switch, or worker replacement prevents delivery. It does not edit or protect the project document. Browser URL lifecycle and global error reporting reuse the existing delivery owner.

The preview-generator client is shared with existing preview consumers and has no per-request cancel message. A canceled/superseded Ergogen conversion can finish in that worker; the captured export owner suppresses its late result and download. This preserves the existing worker contract and does not promise prompt CPU preemption.

On the retained Sofle fixture, pinned React currently rejects the four `switch mx` definitions because they map to the same `switch_mx.kicad_mod` archive path. The Dioxus/Core exporter resolves this collision deterministically so each authored definition remains available; this is a bounded functionality correction to the reference's observed error, not a claim that React delivered a successful baseline ZIP.

## Acceptance for this child

- The mounted Export row is enabled exactly when at least one definition is present and the document has a current accepted snapshot; it invokes the current Export action.
- With the pinned accepted project, clicking it downloads the correctly named ZIP with ZIP signature and expected media type. Archive entries correspond to Rust `FinishExport` output for every definition, with library table, utilities note, and current model paths/assets.
- An empty-definition state stays disabled and explains the prerequisite. Source/document remains unchanged by the export.
- Reuse the existing stale/cancel lifecycle; do not add a parallel document/provider authority or public contract.

## Boundaries and remaining work

This child does not complete generic F8.2 coordination, full/draft KiCad, model-asset parity for unrelated formats, firmware, portable archive, case/mechanical/keycaps outputs, stale-race matrix, or any F8 parent. The JS preview-generator remains the existing Ergogen conversion provider, invoked only for its accepted Core plan jobs. The browser remains responsible only for IndexedDB asset retrieval and file delivery. Any missing model-binding or file-path capability must remain an explicit blocker rather than being approximated.

The shared Ergogen worker's missing per-request cancellation is retained as a continuation of [RF-010](../refactor-findings.json); the adapter suppresses stale delivery but does not promise to stop in-flight CPU work. Its Core output-name correction is recorded under RF-008 and the [architecture boundary](../../../docs/architecture.md). No new finding is introduced beyond RF-001/RF-003/RF-004/RF-008/RF-010.
