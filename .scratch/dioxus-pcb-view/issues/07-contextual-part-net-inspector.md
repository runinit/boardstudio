# 07: Contextual PCB part connections Inspector

**Parent:** F5.3 — Manual pin review, nets, and protected handoff. Related acceptance join: F5.2 current accepted board/selection projection.

**What to build:** Selecting a supported PCB component opens the matching reference-style Inspector for its electrical connections. The designer can map supported named terminals and eligible plated pads to existing board nets, create a named net, and use the same normal project edit/history path. Built-in switches remain inherited/read-only and continue to offer Edit board wiring; they do not gain a second per-switch electrical editor.

**Capability-level start gate:** This child may start without F5.1/F5.2 parent closure once the exact Editor Inspector mount proves: (a) a current accepted PCB `PcbWiringSource` carries the board scope (`instance_id = None`), UI scope, accepted document/token/revision and selected part ID, and the selected part belongs to that board; (b) a selected non-switch/non-controller definition currently resolves to the empty `Unsupported` Inspector branch; and (c) a private controller can submit one immutable `ReplaceDocument` proposal through the existing `Runtime::submit(Event::Edit { .. })` flow while registering/retaining the exact `OperationId` outcome slot. Current source receipts and paired reference journey: [`part Inspector journey`](../../evidence/wiring-nextfrontier-20261002/part-inspector-journey/part-inspector-journey.md). There is **no reusable PCB generic-part edit callback today**; this slice must add its narrow private owner/callback at the exact Inspector mount. Public paired browser acceptance still joins F5.1 scene/selection and F5.2a accepted-board identity. The broader parents remain open.

**Status:** ready-for-agent

- [ ] On a supported selected non-switch component, show its current terminal/net assignments in the PCB Inspector; keep built-in switch selection in the existing inherited named-terminal view.
- [ ] Map a supported named terminal or eligible plated pad to an existing net, including the reference's filtering/eligibility rules; do not expose unsupported or non-plated pads as assignable.
- [ ] Create a named net in the selected board's existing net set and map a selected supported terminal/pad to it through normal accepted edits.
- [ ] Preserve unrelated nets and pins, use ordinary validation/save durability, and verify Undo/Redo and save/reopen for one mapping journey.
- [ ] Keep selection, board identity and edit outcomes bound to the captured accepted Session/source; reject stale board/selection callbacks and do not create a second selection or document authority.
- [ ] Test the production proposal mapping against the TypeScript rule: remove only the selected part's target pad IDs from every prior net, add them to the chosen board net, preserve every unrelated pin/net, and add a new net to the selected board's `netIds`. Register the exact outcome slot before submit; do not infer admission/durability from a later document revision or unrelated runtime status.
- [ ] Match paired React placement, labels, default expansion, keyboard operation and compact-panel scrolling. Compare same-project browser journeys, visible values, changed document and revision/history.
- [ ] Do not close F5.3, F5.2 or their existing parent acceptance joins. Preserve RF-001/RF-006/RF-009 handoff and exact browser/build/project provenance.
