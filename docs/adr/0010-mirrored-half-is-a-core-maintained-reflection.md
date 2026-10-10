---
status: accepted
---

# A mirrored half is a Core-maintained reflection

[ADR-0008](0008-linked-halves-stay-linked.md) decided that the second half of a pair is
the first, reflected, in every respect. On 2026-10-09 the user accepted the following
design for how Core stores and maintains it
([decision ticket](../plans/typed-core-edits/issues/16-native-mirrored-halves.md)).

- **Hybrid representation.** One Core operation, `CreateMirroredHalf`, copies and
  reflects the source half and stores the result in full, so every consumer that reads
  stored parts, matrices, outlines and wiring keeps working. The pair record keeps one
  map, a construction source, a correspondence table (source id to mirror id) and the
  per-item exceptions. After every accepted edit, and on Open, Core re-derives the mirror
  from the source; an edit made on the mirror is inverse-mapped to the source.
- **One pair record.** The mechanical pair is extended with `constructionSource` and
  `halves`. There is no second map that could disagree with the mechanical one.
- **What is reflected:** matrices, keys and layouts; components and parts; the board's
  outlines and the modules on it; wiring that is structurally the same; mechanical
  features. Closure mounts are regenerated from the pair, never copied.
- **What is never reflected, by nature:** keymap bindings, instance role, half and name,
  board references, and copper traces and vias.
- **Exceptions** are per item: `local` or `omitted`, with `linked` as the default.
  Rejoining reflects the source again.
- **Reflected heading.** One rule replaces the three current conventions. A footprint
  keeps its side and takes the reflected heading; where a footprint has a reversible
  variant, the mirror uses it.
- **Numbering.** The mirror keeps the source's matrix numbering (column 0 is the same
  physical column on both halves, with the same pins). The recipes' reversed numbering
  goes away, and right-half part and net ids change once.
- **Two modes.** `new-board` creates a second board (the shipped splits stay two boards
  for now); `reuse-board` creates a flipped instance of the same board and copies
  nothing.
- **Compatibility.** The new fields are optional and absent means unpaired, so existing
  projects are never linked automatically. The format moves to version 5 with no data
  rewrite, so an older build refuses the document instead of silently dropping the link
  ([ADR-0004](0004-document-format-version-and-id-rename.md)). Linking existing
  independent two-board projects is out of scope here.
- **Rejected:** a derived-only half (the page reads stored parts in about 282 places), a
  one-time copy with per-operation propagation (it drifts), recipe mirror arithmetic, and
  links kept in a part property bag.
