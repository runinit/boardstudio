# 13: Resize a selected Case gasket support

**Parent workflows:** F7.2 Case contextual workspace and F7.4 mechanical settings.

**What to build:** Selecting a current resolved gasket support in the Case Objects tree opens its `Gasket N` Inspector with the React `Cut length` and `Pad width` controls. From that selected-support context, `All gasket settings` returns directly to the Gaskets settings pane. Resizing a linked support pair updates both pads through the existing mechanical settings controller and document history.

**Blocked by:** Issue03 mechanical settings controller and the integrated Issue09 Case tree-to-Inspector route (both present in the source baseline).

**Capability start condition:** The current Case tree exposes exact generated gasket support rows, and MechanicalSettings owns a scoped request/outcome controller. This packet adds only the contextual form and the missing support-size patch to that existing owner.

**Status:** same-original5b-archive React/Dioxus linked sizing, Undo/Redo, reopen, and saved-anchor comparison passed on Dioxus candidate source `e4bad4a6f03fd5a76fabdfb1b33809f2cd106840`. A follow-up found React's `All gasket settings` navigation missing. Candidate `34751` exposed the button, but the existing scoped selection guard rejected its `gaskets` group target; the private route admission fix is now frozen for the next candidate. See [implementation handoff](../evidence/13-contextual-gasket-support-sizing/implementation-handoff.md), [public sizing receipt](../evidence/13-contextual-gasket-support-sizing/public-receipt.md), and [layout-return evidence](../evidence/13-contextual-gasket-layout-return-20261003/README.md). Parent acceptance joins remain open.

**Reviewed draft:** [Contextual gasket support sizing](../drafts/13-case-contextual-gasket-support-sizing.md).

- [x] Selecting a current lower or upper support shows `Gasket N`, `Assembly settings`, `Cut length`, and `Pad width`; values and support identity come from the current matching generated Case projection.
- [x] Cut length keeps the React minimum of 5 mm; pad width keeps the 0.5 mm minimum. Invalid edits remain correctable and do not persist. Blur/Enter commits once, Escape restores the accepted value, and pending/saved/failed feedback remains tied to the exact request.
- [x] A linked upper/lower pair updates both projected anchors. An unlinked support updates only its own anchor. Preserve anchor IDs, region, outline, position and linkage metadata; merge into the latest accepted mechanical configuration.
- [x] Commit through existing scoped MechanicalSettings controller admission, canonical/physical-instance update path, save ordering and history. Undo showed 70/3, Redo showed 75/3, and reload/reopen restored the 75/3 layout.
- [x] Stale/previous results cannot expose editable stale support values; selection and Scope changes cannot submit an old support into a new assembly. Assembly settings returns to the existing global Inspector.
- [ ] From the selected support Inspector, expose `All gasket settings` and route directly to the current Gaskets section through the existing Case selection callback; preserve the accepted Scope and leave the document/history unchanged.
- [x] Keep color/reset/visibility as view preferences. No second controller, draft/history owner, Runtime API, file-format change, gasket layout controls, resolver change or drag behavior.
- [x] Reuse the current Case verification/build evidence where unchanged; record the paired browser delta without adding a routine duplicate test matrix. Parent F7.2/F7.4 and F7.3/F7.8 acceptance joins remain open.
- [x] Refactoring ledger disposition: no new takeaway observed in this bounded workflow; preserve relevant RF-001/RF-006 history.

**Ownership:** private MechanicalSettings/contextual support leaf and its source projection. Root owns shared presentation/runtime composition, combined build and integrated browser evidence. No canonical parent edges change.
