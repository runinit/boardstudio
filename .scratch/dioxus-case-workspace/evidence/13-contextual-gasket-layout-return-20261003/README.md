# Contextual gasket layout return action

## React reference

The pinned React 5173 Case workspace shows `All gasket settings` in the selected `Gasket 1` Inspector, after the shared cut-length/pad-width hint. Activating it changes the Inspector to `Gaskets` and displays the layout controls, including `Reset gasket placement`. The control is a view-selection action (`onSelectLayer?.('gaskets')`), not a document edit. Source: `app/src/ui/MechanicalAssemblyPanel.tsx`, selected-support branch around lines 544–556.

Browser profile: `case-reset-react-reference-20261003`; it imported the accepted Issue 14 after-redo archive. That archive descends from preserved original 5b; neither the original fixture nor the Issue 14 archive was modified. `react-support.snapshot.txt` records the control and 70 × 3 mm selected support; `react-all-settings.snapshot.txt` records the resulting Gaskets pane. Screenshot hashes: `react-support.png` `bd1913e75dfad22ffa3fb1a05476f76a21b0d8f255daba3a8ec257e180834c5b`; `react-all-settings.png` `1cfe19e29c64c46ca10f598c7f949c41980d6d648c602a829f58189e1be4fa0e`.

## Dioxus source comparison

Integrated candidate `fe86fa05598dee6ef8ffb02f9f6fcfaa6810f10d` showed the selected `Gasket 1` Inspector with `Assembly settings`, `Cut length`, and `Pad width`, but no `All gasket settings` control. Its snapshot is `dioxus-support.snapshot.txt`. This is a private contextual-Inspector gap; the accepted document and generated support projection are already present.

## Scope

Add the same navigation control to the existing private selected-support Inspector. Route it through the current `on_select_layer` callback with layer ID `gaskets`, keeping the active Scope, MechanicalSettings controller, accepted document, and history unchanged. No support resizing, gasket settings, history, renderer, or shared Inspector ownership changes belong in this packet.

Source implementation is in `web/src/presentation/mechanical_settings.rs` in isolated worktree `/home/chris/.local/share/boardstudio/worktrees/case-gasket-settings-return-20261003`, based on integration HEAD `145cb6c0a5746809460b494b841bf0134c34e924`. Candidate journey is pending the joined build.
