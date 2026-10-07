# Mirrored layout pair — interaction brief

Status: implemented in v2. Halves are linked by default, with independent
hardware on each side, as confirmed by the user.

Experienced keyboard designers need to create a split ergonomic keyboard as
two named layouts beneath one board, with an understandable relationship between
the halves within the BoardStudio workbench.

- Add → Layouts → Mirrored pair opens a focused setup for names, matrix
  dimensions, key assembly, and the gap between the inner key edges.
  Placement previews both halves and their mirror axis together before commit.
- The object tree exposes a layout for each half beneath the board; selecting
  either layout selects its geometry and reveals its relevant controls. The
  breadcrumb path identifies the board and active half.
- One creation action is one undo step. Cancel creates nothing. Both layout
  identities, geometry, relationships, and board membership must survive project
  save/reopen, undo/redo, and export preparation.
- Geometry edits from either half update the other: rows, columns, pitch,
  edge gap, position, rotation, stagger, splay and its origin, key offsets,
  key rotation, and enabled slots. The reflection axis is vertical and saved
  with the relationship.
- Alignment excludes the moving counterpart from fixed-reference choices;
  independent parts can still serve as references.
- Switch definitions, variants, diode choices, and attached components remain
  local after creation. Extra parts have an explicit layout destination in Add
  and can be reassigned in the inspector. An encoder on one half does not
  create one on the other; custom or imported parts follow the same rule.
- Unlink halves keeps both layouts, identities, and current positions. Deleting
  one half's matrix unlinks the survivor; independent components remain on the
  board. Deleting an individual linked key disables the corresponding slot in
  both halves. Removing a companion only affects that half.
- The Rust-owned optional `layouts` record owns a matrix and independent part
  IDs. A single mirror link is stored on the right layout, while edits can
  originate on either side. Creation is a single core transaction; TypeScript
  contracts are generated from Rust.
- Both layouts belong to one board. Board outlines, PCB, case bodies, and export
  readiness remain board-level; creating the pair does not automatically split
  a board outline or establish fabrication readiness. Independent components
  retain their own positions when the linked key geometry changes.
