# 18: Ghost of the previous spacing

Status: resolved
Type: build
Blocked by: [10](10-adaptive-spacing-on-commit.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

When a committed edit changes the spacing, Layout draws a dashed ghost of the keys at the
previous spacing, with a short "+N mm" or "−N mm" note on the axis, until the next edit or
selection change. The view refits to the content in both directions after commits, undo and
redo, and stays still while dragging.

## Acceptance

- [ ] The ghost appears after both widening and narrowing, and clears on the next action.
- [ ] The view grows and shrinks with the content; it does not move during a drag.
- [x] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Answer

Implemented on `docs/ergogen-layout-comparison` (commit `feat(split-board-editing): ghost of the previous spacing (ticket 18)`).

How it works: the ghost is view-only state in the Editor (`web/src/presentation.rs`):
two signals plus a `use_effect` feed a pure tracker in the new
`web/crates/ui-model/src/spacing_ghost.rs` (`spacing_change`,
`spacing_note`, `advance_spacing_ghost`). The tracker compares accepted
documents for the spacing delta (`spacing.or(split_gap)`, epsilon 1e-6 mm)
and renders the pre-commit accepted *scene*. Undo/redo transactions retire
the ghost without raising a new one; a byte-identical doc under a new
transaction (no-op commit) resolves to Keep; selection change retires it.
The canvas (`web/src/presentation/layout_canvas.rs`) draws ghost rects with
class `m1-spacing-ghost` (grey dashed, prototype variant B) and an SVG
`<text class="m1-spacing-ghost-note">` at `(axis_x, top_key_y + 6.0)` with a
`scale(1,-1)` upright counter-flip; the render-side filter additionally
requires matching revision, matching selection, and no drag preview.
`accepted_scene` always wins as the ghost source when present, so drag
previews pre-empt the spacing ghost and its note. Newly-adopted gapless
pairs ghost widening above their stored floor; pairs created by the commit
itself (ticket 11's domain) never ghost. No production camera code changed:
the view already refits from accepted-scene bounds on every paint and not
during drags, so ticket 18 proves the refit with exact viewBox-vs-bounds
equality plus a pure bounds grow/shrink test.

Acceptance evidence:
- Native: 15 `spacing_ghost` decision/tracker tests pass
  (`cargo test -p boardstudio-web-ui-model spacing_ghost`).
- Browser render tests (`spacing_ghost_render_tests` in
  `layout_canvas.rs`): widening, narrowing, settled, preview-preemption
  pass; pre-existing `ghost_tests::transform_preview_ghosts_the_pre_drag_pose`
  still passes (precedence rule: accepted scene wins when present).
- Mounted Editor test (`layout_spacing_ghost_tests`): widen shows ghost +
  exact view-track, drag preview keeps viewBox still and hides the note,
  clear restores ghost, tree-select retires it with no revision change,
  narrow shows the negative note + refit, undo retires + refits, redo stays
  retired. Passes in 1.74 s.
- Bounds test (`keycaps_fit`): layout bounds grow/shrink exactly ±5 mm per
  side (±10 mm total) with spacing. Passes in-browser.
- Gates at commit: `check.py repo tooling` pass, `check.py lint` pass,
  `check.py build` pass, `check.py test` pass (2280 baselined executed,
  65 new incl. these 15; baseline file untouched, no
  `--update-native-baseline`). Full page wasm suite 27/27 modules pass.
- `check.py` top-level (incl. the drag-smoke browser gate) cannot run in
  this environment: it exits immediately requiring `playwright-core`
  (`set NODE_PATH`) — pre-existing env limitation, unrelated to this
  change. The scoped wasm suites above are the substitute evidence.
- Scope respected: no creation/parts/dim/breadcrumb/hover/badge touched;
  sibling hunks for tickets 13/17 in the two shared files were selectively
  left unstaged and remain in the worktree.
