# 14: Show and toggle mounted-module source footprints

**Parent:** F5.4; extends the existing PCB02 module Footprints criterion.

**What to build:** In PCB, show authentic selected-board daughterboard footprints
and independently toggle them through Mounted modules → Footprints, preserving
host visibility and saved project contents.

**Blocked by:** None for this bounded slice: the accepted selected-board PCB scene,
resolved module footprint geometry and shared transient layer control are mounted.
F5.1 and INT.2 remain full F5.4 acceptance joins.

**Status:** implementing — bounded source geometry/visibility journey passed on34748; remaining checks and label correction remain open

**Spec:** [mounted-module footprint visibility](../drafts/F5.4a-mounted-module-footprints.md).

- [ ] The module Footprints row is independently pressed/toggled, including on module-free boards.
- [ ] A real module-bearing fixture shows selected-host-board source courtyards, pads, drills, artwork and references with resolved pose/side applied once.
- [ ] Module visibility preserves host geometry; host copper/pads/holes/artwork/reference controls intersect as TypeScript does.
- [ ] Workspace/board navigation retains transient visibility; toggles leave document, revision and history unchanged.
- [ ] Run one affected compile/check and the changed paired browser journey; preserve accurate limitations and RF handoff.

Module owning-object navigation and other categories remain parent criteria;
there are no inert new controls for those unfinished paths. Source presence and
compilation do not close this ticket or F5.4.

Receipt: [paired mounted-module source geometry and visibility](../evidence/14-mounted-module-footprints-20261003/public-receipt.md). No parent acceptance is claimed.
