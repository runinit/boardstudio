# 15: Port the Layout board Inspector

**Status:** planned

**Blocked by:** Current coordinator PCB/empty-board packet qualification (one waiting packet); no missing Core capability.

Spec: [Layout board Inspector](../specs/F31-board-inspector.md). Primary parent F3.7; contextual navigation joins F3.1. Owner: integration coordinator, after issue21 of the PCB queue qualifies.

- [ ] Board selection and empty selection show reference guidance, Board name, Outline status and Placed parts count.
- [ ] Blur/Enter rename once; empty/unchanged/Escape retain accepted name.
- [ ] Accepted rename updates the selector/tree/Inspector and survives Undo/Redo/reopen on the same fixture.
- [ ] Stale context/revision callbacks cannot edit another owner; component/matrix/outline precedence remains intact.
- [ ] Combined affected check, changed paired journey and consolidated review retained; RF-001 updated or no new takeaway recorded.

Routed PCB reference import/settings remain a separate recorded gap. This child cannot accept F3.1/F3.7 or waive other criteria.
