---
status: accepted
---

# Delete a removed half's exclusive PCB design

When a topology change removes a physical half with its own PCB design, delete that removed half's exclusive design rather than retaining it as an unused design in the project. The user chose deletion over retention on 2026-10-20 for the non-reversible split-to-unibody scenario. If another surviving physical instance uses the same PCB design, retain that shared design; removing one instance of a reversible PCB must not delete the survivor's PCB.

Accessories attached exclusively to the removed half are deleted with it, not kept unassigned or automatically moved to the survivor. The user confirmed this cascade on 2026-10-20. Delete exclusively owned dependent records and reconcile references without leaving dangling links; preserve shared designs, accessories, definitions and assets still needed elsewhere. The removal and its dependent cleanup must form one undoable edit, with Undo restoring the removed design, accessories and relationships.

This is a new product policy, not current implementation evidence. The existing [topology proposal](../../web/crates/catalogue/src/physical_setup.rs), `propose_topology`, replaces the instance list while retaining PCB designs. Implementing deletion must preserve shared/unrelated data and document integrity through the normal edit, saving and Undo boundaries. This decision does not authorize indiscriminate asset reclamation or bypass [ADR-0019](0019-reject-stale-saves-and-reclaim-assets-conservatively.md)'s retention of assets needed by Undo or other editors.

This complements [ADR-0010](0010-mirrored-half-is-a-core-maintained-reflection.md)'s reused versus distinct design identities and [ADR-0024](0024-share-reversible-pcb-routing-not-half-firmware-configuration.md)'s shared routing boundary. The [topology research](../research/2026-10-19-hardware-topology-adr-gaps.md#2-non-destructive-pcb-retention-across-destructive-physical-topology-changes) records the earlier retention behavior, not the newly chosen deletion policy.
