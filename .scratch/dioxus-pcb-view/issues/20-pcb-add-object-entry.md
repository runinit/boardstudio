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

PCB component inventory is mounted through the existing chooser/placement owner; Add replaces the tree and Back/Escape restore Objects. The PCB Add menu now receives the existing Matrix Setup and Mirrored Pair owners. Each entry is enabled only for the captured current Ready/Saved PCB scope, and its click rechecks that accepted owner before switching to Layout and invoking the original owner callback. The existing Board outline and Geometry scripts entries use the same captured-current guard when invoked from PCB, then delegate to the accepted Layout owners. This exposes existing workflows without adding a second setup state or bypassing their Layout admission rules. Source integration and the changed paired journey are still pending.

The separately referenced “Mirror existing half…” action has no owner in this packet and remains open under the existing Layout/Add-object parity work. Board geometry action wiring is not yet publicly qualified in the changed PCB-to-Layout journey. No placeholder completion or parent closure is claimed.
