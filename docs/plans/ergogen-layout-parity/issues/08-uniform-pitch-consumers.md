# 08: Adapt consumers that assume uniform pitch

Status: ready-for-human
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Several consumers assume key centres sit on a uniform `pitch` grid (inventory sections 5–6):

- snapping (`matrix_snap_parameters`, `part_placement::snap_placement_at`);
- mirrored-pair preview bounds (`mirrored_pair_geometry::preview_cells`);
- keycap fit (`keycaps_fit.rs`);
- draft projection for new or resized matrices;
- `layout_align_controller.rs`, which still writes `columnOffsets` through `SetMatrix`.

For each, decide whether it reads projected key poses, uses spread and padding, or keeps pitch
only as a size. Also decide how matrix setup and resize seed the new parameters (neutral values,
derived from pitch).
