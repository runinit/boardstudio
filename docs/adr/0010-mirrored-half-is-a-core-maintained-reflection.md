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
  keeps its side and takes the reflected heading. (Amended below: parameters are never
  changed.)
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

## Amendment: mechanical features and reversible parts (2026-10-10)

- **Mechanical features are linked pair features, mirrored.** A mirrored half starts with
  the source's openings and mounts as linked features mapped through the pair, in both
  modes. They are never cleared and never cloned as independent copies
  ([ADR-0008](0008-linked-halves-stay-linked.md)).
- **Two supported designs, both first class.**
  - A *reversible* design is one board that is flipped: `reuse-board` creates a flipped
    instance of the same board with its pair record and linked features.
  - A *mirrored* design has single-sided parts placed as a mirror image: `new-board`.
  - A linked mirror may also use reversible footprints.
- **Parameters are never changed by mirroring.** A mirrored copy keeps every generator
  parameter, including `reversible` and `side`, exactly as the source has it. Mirroring
  changes pose only: position and heading. This replaces the earlier "use the reversible
  variant" wording.

## Amendment: details of creation (2026-10-10)

- **Keycap legends** are copied to the mirror as they are. Key bindings are never mirrored.
- **Net names** on the mirror are for the mirror's side; they are not carried over from the
  source (a mirror does not keep names such as `PERIPHERAL_LEFT_...`).
- **`reuse-board`** validates the map against the flip and refuses a non-zero map.
- **Topology, transport and relabelling** of the primary half are set by the web adapter
  (physical setup), not by `CreateMirroredHalf`.
- **Modules and embedded circuits** on the source board are refused by the first
  implementation with a named, located error; mirroring them follows in two tickets before
  the reconcile step.

## Amendment: the heading rule in practice (2026-10-10)

The user's description of the single reflected-heading rule, which also answers the
question about handed modules:

- **Mirroring across the vertical axis maps a heading `r` to `180 - r`, over the full
  360 degrees, never modulo 180.** A part at the left or right edge that faces outward,
  such as a TRRS jack or a side reset button, turns through 180 degrees: it must face
  outward on the mirrored half. A part facing up or down needs no rotation, only the
  mirrored X coordinate. A part away from the edge, such as an RGB LED, takes the same
  rule: mirrored position, reflected heading.
- **The part is not flipped.** It stays on its authored side with unchanged parameters
  (the earlier amendment), so a mirrored board keeps nearly the same routing on both
  halves.
- **Modules take the same rule.** A handed module is placed at the reflected pose; Core adds
  a finding for a part or module whose result is uncertain: a non-symmetric footprint at a
  heading that is not a multiple of 90 degrees, or a module whose own geometry is handed.
  The finding asks a person to confirm it. Core does not refuse and does not guess silently.
- **The pair's angle function must use the full 360 degrees.** The earlier comparison of
  headings modulo 180 is not enough for this rule.

## Amendment: key switches and their companions (2026-10-10)

- **Key switches mirror on X only.** A switch is not given the 180-degree turn of the
  general rule: its body keeps its orientation and takes the mirrored tilt (a key rotated
  by `r` becomes `-r`, which is what the shipped right halves already do). Turning it
  would put its LED position on the wrong side.
- **Companions are chosen separately.** The diode and the LED that belong to a switch are
  mirrored in position and may be rotated, per variant. In particular an RGB LED variant
  may be turned so its pads match the switch's LED position. When Core is unsure which
  heading is right, it adds a finding and does not guess silently.
- **Everything else** follows the general rule above (`180 - r`, full 360 degrees).
