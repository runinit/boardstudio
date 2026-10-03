# 17: Choose a controller from PCB wiring

**Parent:** F5.2; completes the missing ordinary-board entry action in PCB03 context.

**What to build:** Add controller→Parts filtered to controllers→existing definition/settings→normal placement→return to this PCB, using an explicit accepted ordinary-PCB origin beside the existing guided origin.

**Blocked by:** None for capability start. Existing catalogue/placement/controller transactions are mounted. One waiting PCB packet is the dispatch limit; PCB16 is the current waiting packet, not an unrelated functional prerequisite.

**Status:** ready-for-agent

**Spec:** [PCB controller chooser](../drafts/F5.2f-pcb-controller-chooser.md).

- [ ] The no-controller Wiring pane shows the matching Add controller action and Parts filter/selection/back context.
- [ ] Browsing and cancel do not write a controller; selected definition starts the existing current-owner placement preview.
- [ ] Commit uses the existing controller edit/history/persistence path and returns to the owning PCB context.
- [ ] Project/board/session replacement retires the ordinary chooser and suppresses old definition/preview results.
- [ ] Setup-guide origin keeps its existing owner requirements and behavior.
- [ ] One affected compile, changed paired journey and consolidated candidate review are retained with all parent/RF limits.
