# 12: Review a protected PCB handoff before remapping

**Parent:** F5.3 — Manual pin review, nets, and protected handoff. Keep F5.1/F5.2/F5.3 and all 62 canonical parent acceptance joins open.

**Reference:** `app/src/ui/WiringPanel.tsx` conditionally shows the protected-handoff summary, a collapsed “Review PCB remap” disclosure, and “Start a new PCB revision”. `app/src/main.tsx` invokes the existing `review-electrical-remap` Core request with the active board, base revision, and protected fingerprint. The Dioxus board inspector currently exposes no equivalent action. Core's existing request checks the revision and fingerprint before clearing only that board's protection.

**Capability-level start gate:** The accepted PCB source carries the current saved document/session, board/UI scope, selection, revision token and scope generation; the board configuration already carries `protected_handoff`; the Editor-lifetime owner can admit that source and submit one normal `Event::Edit`/`ReplaceDocument` through exact operation observation. These capabilities are present. No F5.1/F5.2/F5.3 parent closure is required to implement the private child.

**Behavior:** In board-level Wiring context, show the protected revision summary and the reference disclosure only while the active board has a protected handoff. “Start a new PCB revision” is a deliberate confirmation by the user: revalidate the captured current source and exact handoff fingerprint, clear only that selected board's `protected_handoff`, and submit one strict-revision `ReplaceDocument` edit. Do not clear protection on selection, mode, pin, or Apply actions. Do not change assignments, locks, nets, other board configurations, or project fields. Dioxus uses the existing Session edit/history authority; it adds no Core request, public API, schema, or persistence shape.

**Boundaries:** This action authorizes a new editable hardware revision; it does not regenerate or certify PCB/firmware outputs, remove existing nets, or guarantee a later edit succeeds. The stored baseline fingerprint is the compare token. A changed fingerprint, board, document, session, revision, selection, workspace, scope generation, pending save, preview/gesture, or non-ready lifecycle rejects the callback without an edit. Show pending, saved, and failure feedback truthfully by the exact operation and accepted snapshot. Switch/generic-part Inspector contexts do not expose the board-level control.

**Acceptance:**

- [ ] Match React's protected revision summary, disclosure label and explanation; keep disclosure collapsed by default and use the exact “Start a new PCB revision” action label.
- [ ] Hide the control when the selected board has no protected handoff; preserve all other selected-board configuration fields, all other boards, and unrelated document data when it is used.
- [ ] Revalidate document/session/board/token/revision/UI scope/selection/generation, Ready+Saved lifecycle, no preview/gesture and current fingerprint immediately before proposing the edit. Stale, no-op, or already-cleared callbacks submit nothing.
- [ ] Register/observe the exact operation before submitting one normal strict-revision `ReplaceDocument`; show Saved only after the proposal is the accepted durable next revision. Rejected, cancelled, failed, or persistence-blocked outcomes do not clear the accepted protection.
- [ ] Keep the board wiring resolver, mode, pin locks, Apply, connection replacement, and protected handoff semantics separate. No new public API/Core request/schema or second history authority.
- [ ] Verify the production owner and exact operation settlement, then compare a protected same-board fixture in the packaged paired browser route. Exercise one revision/history entry, Undo/Redo, save/reopen, unrelated board state and stale fingerprint behavior. Public paired/build proof and all parent gates remain open until run.
- [ ] Carry RF-001/RF-006/RF-009 and record exact source/build/archive provenance; create no duplicate F5.3 parent.
