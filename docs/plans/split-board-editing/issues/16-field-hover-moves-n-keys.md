# 16: Restore per-field amber "Moves N keys" preview

Status: resolved
Type: build
Blocked by: none (decision in [06](06-decide-relationship-highlighting-home.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

Hovering or focusing an Inspector number field marks, in amber, the keys that a change to that
field would move, and the canvas hint reads "Moves N keys (n per half)". The preview clears on
leave, blur or selection change and never writes the document. This is the lost variant-D
behavior (ticket 02); drag previews from ticket 24 already ship.

## Acceptance

- [x] Column, row, key and matrix fields each preview the correct key set, including mirrored keys.
- [x] A mounted browser test covers hover and focus, then clearing.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Comment

Implemented and verified per-clause except the full `check.py` gate, which
cannot pass in this checkout right now for reasons outside this ticket (see
below). No document writes; prototype branches and the untracked prototype
dir are untouched.

What shipped:

- Pure projection `field_preview` in `matrix_relations.rs`: column Spread /
  Stagger and Following splay carry to later columns, Column splay and all
  offsets stay local; row Shift / Bend / Padding carry downward, row offsets
  stay; key Shift / Bend carry upward, key offset / rotation stay on the key;
  matrix Origin / Rotation take the whole matrix. Non-number fields, splay
  origin moves (which hold keys still) and foreign contexts preview nothing.
  Eleven native tests pin the sets.
- Mirror rule follows Core's `sync`: layout records (matrix frame, column
  and row layout) and cell Shift / Bend copy to the partner, while a key's
  own offset / rotation stays put once either side's part is Local, and a
  Local / Omitted matrix never authors its partner. So key fields include
  the partner side by default (as the prototype did), except Local keys.
  Anchored dependants stay out: their Relations "Followed by" rows already
  name them. Key-local fields on an unmirrored matrix read "Moves N keys"
  with no per-half split.
- Wiring: hover / focus shows the set through a `FieldPreview` signal,
  leave / blur / unmount releases it, and selection change or any accepted
  revision clears it. Canvas keys and part outlines take solid amber
  `is-field-preview`, distinct from the violet dashed reference and ordered
  so the whole-scene drag amber still wins; the hint reads
  "Moves N keys (n per half)".
- Mounted tests (`layout_field_preview_tests.rs`): hover previews 12 keys
  on both halves with the exact hint, revision unchanged, leave clears;
  focus previews and blur clears; selection change clears.

Verification: native layout suite 123 pass; `check.py test` passes (2280
baselined, no renames so no baseline update); `check.py build` passes;
`run-wasm-tests` for the new module 3/3 and the relations module 14/14
pass; `rustfmt --check` clean on every touched file. Full `check.py`
cannot go green here: `lint` fails on a sibling's unformatted core file,
the browser step needs `playwright-core` (absent; drag smoke), and the
bin/test targets do not compile while sibling WIP adds required
`LayoutMirrorLink` fields (`split_gap`, `spacing`) without updating all
struct literals. The new test fixture builds its link records from JSON so
further additive link fields cannot break it. If ticket 15 retires the
`RelationHighlight` provider, revisit the canvas tie-break (the preview
reads it) at merge time.
