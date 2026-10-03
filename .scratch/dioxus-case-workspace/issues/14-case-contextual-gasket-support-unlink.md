# 14: Unlink a selected Case gasket support

**Parent workflows:** F7.2 Case contextual workspace, F7.4 mechanical settings, and F7.5 Case viewer direct manipulation.

**What to build:** In the Case viewer's `Edit gaskets` toolbar, a designer can unlink the currently selected generated gasket support from its mirrored support pair. The selected support and its paired support keep their positions, dimensions, identities, and upper/lower pad geometry while their existing saved anchors become unlinked.

**Blocked by:** Issue09 (current Case support selection and Inspector route) and Issue13 (support projection and MechanicalSettings request/outcome controller). The Case shared viewer is already mounted in the source baseline; do not wait for all F7.3 direct-manipulation acceptance.

**Capability start condition:** Current generated support rows, exact MechanicalSettings owner identity, and the mounted Case shared viewer are available for the same accepted Scope.

**Status:** isolated source implementation and the assigned page-only strict Clippy check passed. The original5b React reference journey and integrated candidate `a76fa2bd` both passed the bounded toolbar unlink, Undo/Redo, and portable save/reopen journey. See the [implementation handoff](../evidence/14-case-contextual-gasket-support-unlink/implementation-handoff.md) for archive and screenshot receipts. Consolidated Sol review remains with the coordinator. This ticket does not close Issue07, F7.5, F7.2/F7.4, F7.3, F7.8, or any canonical parent gate.

**Reviewed draft:** [Case contextual gasket support unlink](../drafts/14-case-contextual-gasket-support-unlink.md).

- [ ] The Case viewer toolbar exposes `Edit gaskets`; while active with a selected current support, it exposes the reference action `Unlink selected support`. Preserve this viewer-toolbar placement; do not substitute an Inspector-only action.
- [ ] Unlink marks the selected support and its reflected `pairId` partner `unlinked=true`, preserving anchor ID, location, length, width, outline identity and other saved metadata. Linkage means the reflected/mirrored support pair. Each support still provides its matching upper and lower pads; those pads are not split by unlinking.
- [ ] Persist through the existing MechanicalSettings controller, captured current owner/Scope guards, instance/canonical configuration mapping, and normal document history. Undo restores the pair, Redo reapplies unlink, and save/reopen retains it.
- [ ] Prior geometry, stale/mismatched support identity, changed token/revision, wrong workspace/instance, non-editable owner, and busy field state cannot submit or apply an old unlink request. Feedback is tied to the exact unlink request.
- [ ] Keep the React wording `Edit gaskets` and `Unlink selected support`. Do not add reset-position, drag, relink, CAD/provider logic, format/API authority, or a second history owner; Issue07 remains responsible for handle and mount gesture behavior.
- [ ] Reuse the existing affected check once under the assigned build lease and verify the changed unlink action on the integrated candidate. No routine UI test or broad matrix is added.
- [ ] Preserve RF-001/RF-006 history; no new refactoring finding observed in this bounded slice.

**Parent gates unchanged:** Issue07 and F7.5 retain their complete gesture acceptance; F7.2/F7.4/F7.3/F7.8 joins and the canonical 62-parent graph remain open and unchanged.
