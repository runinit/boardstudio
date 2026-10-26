# 02: Screen-size labels and passive canvas chrome

Status: resolved
Type: build
Blocked by: none (decision in [01](01-layout-controls-iconography-prototype.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D`; its spec sheet holds the exact values.
Review: presentation only; RED/GREEN evidence and a diff read, no combined review.

## What to build

The read-only Layout canvas chrome takes variant D's look. Behaviour, hit testing and
document state do not change.

- **Fixed-size labels.** Canvas technical labels draw at 11 px mono at every zoom, in screen
  space. Today they use world units: `m1-anchor-badge-label` (2.9), `m1-split-focus-label`
  (2.2), `m1-spacing-ghost-note` (2.9) and `m1-part-label` (1.8) in `web/assets/m1.css`, drawn
  by the `Scene` in `web/src/presentation/layout_canvas.rs`. Part names sit at the top of the
  part, clear of the tie dot.
- **Anchor badge.** The badge from split-board-editing
  [17](../../split-board-editing/issues/17-canvas-anchor-badge.md) becomes a 7 px violet
  (`accent-secondary`) ring at the anchor point, keeping the tie. Its label moves out of the
  canvas into a "Placed from <anchor>" readout at the canvas's bottom left, in 11 px mono violet.
  The badge projection (`web/crates/layout/src/anchor_badge.rs`) stays the source of truth.
- **Colour roles.** Violet is used only for anchoring: the ring, the tie, the readout and part
  outlines. Amber is used only for keys that move (the existing field and drag previews) and the
  spacing note. Key hover is a 1.5 px accent stroke over a 60% selection fill, and solid muted
  on the dimmed half. Undefined colours are not added; every value comes from a DESIGN.md token.

## Seam and first RED / GREEN slice

Seam: one pure helper that maps a screen-pixel size to canvas units for the current view scale
(the view box the `Scene` already computes). Every fixed-size label and ring reads that helper
instead of a hard-coded world size. Give it a native test.

RED: a render test in `layout_canvas.rs` (next to `spacing_ghost_render_tests`) mounts one scene
at two zoom levels and asserts that the anchor-badge label, the dimmed-half label and the
spacing note have the same on-screen font size (11 px, ±0.5) at both. It fails today because
those labels scale with zoom.

## Acceptance

- [ ] Native test for the px-to-canvas helper. Render tests prove 11 px labels at two zooms, the
  ring at 7 px, and the "Placed from" readout text for column, anchored-matrix and part
  selections. It clears when the selection clears.
- [ ] Existing mounted suites stay green unchanged: `layout_anchor_badge_tests`,
  `layout_split_focus_tests`, `layout_spacing_ghost_tests` and `layout_field_preview_tests`.
  Update a selector only where this ticket renames a class, and say so.
- [ ] Live-app visual pass at three zooms against the prototype's `?variant=D` render, light and
  dark themes. Record it in the Answer.
- [ ] DESIGN.md's Layout canvas paragraph records the fixed-size label rule and the violet/amber
  role split.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` only for deliberate test renames.

## Answer

Implemented on branch `docs/ergogen-layout-comparison` (prototype ref
`prototype/layout-controls-iconography` `e91865f43`, variant D).

What changed (ticket-02 files only):

- `web/crates/layout/src/screen_scale.rs` (new): pure `px_to_world`
  helper mapping screen pixels to canvas units for the current view scale,
  with 3 native tests (11 px mapping, zoom halving, degenerate inputs).
- `web/src/presentation/layout_canvas.rs`: `Scene` takes the view scale
  (`view_width_world`, `surface_width_px`) plus the caller-computed badge.
  The anchor badge is now a 7 px violet ring (tie kept); its label moved to
  the readout. Dimmed-half label, spacing note and part names draw at
  11 px via the helper; part names sit at the top of the part; dim-label and
  note offsets are screen-fixed above the top key edge (rotation-agnostic,
  matching the old unrotated behavior). Hover is tracked per part in the
  scene and only set while no amber preview claims the part.
- `web/src/presentation/layout_matrix_relations.rs`: `RelationMatrixKey`
  and `RelationPartOutline` take `hovered` and append `is-hover`;
  `field_previews` is `pub(super)` so the scene can keep amber winning ties.
- `web/src/presentation.rs`: computes the badge once (shared by ring and
  readout), renders the "Placed from" readout, and fixes the part-placement
  preview label to the same top/11 px rule.
- `web/assets/m1.css`: ring/readout/hover rules; removed the badge
  label-box rules. Key hover is 1.5 px accent over 60% selection fill, solid
  muted on the dimmed half, never overriding selection or amber previews.
- `web/src/presentation/layout_anchor_badge_tests.rs`: selector-only
  update — the probe reads the readout (`.m1-canvas-anchor-readout`),
  stripping the "Placed from " prefix.
- `DESIGN.md`: Layout canvas paragraph records the fixed-size rule and the
  violet/amber split.
- New `screen_chrome_render_tests` in `layout_canvas.rs`: two-zoom asserts
  for 11 px labels and the 7 px ring plus absence of the canvas badge
  label, and an Editor-level part-selection readout test (text + clearing).

Evidence:

- Native helper tests: 3/3 green (`cargo test -p boardstudio-web-layout
  screen_scale`) before the tree broke (see below).
- Render/mounted suites, `check.py`, and the visual pass could NOT run:
  a concurrent ticket-03 edit to
  `web/crates/layout/src/objects/layout_transform_toolbar.rs` leaves the
  `boardstudio-web-layout` lib uncompilable (6 errors: E0015, E0277 x2,
  E0425 x2, E0716, all in that file, none in ticket-02 files), so the wasm
  target, `wasm-pack` suites, and any app build are blocked. No results
  faked; rerun `scripts/check.py` once that file is green.
- Two-axis review (inline): Standards — ticket-numbered comments, exact
  token reuse, exact-match scripted edits with diff reads; one judgement
  call each for the view-width/surface-width pair travelling together and
  `use_signal` inside the input literal. Spec — all four labels fixed,
  ring/readout/roles per
  spec; two disclosed deviations: (1) the part-placement preview label
  follows the same top/11 px rule for consistency; (2) hover never overrides
  selection (`:not(.is-selected)`), keeping ticket 14's solid-selection
  contract over the prototype's hover-wins order.

Map pointer: not added (`docs/plans/layout-controls-polish/map.md` is not
a ticket-02 file) — left for the orchestrator.
