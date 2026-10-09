# 10: Highlight Relations references on the Layout canvas

Status: claimed
Type: build
Blocked by: 09
Spec: [spec.md](../spec.md#relations) · Map: [map.md](../map.md)

## What to build

Hovering or focusing a Relations reference (structure item, departure or constraint
end) highlights its parts on the Layout canvas. A row, column or matrix reference
highlights all of its members.

- State: a transient `relation_highlight: Option<Vec<String>>` (part IDs) in
  `ui-model` view state, alongside the existing finding-focus state
  (`layout_viewer.rs::finding_focus_target_ids` is the precedent). It is never written
  to the document, history or saves, and it does not change the selection.
- Render: a distinct highlight treatment from selection. Use DESIGN.md's existing
  geometry and selection tokens; the `impeccable` pass may refine it. Do not reuse
  the selection style, so the user does not read the highlight as a selection change.
- Clearing: on pointerleave or blur, on selection change, on workspace or board
  change, and when an accepted revision no longer contains a highlighted ID. The
  canvas then filters its IDs against the accepted document.
- Accessibility: keyboard focus gives the same highlight, and the panel button
  carries `aria-describedby` text naming what is highlighted.

`ui-model` sits low in the crate graph, so editing it recompiles every presentation
crate (docs/architecture.md). Keep the type addition minimal.

## Tests (browser)

1. Hovering Column 3's "inherited by Columns 4–7" item marks exactly those members as
   highlighted on the canvas (assert via DOM class/data attribute). Leaving clears
   it.
2. Keyboard focus gives the same highlight. Blur clears it.
3. Changing the selection clears the highlight.
4. When the highlighted key is removed by a landed edit, nothing is highlighted and
   no error is raised.
5. No document revision or Undo entry results from highlighting.

## Acceptance

`lint`, `typecheck` and `browser` pass. DESIGN.md records the highlight treatment;
stage only that hunk.

## Files

`web/crates/ui-model` (state type), the Layout canvas renderer that draws finding
focus, and the Relations buttons from 09.
