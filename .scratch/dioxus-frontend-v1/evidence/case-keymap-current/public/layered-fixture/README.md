# Layered Keymap paired public trace

Candidate artifact: dc81237c at `http://127.0.0.1:34663/`; React reference `http://127.0.0.1:5173/`.

Fixture creation used only React's built-in public “VIK module review · above and below” demo, then the public Export → Save .boardstudio project action. The exported archive is preserved here with SHA-256 `952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`. The same archive was publicly imported into fresh disk-backed profiles `layer-candidate` and `layer-reference`.

Both apps expose Base and Navigation layers. Selecting Navigation by the public layer button yields matching ordered values: `SW1 · ESC`, `SW2 · Navigation pulse`, then SW3–SW15 `Transparent`. Selecting real canvas key `Edit key SW2` updates both selected-key state to `SW2 · Navigation pulse`. The Navigation payload is Macro `Navigation pulse` in React's editable panel; the candidate's current Keymap view presents the resulting read model without behavior/macro edit controls. This is a UI-level scope difference; F6K.2 binding/macro editing is tracked separately.

Read-only IndexedDB snapshots show matching project identity (`vik-module-review`), title, revision6, and exact `keymap` object (2 layers Base/Navigation and one macro). I compared the unmodified `project.json` inside the exported archive with both persisted ProjectDocs: candidate matches the full archive ProjectDoc exactly; React differs only in three `parts[*].pose.at.y` floating-point values, normalized from `-9.020000000000001` to `-9.02` for the VIK connector entries. The keymap object exactly matches the source archive in both apps. Thus both sides were fed the same accepted document, with three React import float-normalization differences unrelated to Keymap. No keymap edit, project revision increment, private runtime injection, or direct storage write occurred.

Screenshots are same desktop session viewport and System theme: `candidate-navigation-sw2.png`, `reference-navigation-sw2.png`. Snapshots and raw read-only records are included.

## Corrected Space-key activation

Earlier Space attempts used invalid `press @e... Space` syntax and do not establish keyboard behavior. The corrected paired test clicked SW1, focused the real `Edit key SW2` canvas control, verified it was the active element while SW1 was selected, then sent `agent-browser press Space` (key only). In both candidate and React, selected-key state moved to `matrix/matrix/r0c1` (SW2). The DOM before/after records are `candidate-key-space-{before,after}.json` and `reference-key-space-{before,after}.json`. This exercises the focused canvas control with the browser's key command rather than passing a ref to `press`.
