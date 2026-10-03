# 02: Edit the selected board's authored Case body stack

**Parent:** F7.2 — Authored Case body and stack editor.

**What to build:** In the Case workspace, users can add an authored plate, tray or lid to the selected board, switch among that board's saved bodies, and edit its supported dimensions, mounts and gasket channel. Empty and cross-board mechanical-configuration states explain what the user can do. When the selected board has an active matching generated mechanical stack, the generated assembly panel and note remain visible and the authored body editor stays hidden while those bodies remain saved. Committed authored-body edits use the existing Session edit/history path and remain distinct from generated mechanical output.

**Blocked by:** T1-01, “Isolate the existing workspace UI for parallel work” (INT.1; accepted).

**Status:** ready-for-agent

- [ ] On the pinned React fixtures, match the Case stack's body list, add-body defaults, supported body kinds, body selection, no-body copy and selected-board filtering through the public Dioxus route. If the selected board has a matching active mechanical configuration, render the generated assembly panel/note and keep its authored body records saved but hide their editor until F7.4 disables that stack. Do not expose simultaneous generated-stack and authored-body editing.
- [ ] Edit thickness, clearance, z offset and non-plate wall dimensions. Add, edit and remove mounting holes/bosses and add, edit and remove gasket channels with the same supported fields and validation behavior as the reference.
- [ ] Keep the Mounting and Gasket channel groups as native disclosures: Mounting starts open only when mounts exist; Gasket channel starts open only when configured. Preserve a user's chosen open/closed state across accepted edits until the selected body changes.
- [ ] Commit field edits through the existing `SetCase`/Session edit path so normal save, undo and redo behavior applies. Numeric drafts commit on blur/Enter and Escape restores the accepted value; invalid values remain visible with actionable validation feedback.
- [ ] Enter commits a numeric draft once; its resulting native blur does not report a second blocked edit while the accepted request is saving.
- [ ] Keep draft and result identity scoped to the accepted document, selected board, project session and currently supplied Case instance where applicable. A board/session change cannot submit an old body's edits into the new scope. Physical-instance selection/handoff remains owned by F7.7/F5.
- [ ] When mechanical configuration belongs to another board, show the reference mismatch explanation and “Show configured board” action alongside the selected board's authored body editor. Selecting it changes the shown board through the existing owner. Mechanical configuration editing and enable/disable behavior belong to F7.4; this ticket only preserves the required presentation join. Keep authored body records separate from `MechanicalConfiguration` and generated geometry. Retain Case iconography only as visual support; do not add a Case choice wizard. Do not implement mechanical stack configuration, viewer editing, generation/readiness or CAD here.
- [ ] Verify keyboard focus/order, compact and desktop layouts, light/dark appearance, empty/mismatch/invalid states and the relevant public save/history behavior against the same fixtures. Retain exact fixture/action and check results.
- [ ] Record “No new refactoring takeaway observed” for the private Case form/edit seam, or update an existing RF finding with source-backed evidence; do not start unrelated restructuring.

**Parent graph:** Canonical F7.2 `Start after: INT.1` (satisfied by accepted T1-01); `Acceptance joins: none`. It does not wait for F7.1, F5, F3, F4 or F6. F7.7 keeps the selected physical-instance Case projection and F5.6 join; F7.6 keeps preview/generation/export; F7.4 keeps the generated mechanical configuration editor. This child covers F7.2 authored body presentation/editing. F7.4 supplies mechanical configuration/disable behavior; its existing acceptance join to INT.2 and every other parent dependency stay unchanged. The canonical 62-task graph is unchanged.

**Suggested routing:** Luna Medium author/verifier; batch-eligible Astra review with other low-risk, independently owned UI slices.
