# Review wiring clears authoring context

Reference: React `5a472a9426e6e38993361da402cd4ec730feb369`, `Workbench.tsx:1069`, clears the Inspector scope and selected IDs.

RED reproduced on source `99ec041a2895e5ab23880be25501beb9360487db`, public candidate34763: selected Board2 Generated outline → Export → Review wiring → Layout. The old `.m1-outline-inspector` reappears with Board2 and Copy outline, although Review wiring cleared part IDs. Accepted revision8 remained unchanged. Root re-read the resulting DOM before changing source. Sol classified this exact retained private context as P2.

Correction: Review wiring clears `SelectionAdapter.selected_context` and its anchor scope; it also clears a retained Session selection anchor when present. Existing Session selection, workspace routing and Inspector pinning remain the owners. No public API or durable model change.

GREEN for this changed journey is pending the next combined public candidate. No duplicate neighboring export/history matrix is required.
