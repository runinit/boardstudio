# F5.2a: PCB wiring preview and selected-switch context

**Parent:** canonical F5.2, “Electrical resolver and wiring summary.”

**Status:** ready-for-agent

**Blocked by:** none for private implementation. Existing `ResolveElectrical` and Runtime/CoreWorker identity reads support this bounded child. Public paired acceptance requires the F5.1a real PCB scene and hit/selection route to be mounted.

**Contract:** [reviewed source-grounded contract](../drafts/F5.2a-wiring-preview-and-switch-context.md). Independent Spec and Standards clearance: `/tmp/frontend-parity-reset-20261002/pcb/wiring-preview-contract-review.md` and `/tmp/frontend-parity-reset-20261002/pcb/wiring-contract-standards-review.md`.

**Evidence:** [paired reference and source audit](../evidence/wiring-context-20261002/audit.md), with isolated React snapshots/screenshots and the candidate placeholder observation in the same directory.

- [ ] Automatically resolve the accepted project/board plan on first ready and accepted document/board changes; keep request/result lifetime independent of switch selection. Manual Resolve is retry/refresh.
- [ ] Show the current Core plan’s controller, mode, matrix/direct assignments, used/free pins, peripherals and diagnostics. Match the React controller-candidate predicate (`Controller` definition or generator source containing `"/mcu_"`); preserve Core rejection/error behavior for candidates it cannot resolve.
- [ ] Selecting an accepted switch shows its inherited wiring and named terminals using only the first matching selected-board net; returning to Wiring clears Session selection/context without editing `ProjectDoc`.
- [ ] Suppress stale async results across project/session/board/worker changes. Selection changes on the same board do not duplicate resolution or stale the board plan.
- [ ] Read-only preview only. Apply, edit controls, net mutation, protected remap, connection replacement, firmware and module behavior remain in their existing parent tickets.
- [ ] Complete paired browser evidence, independent Spec/Standards review and required RF-009/RF-001 handoff. F5.2/F5.3 acceptance and joins remain open.
