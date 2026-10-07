# Edit settlement: review and decision brief

Updated on 2026-10-07 after ticket 15 merged (`d13f0dd1`) and ticket 17's source
cleanup. The ticket Outcomes carry final checks and reviews. No decisions in
tickets 18 or 19 are resolved here.

## Completed work and review corrections

Tickets 13, 14 and 16 are resolved. The requested Astra/high review of 14 and 16 found real queue and draft bugs; the follow-up changes are `fb0138356` and `0cbab30a7`:

- Module placement failure restores the last committed draft to accepted coordinates, preserves newer typing, and displays the ticket's failure. Recovery does not itself mean the selected owner vanished.
- Authored body gasket inset, width and depth resolve independently; a queued width change preserves an earlier inset change.
- Add/remove opening, add/remove vertex, add mount and add suggested closure mounts apply their intent to accepted collections. Explicit “Adopt” actions still replace the requested collection. A queued opening/vertex field retires when an earlier removal shifts its target index.
- Library and setup-guide name fields preserve uncommitted typing after older landings. Escape followed by blur does not queue a canceled Library rename. Authored body numeric fields preserve newer typing too.

The regressions were observed failing before their fixes. A first module fixture reused an operation ID already consumed by its manually opened Session; that fixture was replaced with the real Runtime open path before establishing the valid failure regression.

Verification after correction: PCB browser **38**, Case browser **60**, Library browser **13**, page browser **42**, native Application **29** and Runtime **102** passed. Repository/doc-link checks, page typecheck, WASM test ownership and whitespace checks passed. The earlier full native check passed the workspace and footprint suites, then failed at the CAD STEP oracle's workspace discovery: this nested worktree's `cad/step-oracle` is associated with the outer checkout's workspace. The manifest exists; this is not a missing fixture. Targeted Clippy previously stopped at the existing Application `Resolution` large-enum warning. Those unrelated gates remain unchanged.

Astra/high Standards and Spec rereviews found no remaining concrete findings after the checked-binding correction in `5d3bce3f`. A separate Astra/high review corrected caller classification, direct-event strictness, typed-operation/Undo cost and the provisional ranking in the decision evidence below.

## Tickets 15 and 17: completed migration evidence

Ticket 15 moved the remaining Parts actions onto accepted-state resolvers and edit
tickets. Its final Parts browser suite passed 50 tests; Astra high Standards and
Spec reviews found no remaining actionable findings after fixes. The native CAD
volume failure was reproduced unchanged on pre-ticket dev and is recorded in the
[ticket Outcome](issues/15-parts-remainder.md#outcome).

The [cleanup audit](../../investigations/edit-settlement-cleanup-audit.md) now
classifies the final direct-event callers and retained state. Firmware tests use
real Session/Core state through the common adapter, and Runtime's fabricated
feature interception is removed. The replacement inventory distinguishes current
resolvers from command-building helpers and typed operations. Consult
[ticket 17](issues/17-cleanup.md) for its final validation status.

## Ticket 18: choose the enforcement boundary

The [four-option investigation](../../investigations/edit-event-restriction-options.md) distinguishes the cost and protection of keeping the API, allowing direct previews only, splitting operation types, and making direct events strict. The central limitation is that `EditResolver` can still return any command: restricting `Event::Edit` alone cannot prevent a resolver from returning an old whole-document clone captured outside its closure.

The first question is **what should be enforced**: preventing accidental direct product commits, or preventing stale whole-document payloads across every submission route? A reasonable staged recommendation is a real preview-only boundary for the direct event, followed by typed Core intents for selected concepts. That is a proposal for the human, not an accepted decision. A naming convention alone provides review pressure; runtime rejection or a restricted payload type adds actual enforcement.

Preserve ADR-0005's ordinary queued composition against accepted state. Session's captured gesture route, export and protected remap inputs retain their existing strict captured-revision checks. A global removal of revision refresh would break the tested direct-event queue behavior and needs an explicit decision about retained callers.

## Ticket 19: choose the first typed Core slice

The [resolver inventory](replace-document-resolvers.md) groups concepts, panels, existing typed operations, web-side domain rules and replacement cost. Usage frequency is inferred from workflows, not measured telemetry; all rankings are provisional.

For a bounded first demonstration, the initial recommendation is **wiring mode → `SetWiringMode`**: one board-scoped choice. Its inspected rules are modest: board existence, a default Matrix mode when configuration is absent, unchanged detection and configuration assignment. Toolbar transform drag releases already use accepted-state resolvers; they are distinct from Session's strict gesture route.

A wiring/net batch removes more duplicated rules but needs atomic identity and plan semantics. Outline version features remove substantial domain duplication but are a broader first effort. Project/board rename is the smallest implementation and removes less domain policy.

Typed operations can improve changed-ID precision and avoid unnecessary outline rebuilding, but they still store full-document Undo snapshots and Undo still recomputes. They do not alone reduce Undo storage.

The first human question is **which priority wins**: the smallest bounded demonstration, frequent everyday editing, or removal of the most duplicated/risky domain rule? After choosing that priority and the concept, decide whether to create a new effort/map and whether its scope is one operation or a cohesive group. A future import/recovery-only `ReplaceDocument` rule also needs explicit classification of bulk setup, scripts, generators, assets and assemblies; none is silently declared an exception here.

## Next discussion

Use the refreshed ticket-15/17 evidence and the frontier questions above for the next discussion with the human. Record an Answer only after the choices are made; write an ADR only if the resulting tradeoff warrants one. The investigations are preparation, not authorization to widen APIs or impose the proposed restrictions.
