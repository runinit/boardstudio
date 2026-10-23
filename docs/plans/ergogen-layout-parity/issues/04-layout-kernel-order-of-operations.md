# 04: Specify the layout kernel's order of operations

Status: ready-for-human
Type: grilling
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Write the exact per-key formula that replaces `core/src/matrix.rs::location` and the key
heading. It must combine:

- column spread and stagger, both inherited;
- splay with an origin relative to the column base;
- row padding;
- the carried shift and carried rotation at row and key level;
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
