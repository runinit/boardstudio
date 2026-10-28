# 04: Specify the layout kernel's order of operations

Status: resolved
Type: grilling
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Write the exact per-key formula that replaces `core/src/matrix.rs::location` and the key
heading. It must combine:

- column spread and stagger, both inherited;
- splay with an origin relative to the column base;
- row padding;
- Shift and Bend (the carried move and turn) at row and key level;
- independent row, column and key offsets, measured in the key's carried frame, which do not
  move the splay origin;
- the matrix origin, rotation and local X/Y mirror;
- the frame supplied by an anchor, treated here as an input frame (ticket 07 decides its rules).

Also decide:

- the inverse mapping used by key drags and nudges (`layout::move_keys`,
  `local_displacement`), which still writes the key's independent offset;
- what the scene publishes in `MatrixColumnBasis` and `MatrixScene` for gestures and Relations.
  `staggerAxis` exists; a spread axis, per-row and per-key carried frames and the splay origin
  are candidates;
- the parameter storage shape on `Matrix` and `MatrixCell` (vectors versus per-cell fields),
  resize and truncate rules, and validation.

Acceptance for the answer: the formula reproduces scenarios S1–S15 of the comparison harness,
with S15 matching under the additive rule. Use `codebase-design` for the Core interface.

## Answer

Decided with the user on 2026-10-22, in two grilling rounds where every recommendation was
accepted. A draft kernel reproduces S1–S14 against pinned ergogen `8286e18f`. S15 differs only
where the additive rule is meant to differ, and matches with offsets on one level. Three mixed
cases also match: a bend then an offset, a curved column with spread, padding and splay, and a
linked half carrying Shift, Bend and Key rotation. The largest error was `1.13e-7`. Evidence is
in the gitignored `.evidence/ergogen-layout-parity-04/` (`kernel.js`, `results.txt`).

### Formula

Per column, in the matrix's local frame:

```text
1. Column base:  b₀ = (0, stagger₀);  b_c = b_{c−1} + (spread_c, stagger_c)
   Splay: a turn of splay_c about (b_c + splayOrigin_c); that point is first carried through
   every earlier splay. Column c starts at b_c, carried through splays 0..c, with heading
   Σ splay₀..c.
2. Rows, bottom to top, with a running frame F:
   r > 0:  F ← F · move(0, padding_r)                                      // along F's heading
   F ← F · move(rowShift_r + keyShift_rc) · turn(rowBend_r + keyBend_rc)   // carried to later rows
   key = F · move(rowOffset_r + columnOffset_c + keyOffset_rc) · turn(keyRotation_rc)  // this key only
3. A local X or Y mirror reflects the key's local position and negates its whole local heading
   (splay, Bend and Key rotation).
4. world = anchorFrame · move(origin) · turn(rotation) · local     // anchorFrame = world origin when unanchored
```

### Decisions

1. **Spread and padding are full steps**, stored optionally; absent means `pitch.x` or
   `pitch.y`. Changing pitch moves only the columns and rows still on the default.
2. **Each step belongs to the item it leads into.** Column c's spread is the step from column
   c−1, and row r's padding is the step from row r−1. Column 0 has no spread and row 0 no
   padding. (Ergogen attaches padding to the lower key; the fixtures translate this.)
3. **Row and key Shift and Bend add up** into one step: the shift (row + key), then the bend
   (row + key).
4. **Offsets, then Key rotation, are measured in the key's carried frame** after its own Shift
   and Bend (ergogen `adjust`). Row, column and key offsets add up (S15). They never move the
   splay origin or later rows.
5. **The mirror reverses the whole local heading.** Linked halves copy every value unchanged.
   Today `layout::reflected` stores a negated Key rotation; under the new rule it no longer
   does.
6. **The anchor frame wraps the matrix frame.** Origin and rotation are measured in the anchor's
   frame. [Matrix anchors](07-matrix-anchors.md) decides the anchor's own rules.
7. **Disabled and deleted keys still carry** their stored Shift and Bend, as with ergogen
   `skip`, so removing a key never moves its neighbours. Relations shows these values.
8. **Parity tolerance.** Ergogen goldens compare within `1e-6` mm and degrees, because Maker.js
   rounds every rotated point to `1e-7` (`point.rotate`/`fromPolar`). BoardStudio-only
   regressions stay at `1e-9`.
9. **Inverse mapping.** A key drag or nudge goes through the inverse of the key's offset frame
   (carried heading after its own Bend, mirror, matrix rotation, anchor) and is added to the Key
   local offset. Only that key moves. Authored companions carry as they do today, and moving
   both linked halves at once is still refused.
10. **Scene.** `MatrixSceneCell` gains `shiftFrame` and `offsetFrame` (origin plus `axisX`/`axisY`,
    in world space, mirror-aware). `shiftFrame` is where the Shift starts, along the heading
    before the key's own Bend; padding gestures project onto its `axisY`. `offsetFrame` is the
    point after the Shift (the Bend's turning point), with the heading after the Bend; it is
    used by offsets, Key rotation and drags. `MatrixColumnBasis` gains `base` (world) and
    `spreadAxis` (absent for column 0). `staggerAxis` stays. Both axes are exact by
    differencing. `customOrigin` means "origin ≠ 0". Row-level gestures use the dragged key's
    frames instead of column 0's basis.
11. **Storage is grouped per axis**, replacing the parallel vectors:
    `Matrix.columnLayout: [{ spread?, stagger, splay, splayOrigin, offset }]` (`splayOrigin` is
    relative to the column base; 0 means the base), `Matrix.rowLayout: [{ padding?, shift, bend,
    offset }]`, and `MatrixCell` gains `shift?` and `bend?`. Fields at their defaults are
    skipped when saved. A short list means the missing records are defaults.
12. **Resize and validation.** A layout list longer than its row or column count is refused.
    Shrinking truncates; growing adds default records. Every value must be finite. Spread and
    padding may be zero or negative, and overlap is a collision finding, not a refusal. Column 0
    spread and row 0 padding are refused. Pitch must still exceed the edge gap.
13. **Removing a row or column** (`remove_axis`) removes its record. The rows or columns after
    it close the gap; nothing compensates.
14. **One Core kernel module**, for example `matrix::kernel::evaluate(matrix, anchor_frame) ->
    MatrixLayout`. It computes each cell's pose and frames, and each column's base, splay
    origin and axes. It replaces `location`, `column_rotation`, `column_basis`,
    `splay::origin_world`, `local_displacement` and the formula in `mirrored_half/inverse.rs`.
    Parity fixtures and BoardStudio regressions test it at that interface.
