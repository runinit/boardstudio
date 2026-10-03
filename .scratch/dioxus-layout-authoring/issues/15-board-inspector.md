# 15: Port the Layout board Inspector

**Status:** implementation-in-progress

**Blocked by:** None.34762 entry/actions and34763 changed action styling are proven; no missing Core capability. Implement together with Geometry scripts in the next Layout contextual Inspector packet, preserving one waiting packet.

Spec: [Layout board Inspector](../specs/F31-board-inspector.md). Primary parent F3.7; contextual navigation joins F3.1. Owner: Layout author; coordinator serially integrates the combined contextual Inspector packet.

- [ ] Board selection and empty selection show reference guidance, Board name, Outline status and Placed parts count.
- [ ] Blur/Enter rename once; empty/unchanged/Escape retain accepted name.
- [ ] Accepted rename updates the selector/tree/Inspector and survives Undo/Redo/reopen on the same fixture.
- [ ] Stale context/revision callbacks cannot edit another owner; component/matrix/outline precedence remains intact.
- [ ] Combined affected check, changed paired journey and consolidated review retained; RF-001 updated or no new takeaway recorded.

Routed PCB reference import/settings remain a separate recorded gap. This child cannot accept F3.1/F3.7 or waive other criteria.
