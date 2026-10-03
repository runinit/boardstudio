# 14: Show and toggle mounted-module source footprints

**Parent:** F5.4; extends the existing PCB02 module Footprints criterion.

**What to build:** In PCB, show authentic selected-board daughterboard footprints
and independently toggle them through Mounted modules → Footprints, preserving
host visibility and saved project contents.

**Blocked by:** None for this bounded slice: the accepted selected-board PCB scene,
resolved module footprint geometry and shared transient layer control are mounted.
F5.1 and INT.2 remain full F5.4 acceptance joins.

**Status:** geometry/visibility is bounded-paired on34748; matching Parts source navigation and guarded return are implemented in the current candidate packet, with the changed paired journey still open

**Spec:** [mounted-module footprint visibility](../drafts/F5.4a-mounted-module-footprints.md).

- [ ] The module Footprints row is independently pressed/toggled, including on module-free boards.
- [ ] A real module-bearing fixture shows selected-host-board source courtyards, pads, drills, artwork and references with resolved pose/side applied once.
- [ ] Module visibility preserves host geometry; host copper/pads/holes/artwork/reference controls intersect as TypeScript does.
- [ ] Workspace/board navigation retains transient visibility; toggles leave document, revision and history unchanged.
- [ ] Clicking, Enter-activating or Space-activating a mounted PCB module opens its exact source entry in Parts; the source inspector can return to the same selected placement Inspector without creating a placement or changing the document.
- [ ] Run one affected compile/check and the changed paired browser journey; preserve accurate limitations and RF handoff.

Other module categories and shared finding-focus behavior remain parent criteria;
there are no inert controls for those unfinished paths. Source presence and
compilation do not close this ticket or F5.4.

Receipt: [paired mounted-module source geometry and visibility](../evidence/14-mounted-module-footprints-20261003/public-receipt.md). No parent acceptance is claimed.
