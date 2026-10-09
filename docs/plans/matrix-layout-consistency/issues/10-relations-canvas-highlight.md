# 10: Highlight Relations references on the Layout canvas

Status: resolved
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

1. Hovering Column 3's "Inherited by" Column 4 (one item per later column, per ticket 08) item marks exactly those members as
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

## Outcome

Resolved 2026-10-09. Hover or keyboard focus on a Relations reference (Structure, departure or
constraint button) sets a transient `RelationHighlight` signal in `ui-model` state; the Layout
canvas draws matching matrix keys and parts with a dashed `accent-secondary` outline that is
distinct from selection (a selected item keeps its selection style). Leave, blur, unmount,
selection, workspace and board change clear it; removed IDs draw nothing. The button carries
`aria-describedby` text naming the highlight, with a per-instance id. Highlighting never writes a
revision, history entry or the selection. DESIGN.md records the treatment.

Tests (`layout_relation_highlight_tests.rs`): four were red before implementation (missing
highlight class); the revision guard asserts a highlight is present mid-sequence. Full web bin
browser batch passed (70 tests).

Notes: each "Inherited by" item highlights its own column, which is ticket 08's design, so no
projection change. Editor reads the highlight signal, so each enter, leave, focus and blur
re-renders the Layout canvas; moving key and part drawing into components that read the signal
would narrow it (backlog). The ticket's "Columns 4–7" wording was corrected.
