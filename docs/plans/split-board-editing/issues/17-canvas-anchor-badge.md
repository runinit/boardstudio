# 17: Canvas anchor badge for the selection

Status: resolved
Type: build
Blocked by: [15](15-placement-breadcrumb-retire-relations.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

On the Layout canvas, the selection shows what it is placed from. An anchor badge (authored
icon) sits at the anchor point with a short label, its target gets the violet dashed highlight,
and a dashed tie runs to the selection:
- Column N → Column N−1 ("spread, stagger, splay carry"); Column 1 → the matrix origin.
- A row or key → the row or key below it, or the column base.
- An anchored matrix → its anchor key ("follows X, Y, rotation").
- An unanchored matrix → the half origin (default).

It draws on the selected half, including a dimmed one. Parts are added in 13.

## Acceptance

- [x] Each selection kind draws the right badge, label and target; the badge clears when the selection clears.
- [x] Mounted browser coverage for column, anchored matrix and key (plus dimmed half).
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Outcome

- Native projection 15/15 green (`anchor_badge::tests`, incl. clearing and the
  ticket-13 part cases); full `check.py` repo/tooling/lint/build/test/typecheck
  green (2280 baselined + 76 new additive, baseline untouched). Mounted badge
  coverage 5/5 green (`layout_anchor_badge_tests`: column, anchored matrix,
  key, dimmed-half key, clearing) on a matched driver pair (chromedriver 153 +
  Chrome-for-Testing 153), since the system chromium 155 core-dumps bare and
  no playwright-core module is installed. Full default `check.py` is green
  except the drag browser smoke ("Part drag: Undo restored the complete saved
  document"), which fails identically on a clean control tree (ticket-17 hunks
  stashed, page rebuilt), so it is pre-existing on HEAD in a foreign
  (part-drag undo) lane and not fixed here.

## Answer

- `anchor_badge` projection (`web/crates/layout/src/anchor_badge.rs`) plus
  canvas render (`layout_canvas.rs` `Scene`), violet target via the existing
  `is-relation-highlight` on key rects and part outlines, prototype-B badge
  glyph/label/tie with fresh `m1-*` classes. First column of an anchored frame
  names what the frame follows ("Thumb origin (← Main R1C1 · follows X, Y,
  rotation)"); an anchored matrix with no evaluated anchor pose leaves no badge.
- Standards review findings adjudicated: the `half_offset` export is this
  ticket's combined-review gate (precedent: `is_matrix_key`,
  `proposed_creation_gap`); "splay carry" is the ticket's own label words and
  the glossary Avoid scopes to column-local turns; per-key `String` class
  building rejected on render cost (chains stay explicit). Duplication
  findings fixed (single cell lookup, shared label words, one dom helper).
- Spec review findings adjudicated: the off-by-one claim is a misread (R{row}
  names the anchor, matching the prototype exactly); Component-with-cell
  contexts badge like their key (Relationships parity, pure parts still
  ticket 13); aggregate anchors badge at the mean pose with the chain's own
  words; the export and part-outline target are rendering consistency, not
  scope creep.
- Closing combined review (two parallel axes, this turn): Standards reports no
  hard violations; its four judgement-call smells are declined (half-origin
  label duplication spans ticket-13 `part_badge`, `members()` vs `own_cell()`
  is behavior-identical, the `anchor_description` triple is used twice —
  all zero-behavior churn on committed code). Spec's two findings are
  adjudicated with fresh evidence: badge numbering (`Column {column}`,
  `Row {row}`, `Key R{row}`) matches the prototype verbatim
  (`split-board-editing.prototype.html` lines 408-410 use the same
  expressions), while the app's 1-based convention (`context_summary`,
  `matrix_relations`) is a cross-ticket unification follow-up, not this
  ticket; the `keys_visible` gate mirrors the key-drawing gate at
  `layout_canvas.rs:513`, so a hidden-keys selection draws neither. No code
  changes from review. Map pointer: [map.md](../map.md).
