# 13: Decide the format reset and recovery behaviour

Status: ready-for-human
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md), [07](07-matrix-anchors.md), [11](11-mirrored-halves-rules.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Graduated from the map's Format reset fog after the layout and exception decisions
settled. ADR-0034 already chooses a format bump, refusal of older documents and no
migration. Decide the user-visible and storage-lifecycle details without reopening that
choice:

- the new format identity and its checks across local storage and portable archives;
- the message and available recovery actions when an older project is refused;
- whether/how old stored projects are retained, exported, explicitly cleared or replaced;
- how reset/new-project actions treat current drafts, pending edits, Undo/Redo and
  retained browser data, including cancellation and storage failures;
- messages for archive import and incompatible document versions, keeping refusal
  distinct from corruption or successful conversion.

Read the current storage/import/version paths before proposing policy. Do not infer
permission for automatic data deletion from the no-migration decision. This ticket
decides the policy with the user; it does not perform a reset or remove stored projects.
