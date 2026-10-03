# 08: Change the selected board's wiring mode

**Parent:** F5.2 — Electrical resolver and wiring summary. This child does not close F5.2/F5.3 or their acceptance joins.

**What to build:** The PCB Wiring Inspector offers the same Matrix and Direct GPIO mode selector as React. Choosing a mode updates only the selected board's existing electrical configuration through the normal accepted edit/history path; the displayed mode and resolver plan follow the accepted saved project.

**Blocked by:** None for private implementation after the capability gate below. Paired browser acceptance joins the current F5.1 PCB route and F5.2 resolver identity.

**Capability-level start gate:** The Editor's current accepted snapshot has a saved board-scoped `PcbWiringSource` with document/session, UI and normalized board scope, token/revision, active selection and scope generation; the existing Editor-lifetime `use_pcb_wiring_controller` resolves `ElectricalMode` from that board configuration and keys currentness by accepted revision; the PCB Inspector is mounted for the board/controller context; and the normal Runtime `Event::Edit`/`ReplaceDocument` path plus exact operation-ID allocation are already callable. This gate is source-present in the isolated base; the full F5.1/F5.2 parents do not need closure before the private child starts.

**Status:** draft; ready for agent after independent Spec and Standards review.

**Contract:** [F5.2c board wiring mode spec](../drafts/F5.2c-board-wiring-mode.md).

- [ ] Show the reference-labeled `Wiring mode` selector with `Matrix` and `Direct GPIO` values in board-level Wiring context; do not show it in switch or generic-part contextual inspectors. Controller selection continues to use the board-level panel.
- [ ] Read the visible value from the accepted selected-board `ElectricalBoardConfiguration`; do not optimistically present an unaccepted value.
- [ ] Commit a changed mode to only the selected board using the existing `ReplaceDocument` and normal edit/history flow. A no-op choice submits no edit. Create the existing default electrical board configuration when absent.
- [ ] Preserve every other board and all unrelated selected-board config fields, document data, pins, nets and project extensions.
- [ ] Reject stale UI requests after project/session/board/revision/workspace/selection scope changes, and admit edits only while the accepted source is saved and ready with no preview or active gesture.
- [ ] After acceptance, invalidate the old plan identity and resolve against the new accepted board revision/mode; preserve the existing Resolve retry and async stale-result behavior.
- [ ] Verify paired mode change, one revision/history entry, Undo/Redo, save/reopen and unaffected second-board configuration. Keep F5.2/F5.3/F5.1 and all parent joins open.
- [ ] Record exact source/build/project identity and carry RF-001/RF-006/RF-009; no API/schema widening.
