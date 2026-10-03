# Keymap binding Tab focus paired journey

Reference: pinned React source `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/`. Pre-repair candidate: `http://127.0.0.1:34757/`, source `d7742d46ad0e4dc05d5eb229c7c7de50e6f7945c` (root-verified guarded page build inheriting unchanged providers from full `243aa551`). The focused browser profiles were `keymap-journey-dioxus-20261003` and `keymap-journey-react-20261003`; both opened Sofle v2, Keymap / Base, Left PCB, and selected `left-keys-SW1`.

## Pre-repair paired result (RED)

In each editor, choose `Mod tap`, focus the `left-keys-SW1 tap` input, replace accepted value `A` with `Y`, and press Tab. React commits `Y` on blur and leaves focus on the `left-keys-SW1 hold modifier` select. The Dioxus candidate also commits the edit, but the accepted-value update remounts its keycode field before the browser's Tab navigation finishes; `document.activeElement` becomes `BODY`.

Retained active-element observations from the paired action:

- Dioxus: `{"tag":"BODY","aria":null}`
- React: `{"tag":"SELECT","aria":"left-keys-SW1 hold modifier"}`

## First repair replay (still RED)

The first repair kept the keycode-field component key stable across accepted-value updates, but still disabled the entire editor while the request was pending. On verified candidate `http://127.0.0.1:34758/`, source `37d81320712b16cd6901f8e8ea3c80a0833ce576`, provenance SHA-256 `540bccd3ef9eab93cdc521191acd918d3ca1b605ec0969ba6a139ec517fa391f`, the same fresh-profile journey again ended with Dioxus `{"tag":"BODY","aria":null}` and React `{"tag":"SELECT","aria":"left-keys-SW1 hold modifier"}`. This isolated the remaining cause to the pending-state disabled control. The current follow-up leaves admitted controls focusable while the existing event handler continues rejecting additional requests during a pending edit. Its post-fix paired replay remains pending; do not treat source inspection alone as paired GREEN.

The control journey is restricted to this focus transition. Other Keymap binding, macro, and encoder behavior was not requalified here.

Root follow-up before the next candidate: `current_saved_source` also disappears during Busy/Unsaved persistence, although `current_display_source` intentionally retains the exact accepted editor. Focus availability now follows that displayed owner, active Keymap and current instance. The callback still rejects pending requests and rechecks the full Saved source before submission. This remains the same nativeTab RED repair; repaired public GREEN is pending.
