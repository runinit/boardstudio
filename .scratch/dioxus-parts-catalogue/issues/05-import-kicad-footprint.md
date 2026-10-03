# 05: Import a KiCad footprint into the active project

**Parent:** F4.2 — Create, import and edit footprint definitions. Preserve the parent’s INT.2 acceptance join and all existing F4 criteria.

**What to build:** From the mounted Parts library, a designer selects a `.kicad_mod` file and receives its source-backed parsed definition as the selected project part. Existing Parts details and preview show the accepted result.

**Blocked by:** None among local F4 child tickets. The page already has a mounted Parts action owner, an accepted document/scope, private CoreWorker access, the `ImportFootprint` artifact request and a normal accepted `ReplaceDocument` edit path. The import child provides its own selection/project guard; full F4.1/F4.2 acceptance is not a start lock.

**Status:** implementation candidate; paired acceptance open.

- [ ] Show a KiCad footprint file action in Parts even while the catalogue is loading or has failed. Accept supported `.kicad_mod` input, clear the file control after selection so the same file can be retried, and report read/parse/provider errors beside the action.
- [ ] Read the selected file and call the existing `ImportFootprint` artifact operation through a page-private Runtime adapter. Keep the parser and source projection authoritative; do not add an endpoint, parser, worker, or second geometry implementation.
- [ ] On a valid reply, create one collision-free project definition using the parser's identity, source, pads, courtyard, units and provenance. Preserve raw source exactly as returned by the importer, pad IDs and legitimate repeated KiCad pad numbers. Do not apply native custom-authoring uniqueness rules. Never regenerate imported pads from preview geometry.
- [ ] Admit one ordinary `ReplaceDocument` edit only while the captured Parts workspace, active project/session/scope, selection, accepted revision/token, and view generation remain current. Preserve unrelated accepted document data/assets; stale or cancelled work cannot commit or select a result.
- [ ] Show loading and cancellation while file reading/import is pending. Cancellation may leave the worker request to settle, but its result must be ignored. Accepted edit rejection or durability failure remains visible and leaves the prior document intact.
- [ ] After the accepted edit completes, select the imported definition through the existing Parts selection owner. Reuse the selected-detail and 2D preview surfaces, and keep source-owned geometry read-only in the existing definition editor.
- [ ] Verify normal Undo/Redo and save/reopen/archive round-trip for the imported definition, source identity and unrelated project data. Pair the exact pinned React and Dioxus import journeys through valid and malformed files, including compact/desktop and error recovery.
- [ ] Preserve F4.2 INT.2 and wider F4/F9 joins; this child does not claim F4.2 or F4 completion. Record relevant existing RF findings or explicitly state no new refactoring takeaway for the reviewed scope.

**Ownership and limits:** Parts owns file selection, pending/error/cancel state and the captured selection owner. The page Runtime owns only its private call to the existing artifact worker. Core owns KiCad parsing and geometry projection; Session/Core own document validation, revision, Undo/Redo and durable save. F4 custom geometry, model attachment, generator settings, and F3 placement remain in their existing slices.
