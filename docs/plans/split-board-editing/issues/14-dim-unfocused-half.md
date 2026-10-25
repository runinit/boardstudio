# 14: Dim the unfocused half; click to focus

Status: ready-for-agent
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

- [ ] A mounted browser test covers: click the dimmed half → focus and selection; tree select →
  focus unchanged; switch to PCB → no dim; switch back → focus kept.
- [ ] Focus creates no document revision or Undo entry.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.
