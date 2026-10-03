# Keymap binding Tab focus paired journey

Reference: pinned React source `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/`. Pre-repair candidate: `http://127.0.0.1:34757/`, source `d7742d46ad0e4dc05d5eb229c7c7de50e6f7945c` (root-verified guarded page build inheriting unchanged providers from full `243aa551`). The focused browser profiles were `keymap-journey-dioxus-20261003` and `keymap-journey-react-20261003`; both opened Sofle v2, Keymap / Base, Left PCB, and selected `left-keys-SW1`.

## Pre-repair paired result (RED)

In each editor, choose `Mod tap`, focus the `left-keys-SW1 tap` input, replace accepted value `A` with `Y`, and press Tab. React commits `Y` on blur and leaves focus on the `left-keys-SW1 hold modifier` select. The Dioxus candidate also commits the edit, but the accepted-value update remounts its keycode field before the browser's Tab navigation finishes; `document.activeElement` becomes `BODY`.

Retained active-element observations from the paired action:

- Dioxus: `{"tag":"BODY","aria":null}`
- React: `{"tag":"SELECT","aria":"left-keys-SW1 hold modifier"}`

The control journey is restricted to this focus transition. Other Keymap binding, macro, and encoder behavior was not requalified here. The post-repair candidate replay is pending; do not treat the source change alone as paired GREEN.
