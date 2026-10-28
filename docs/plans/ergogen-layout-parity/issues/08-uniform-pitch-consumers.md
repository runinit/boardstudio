# 08: Adapt consumers that assume uniform pitch

Status: resolved
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
only as a size. Setup and resize seed default records, where absent spread and padding mean
pitch (settled by the [kernel ticket](04-layout-kernel-order-of-operations.md#answer)).

## Answer

Decided with the user on 2026-10-22 in one grilling round; every recommendation was accepted.
The term **Matrix pitch** is recorded in [GLOSSARY.md](../../../../GLOSSARY.md#matrix-layout).

Findings:

- `keycaps_fit.rs`, `mirrored_pair_geometry::preview_cells`, the split-setup inset and the
  canvas cell size already place keys at projected scene poses. They use pitch only for size.
- Snapping uses pitch as a unit (`gesture_snap_inputs`, `matrix_snap_parameters`, origin
  `snap_delta`).
- Draft projection is `project_matrix` in Draft mode, so it already goes through the kernel.
- The real gap is `layout_align_controller.rs::make_edit`. For a Column it writes a
  client-computed column offset through `SetMatrix`. Under ticket 04 a column offset turns with
  each key's Bend, so a curved column would fan out.
- **Convert to stagger** (`convert_offset_available`) has the same root cause. Its exactness
  rule changes.

### Decisions

1. **Pitch has three roles, and nothing reads it as a key position.**
   - **Default step:** an absent Spread or Padding means pitch.
   - **Size:** key, keycap, plate opening, pair preview and canvas cell size are pitch minus
     edge gap. The consumers above need no change beyond reading kernel poses.
   - **Snap unit:** gestures and origin snapping step in pitch × snap fraction, using the
     selected matrix's pitch (19.05 mm when none is selected). The new Spread, Padding, Shift
     and Bend drags snap the same way.
2. **Draft projection, setup and resize.**
   - Draft projection goes through the new kernel.
   - New matrices are seeded with no layout records.
   - Add row and Add column append default records.
   - A pitch change moves only the rows and columns still on the default step.
   - The split-setup gap formula stays size-based and unchanged.
3. **Align on a Column is a Core "move this column" edit.** Align sends a world translation,
   and Core moves only this column, rigidly and exactly:
   - It writes this column's Spread and Stagger plus the next column's compensating step.
   - Column 0 writes the matrix Origin plus column 1's step.
   - This is the same Core function as the hold-still compensation in
     [ticket 06](06-origin-edits-hold-keys-still.md#answer), and like it, it pins an explicit
     Spread.
   - The client-side `column_offsets` edit through `SetMatrix` is removed.
4. **Other Align targets.**
   - **Matrix:** writes the Origin through the inverse of its frame, including the anchor frame
     and Follow subset ([ticket 07](07-matrix-anchors.md#answer)).
   - **Row:** stays unsupported.
   - **Key and Component:** `MoveParts` as today, with keys going through the key-drag inverse
     (ticket 04, decision 9).
   - Each Align is one Core edit and one Undo entry.
5. **Convert to stagger** stays Y only and keeps every pose. It moves offset Y into this
   column's Stagger and subtracts it from the next column's.
   - It is offered only when the column's Splay, its Column rotation and every key's carried
     Bend are all zero. Otherwise it is refused with "Converting would move this column: its
     keys are turned."
   - The old "splay pivots with the stagger" reason goes away, because the splay origin now
     always moves with the column base.
   - `convert_offset_available` stays the single source for Core and the Inspector.

### Follow qualification (2026-10-23)

[Ticket 11](11-mirrored-halves-rules.md#answer) qualifies the Align inverse: refuse an
exact Align when its required translation cannot be represented by the current Follow
subset, naming the component to enable or offering explicit Detach. Do not silently
project an exact Align onto fewer directions or change Follow. Ordinary matrix placement
drags, unlike exact Align, are constrained to the allowed directions; on a linked partner
these are the transported source-frame directions.
