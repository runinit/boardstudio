# Contextual gasket layout return action

## React reference

The pinned React 5173 Case workspace shows `All gasket settings` in the selected `Gasket 1` Inspector, after the shared cut-length/pad-width hint. Activating it changes the Inspector to `Gaskets` and displays the layout controls, including `Reset gasket placement`. The control is a view-selection action (`onSelectLayer?.('gaskets')`), not a document edit. Source: `app/src/ui/MechanicalAssemblyPanel.tsx`, selected-support branch around lines 544–556.

Browser profile: `case-reset-react-reference-20261003`; it imported the accepted Issue 14 after-redo archive. That archive descends from preserved original 5b; neither the original fixture nor the Issue 14 archive was modified. `react-support.snapshot.txt` records the control and 70 × 3 mm selected support; `react-all-settings.snapshot.txt` records the resulting Gaskets pane. Screenshot hashes: `react-support.png` `bd1913e75dfad22ffa3fb1a05476f76a21b0d8f255daba3a8ec257e180834c5b`; `react-all-settings.png` `1cfe19e29c64c46ca10f598c7f949c41980d6d648c602a829f58189e1be4fa0e`.

## Dioxus source comparison

Integrated candidate `fe86fa05598dee6ef8ffb02f9f6fcfaa6810f10d` showed the selected `Gasket 1` Inspector with `Assembly settings`, `Cut length`, and `Pad width`, but no `All gasket settings` control. Its snapshot is `dioxus-support.snapshot.txt`. Candidate `34751` included the new button, but clicking it left the Inspector on `Gasket 1`; `dioxus-rejected-action-34751.snapshot.txt` captures the outcome. The button's `gaskets` target was rejected by the existing exact-current `MechanicalSettingsMount` selection guards, which only admitted generated stack body IDs and individual gasket IDs.

## Scope

Add the same navigation control to the existing private selected-support Inspector. Route it through the current `on_select_layer` callback with layer ID `gaskets`; admit that group selection through the existing current-scene guards only when the accepted exact Case scene contains gasket supports. Keep the active Scope, MechanicalSettings controller, accepted document, and history unchanged. No support resizing, gasket settings, history, renderer, or shared Inspector ownership changes belong in this packet.

Source implementation is in `web/src/presentation/mechanical_settings.rs` and `web/src/presentation/mechanical_settings_mount.rs` in isolated worktree `/home/chris/.local/share/boardstudio/worktrees/case-gasket-settings-return-20261003`, based on integration HEAD `145cb6c0a5746809460b494b841bf0134c34e924`. The React route is captured in `react-support.png` (SHA-256 `bd1913e75dfad22ffa3fb1a05476f76a21b0d8f255daba3a8ec257e180834c5b`) and `react-all-settings.png` (SHA-256 `1cfe19e29c64c46ca10f598c7f949c41980d6d648c602a829f58189e1be4fa0e`). The candidate button is captured in `dioxus-support.png` (SHA-256 `0a959bf1f5b88f482cc697a074ab3ce5bb269a919567714ecd5e6b09d021ca2c`); its 34751 outcome screenshot has SHA-256 `c881b48780b5d33f395b06397e37c95f4b0d253e3375d87d0bb281d82d034dc2`. Candidate green journey awaits the next integrated build.
