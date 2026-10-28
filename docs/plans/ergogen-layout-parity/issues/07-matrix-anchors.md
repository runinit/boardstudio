# 07: Define matrix anchors and several matrices per layout

Status: resolved
Type: grilling
Blocked by: [02](02-ergogen-anchor-and-mirror-semantics.md)
Map: [map.md](../map.md) · Decision: [ADR-0036](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

A layout can hold several matrices, and a matrix can anchor its frame to a key of another
matrix in the same layout, with shift, rotation and ergogen-style `affect` (ADR-0036). Decide:

- the stored anchor shape. How it composes is settled by the
  [kernel ticket](04-layout-kernel-order-of-operations.md#answer): world = anchor frame ·
  origin · rotation · mirror · local;
- the evaluation order across matrices, and refusal of cycles;
- what happens when the anchor key is disabled, deleted, or renumbered by row or column
  compaction (`matrix/deletion.rs::remove_axis`), or when its matrix is removed. For example:
  detach and keep the current pose, or refuse;
- linked halves: the anchor maps to the partner's matching key, and Local or Omitted exceptions
  on either end apply;
- a layout's ownership rules (`layout::validate` currently gives each layout exactly one
  matrix), board membership, and setup for an extra matrix (for example "Add thumb cluster");
- selection and context when several matrices are selected, and how anchored matrices appear in
  Objects;
- how explicit constraints and part-offset constraints that reference anchored keys are solved,
  in order.

## Answer

Decided with the user on 2026-10-22 in one grilling round; every recommendation was accepted.
The terms **Layout** and **Matrix anchor** are recorded in
[GLOSSARY.md](../../../../GLOSSARY.md#matrix-layout).

Facts the decisions rest on:

- Each `Layout` owns exactly one matrix (`matrix/layout.rs::validate`), though a board may hold
  matrices outside any Layout.
- Same-board linked halves link two Layouts (`mirror_link`). Cross-board Mirrored halves
  record one Matrix correspondence per matrix.
- Matrices are projected first, then the constraint solver may move a key
  (`constraints::pose`).
- `deletion.rs::remove_axis` renumbers part IDs and remaps constraints in one edit.

### Decisions

1. **A Layout lists one or more matrices.** `Layout.matrix_id` becomes `matrixIds`. The list
   is never empty, no matrix is in two Layouts, and every listed matrix is on the Layout's
   board. A matrix may anchor only to a matrix in its own Layout, so a Layout's mirror link and a
   Mirrored half's reflection always carry the whole anchored group.
2. **Stored shape.** `Matrix.anchor?: { matrixId, row, column, follow }`.
   - The matrix's existing Origin and Rotation are the anchor's shift and rotation, measured in
     the anchor frame (ticket 04, decision 6).
   - The anchor frame is the anchor key's **final world pose**: after offsets and Key rotation,
     as drawn.
   - `follow` is a subset of {X, Y, rotation}, default all three (ergogen `affect`). A
     component that is not followed comes from world zero, as in ergogen, so an X-only anchor
     drops the shift's Y.
   - There is no `orient` and no `resist`.
3. **Evaluation order.** Matrices are evaluated in anchor-dependency order, so their order in
   the document never matters. The edit is refused, with the anchor named, when an anchor
   points at the matrix itself, another board, another Layout, or would form a cycle.
   Fixtures translate ergogen's "anchors see only earlier points" into this order.
4. **Disabled and deleted anchor keys keep anchoring.** Both still carry their pose (ticket 04,
   decision 7), as ergogen `skip` keeps anchor advancement.
5. **Renumbering, removal and detaching.**
   - Row or column compaction remaps the anchor's row and column in the same edit, so the
     anchor follows the same physical key.
   - The anchored matrix **detaches and keeps its current pose** when the anchor key's row or
     column is removed, the grid shrinks past it, or the anchor matrix is removed. The edit
     writes its world frame into Origin and Rotation, clears the anchor and reports the
     change. It is never refused and never silently retargeted.
   - The Inspector's **Detach** action does the same.
6. **Linked halves.**
   - The mirrored half's anchored matrix anchors to the partner's matching key (same row and
     column, through the Matrix correspondence or the Layout mirror link).
   - Its Origin X and Rotation are reflected, which matches ergogen's default for mirrored
     anchors.
   - Asymmetry uses a **Local** exception on that matrix instead of `resist`.
   - An Omitted anchor key on one half still anchors (decision 4).
7. **Constraints.**
   - Matrices are laid out in anchor order, then constraints are solved, as today.
   - Constraints and part-offset constraints on keys of anchored matrices solve against the
     final anchored pose.
   - These edits are refused: anchoring to a constraint-driven key, and constraining a key
     that a matrix anchors to. This matches today's "Unlink the halves before constraining a
     linked key's position."
8. **Creating and editing.**
   - A Key selection offers **Add anchored matrix**. It creates a 1×3 matrix in the same
     Layout, on the same board, anchored to that key with Origin (0, −pitch). On linked halves
     it is mirrored automatically.
   - The Matrix Inspector gets an **Anchor** section: the anchor key (with a canvas Pick, like
     Pick origin), Follow X/Y/rotation, and Detach.
   - Dragging an anchored matrix writes its Origin through the inverse of the anchor frame.
     Moving the anchor key moves the anchored matrix with it.
   - The [prototype ticket](09-inspector-and-toolbar-prototype.md) decides how this looks and
     is worded. The starters fog decides whether the split starter uses it.
9. **Selection and Objects.**
   - A selection that spans several matrices has no shared frame. It gets frame-free actions
     only (delete, visibility). Transform tools and parameter fields need exactly one matrix
     context.
   - In Objects, a Layout's matrices are listed flat in evaluation order. An anchored matrix
     shows the detail "Follows Main · row 4, column 2".

### Linked-half qualifications (2026-10-23)

[Ticket 11](11-mirrored-halves-rules.md#answer) completes these rules:

- Partial Follow on a linked partner uses the reflected source-world origin and axes,
  not independent partner-world zeroing. Dependants follow actual Local anchor keys;
  Omitted matrices as well as keys retain logical anchor poses.
- Making an anchored matrix Local detaches it and keeps its world pose. Linked Detach
  normally clears both anchors; if keeping both poses requires another Local exception,
  show it and obtain confirmation before committing. The same confirmation applies to
  actual anchor removal; cancellation leaves the whole edit unchanged.
- Refuse creation/restoration of a missing counterpart anchor without silently changing
  a Local parent or retargeting. Rejoin preserves explicit child exceptions and validates
  the resulting graph atomically.
- Follow constrains matrix placement drags; Follow Rotation off disables the matrix
  Rotate gesture. Exact operations that cannot preserve their promises are refused as
  described in ticket 11. These qualifications apply to the original creation/edit rules.
