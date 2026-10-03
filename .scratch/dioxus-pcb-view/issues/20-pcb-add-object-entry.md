# 20: Place a component from the PCB Add object inventory

**What to build:** Restore PCB Add object with current-board search/categories/layout target/Back to objects/Browse all parts and reuse the existing Layout placement workflow for the selected component.

**Blocked by:** None (ordinary component preparation/placement is already proven; board creation19 is independent).

**Status:** implementation-in-progress

- [ ] PCB Add object opens a contextual inventory rather than a generic Layout tree.
- [ ] Search/categories/layout target use the accepted catalogue and current PCB board.
- [ ] Choosing a component reaches existing Layout placement; completing/cancelling retains the current-board/history rules.
- [ ] The pending component has the reference placement instruction, including target layout where applicable and Click/Enter/Esc actions.
- [ ] Back/Escape and Browse return through existing owners; retained stale action cannot retarget a newer project/board.
- [ ] Missing broader Layout/Board geometry menu capabilities remain explicitly tracked, never represented as completed placeholders.
- [ ] Changed paired journey, one combined affected check and consolidated review retained; no new mirrored tests.

Spec: [Board and object creation](../drafts/F5.1c-board-and-object-creation.md). All parent criteria/RF history remain; the existing generic-menu placement observation belongs to RF-001.

PCB component inventory is mounted through the existing chooser/placement owner; Add replaces the tree and Back/Escape restore Objects. Existing Layout/pair/matrix/script sections are retained where actually wired; PCB Layouts/Board geometry section parity is still open in the parent inventory. No placeholder completion is claimed.
