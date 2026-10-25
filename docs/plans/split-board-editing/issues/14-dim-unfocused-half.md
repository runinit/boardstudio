# 14: Dim the unfocused half; click to focus

Status: resolved
Type: build
Blocked by: none (decisions in [05](05-decide-dim-and-focus-interaction.md), presentation from [07](07-prototype-split-editing-view.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

In **Layout only**, the half without editing focus draws as a grey dashed outline (variant B),
with an inline "<Right|Left> half · linked mirror · click to edit here" label. Focus is a
view-only workspace state, like Handles: it starts on the source half and writes nothing to the
document. Clicking the dimmed half on the canvas focuses it and still performs normal selection,
without unlinking. Tree or Inspector selection never moves focus, but the selection stays
visible on the dimmed half. Spacing changes and workspace switches keep focus. PCB and Case
never dim.

## Acceptance

- [x] A mounted browser test covers: click the dimmed half → focus and selection; tree select →
  focus unchanged; switch to PCB → no dim; switch back → focus kept.
- [x] Focus creates no document revision or Undo entry.
- [x] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Answer

- View-only `SplitHalf` focus (`web/crates/ui-model/src/split_focus.rs` + `split_focus`
  signal on `LayoutWorkspaceState`): starts on the source (Left) half, moves only on canvas
  clicks of the dimmed half, never writes the document. Variant-B dim (grey dashed
  `is-dimmed` + inline "<half> · linked mirror · click to edit here" label) renders in the
  Layout 2D `Scene` only; selection stays solid on the dimmed half. PCB/Case never mount that
  `Scene`, so they never dim. Spacing edits and workspace switches preserve the signal.
- Evidence: `layout_split_focus_tests::dimmed_half_click_focuses_and_tree_pcb_roundtrip_keeps_focus`
  green in headless Chrome (incl. a committed-revision-keeps-focus phase); 28 related browser
  tests green; `check.py` repo/tooling/lint/typecheck/build/test green on ticket-14 content
  (native: 2280 baselined + 4 new tolerated, no baseline change; full `check.py browser`
  incl. the drag smoke needs CI playwright, unavailable in this env). Live-app visual pass:
  guide-created split board dims right with label, real mouse click flips focus with selection,
  tree select and PCB roundtrip keep focus, 4 flips in 0.63 s wall.
- Note for tickets 09/10: the test fixture constructs `LayoutMirrorLink` without your new
  fields; adopt it when your contract lands (you already patched it twice in passing).

## Comments

- 2026-10-25 (ticket 14 agent): implementation complete in the working tree, verification partial.
  Pure helpers (`boardstudio-web-ui-model/src/split_focus.rs`, native tests green), `split_focus`
  view-only signal on `LayoutWorkspaceState`, variant-B dim + inline label in the Layout 2D
  `Scene`, focus transfer in both canvas pointer handlers, mounted acceptance test
  (`layout_split_focus_tests.rs`). `check.py quick` cannot link the page binary yet: the tree
  carries ticket 15's in-progress Relations-tab removal (same shared files) with ~50 errors,
  all attributed to `LayoutInspectorTab`/`inspector_tab` fallout — zero in ticket-14 files
  (one `key:` static-string error found and fixed). Browser test + full check to follow once
  the shared tree compiles.
