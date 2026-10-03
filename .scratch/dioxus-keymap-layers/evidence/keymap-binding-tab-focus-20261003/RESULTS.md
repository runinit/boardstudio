# Keymap binding Tab focus paired journey

Reference: pinned React source `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/`. Pre-repair candidate: `http://127.0.0.1:34757/`, source `d7742d46ad0e4dc05d5eb229c7c7de50e6f7945c` (root-verified guarded page build inheriting unchanged providers from full `243aa551`). The focused browser profiles were `keymap-journey-dioxus-20261003` and `keymap-journey-react-20261003`; both opened Sofle v2, Keymap / Base, Left PCB, and selected `left-keys-SW1`.

## Pre-repair paired result (RED)

In each editor, choose `Mod tap`, focus the `left-keys-SW1 tap` input, replace accepted value `A` with `Y`, and press Tab. React commits `Y` on blur and leaves focus on the `left-keys-SW1 hold modifier` select. The Dioxus candidate also commits the edit, but the accepted-value update remounts its keycode field before the browser's Tab navigation finishes; `document.activeElement` becomes `BODY`.

Retained active-element observations from the paired action:

- Dioxus: `{"tag":"BODY","aria":null}`
- React: `{"tag":"SELECT","aria":"left-keys-SW1 hold modifier"}`

## First repair replay (still RED)

The first repair kept the keycode-field component key stable across accepted-value updates, but still disabled the entire editor while the request was pending. On verified candidate `http://127.0.0.1:34758/`, source `37d81320712b16cd6901f8e8ea3c80a0833ce576`, provenance SHA-256 `540bccd3ef9eab93cdc521191acd918d3ca1b605ec0969ba6a139ec517fa391f`, the same fresh-profile journey again ended with Dioxus `{"tag":"BODY","aria":null}` and React `{"tag":"SELECT","aria":"left-keys-SW1 hold modifier"}`. This isolated the remaining cause to the pending-state disabled control. The current follow-up leaves admitted controls focusable while the existing event handler continues rejecting additional requests during a pending edit. The post-fix paired replay was pending at that checkpoint; the later 34759 replay is recorded below.

The control journey is restricted to this focus transition. Other Keymap binding, macro, and encoder behavior was not requalified here.

Root follow-up before the next candidate: `current_saved_source` also disappears during Busy/Unsaved persistence, although `current_display_source` intentionally retains the exact accepted editor. Focus availability now follows that displayed owner, active Keymap and current instance. The callback still rejects pending requests and rechecks the full Saved source before submission. This remains the same nativeTab RED repair; the changed GREEN is recorded below.

## Owner/pending focus repair replay (GREEN)

Candidate: `http://127.0.0.1:34759/boardstudio/`, build `frontend-previews-contexts-20261003`, source `04c85b88eae2894b416979f785a1f0060d2eb27d`, provenance SHA-256 `81ee87aead500473ae22f79834a2bc921618442a081719860a08188c063e49aa`. The root verified the full package (8 fresh + 22 inherited commands, 1,366 source inputs, 145 route assets, no drift/mismatches) and strict page WASM all-target Clippy passed before this browser replay.

I used the retained named sessions `keymap-tab-fix-dioxus-34758` and `keymap-tab-fix-react-34758`. Both showed the same selected key `left-keys-SW1`, `Mod tap`, tap `A`, and hold `LSHIFT` before the changed action. Dioxus had revision 29 before the action. I replaced the tap with `Y` and pressed Tab once in each app. Both kept focus on the `left-keys-SW1 hold modifier` `<select>` and showed tap `Y`; Dioxus reported `Binding saved.` and `Revision 30 · Saved` (one accepted revision increment), while the React selected-key option reflected `left-keys-SW1 · Y / LSHIFT` and its local saved state. After 700 ms neither page exposed a pending status. No second edit was initiated. This closes only the changed-value Tab focus interaction; it does not requalify the surrounding Keymap workflows or assert full parent acceptance.

The Dioxus profile reached candidate 34759 without its prior in-memory document, so I imported a temporary test copy derived from `public-binding-editor/reference-layered-bindings.boardstudio` and placed a `Mod tap`/`A` binding on `matrix/left-keys/r0c0` (the selected `left-keys-SW1`) as the initial condition. React's retained session was restored from its prior accepted `Y` to `A` with one Undo before the single paired action. The original reference fixture was not modified. This setup difference is disclosed; the paired claim is limited to the field edit/blur/Tab/focus/save behavior.
