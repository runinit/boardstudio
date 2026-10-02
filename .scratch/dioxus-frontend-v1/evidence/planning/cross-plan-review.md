# Cross-plan review: F4, F9, and EXECUTION

## Material findings

1. **The execution document names a missing authoritative graph artifact.** `EXECUTION.md` says `tasks.json` is the dependency authority and defines `start_after` versus `acceptance_after`, but `/tmp/boardstudio-workflow-plans/tasks.json` was absent during this review; the available per-workflow JSONs still encode broad symbolic `depends_on` entries. The coordinator has said normalization is in progress, so this is a handoff completion item rather than an objection to the task plan. **Fix:** publish the normalized graph before dispatch and ensure each F4 finding in `reviews/F4.md` becomes a narrow start gate or acceptance join there.

2. **F4 and EXECUTION do not yet make the common-viewer handoff explicit at the Parts preview slice.** EXECUTION assigns one viewer implementation to F7 and F4 only provides isolated preview inputs, but F4.4 owns a 3D preview/lifecycle acceptance with no F7 viewer join in its task record. This overlap can result in F4 and F7 implementing separate render paths or one accepting behavior without the shared viewer. **Fix:** explicitly link F4.4 3D acceptance to F7's viewer/consumer task and keep 2D library preview unblocked; see F4 review finding 3.

## Verified alignment

The cross-workflow ownership map keeps F4 module definitions/profiles, F5 mounted-module wiring/physical instances, F6 firmware/keycap consumers, and F7 physical Case/common viewer responsibilities distinct. F9's eventual joins cover all workflows without locking early fixture work, and the execution document preserves independent review, bounded concurrency, coordinator-owned shared files, and explicit cutover approval.

**Scope/limits:** Read-only planning review. No builds, browser runs, or repository edits.
