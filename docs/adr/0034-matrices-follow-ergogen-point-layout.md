---
status: accepted
---

# Matrices follow ergogen's point-layout model, without migrating saved projects

BoardStudio's matrix layout began as an ergogen fork and drifted from it. It had no inherited
column spread, no per-row step, and nothing that carries up a column. Its custom splay origin
was absolute, and one matrix could not anchor to another's key. These gaps made common ergogen
layouts (curved columns, thumb clusters) hard or impossible to build
([comparison](../research/2026-10-22-ergogen-layout-engine-comparison.md)).

On 2026-10-22 the user decided:

- **Parity is the goal.** Every ergogen points concept must be expressible and behave
  identically, proven by golden fixtures generated from a pinned upstream ergogen. Importing
  ergogen configuration files is not part of this decision.
- **Evolve the existing matrix.** A matrix plays the role of an ergogen zone. Cell identity,
  member IDs, companions, linked halves and electrical scan stay as they are.
- **Keep two deliberate differences.**
  - Row, column and key offsets still add together at a key. Ergogen instead lets the more
    specific level replace the others.
  - The default pitch stays 19.05 mm.
- **Keep BoardStudio's extended features, whose storage may change.**
  - Linked halves and the in-place X/Y mirror.
  - "This column" splay and origin edits that hold keys still.
  - The named placement gestures and Relations.
  - Companion modes and constraints.
- **Do not migrate saved projects.** The format version is bumped, older documents are
  refused, and starters and fixtures are rebuilt. There were no saved projects worth keeping.
  This supersedes [ADR-0007](0007-matrix-gestures-edit-named-parameters.md)'s rule that saved
  offsets keep their meaning. ADR-0007's named, Core-owned placement edits still apply.

## Considered Options

- **Replace the matrix with an ergogen zone/column/row/key tree.** This gives the closest
  configuration shape, but it breaks every consumer built on matrix and cell identity.
- **Migrate saved projects, keeping exact geometry.** This is safe, but the hidden
  compensation values (pose-preserving offsets and counter-splays) would carry into the new
  model, and nothing needed preserving.
