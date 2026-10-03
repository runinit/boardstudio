# 15: Port the Layout board Inspector

**Status:** integrated source; combined check and changed paired journey pending.

**Blocked by:** None.34762 entry/actions and34763 changed action styling are proven; no missing Core capability. Joined with Geometry scripts in one Layout contextual Inspector packet.

Spec: [Layout board Inspector](../specs/F31-board-inspector.md). Contextual behavior maps to F3.5/F3.1; integrated Layout qualification remains F3.7. Owner: Layout author; coordinator serially integrates the mount and runs one combined check/package.

- [x] Private Board projection uses the accepted board scope, readiness and live part ids; geometry scripts keep first-page precedence and component/matrix/outline inspectors retain precedence.
- [x] Board name action is tied to exact scope, accepted token/revision, Layout scope generation, current tree context and local generation; it uses existing ReplaceDocument history without a Core API.
- [ ] Candidate journey: empty and active Board context show reference guidance, Board name, Outline status and Placed parts count; generic Position is hidden.
- [ ] Candidate journey: blur/Enter rename commits once with trim; empty/unchanged/Escape retain the accepted name.
- [ ] Accepted rename updates selector/tree/Inspector and survives Undo/Redo/reopen on the pinned fixture.
- [ ] Paired candidate confirms stale context/revision callbacks cannot edit another owner.
- [ ] Combined affected check, changed paired journey and consolidated review retained; RF-001 updated or no new takeaway recorded.

Routed PCB reference import/settings remain a separate recorded gap. The author adds only the private feature module and minimum Layout composition; root serially joins and runs the assigned combined check/package. This child cannot accept F3.1/F3.7 or waive any other parent criterion. No new UI tests or heavy commands are assigned. RF-001 is the only relevant refactoring finding; no new finding observed.
