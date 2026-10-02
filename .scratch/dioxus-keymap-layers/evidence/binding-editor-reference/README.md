# React Keymap binding editor reference

This is a public-UI reference packet for the binding-editor author. It is not candidate verification. The reference was `http://127.0.0.1:5173/`, source pin `5a472a9426e6e38993361da402cd4ec730feb369`, in isolated agent-browser session `keybinding-reference-6468` with profile `/var/tmp/frontend-run/profiles/keybinding-reference-6468`. The existing React process was left running. Actions below were performed through visible controls; project-record reads used a readonly IndexedDB transaction.

## Real fixture and archive

The source was the already UI-produced layered Sofle archive `imported-layered-sofle.boardstudio` (SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`). In the public Keymap → Macros UI, I added `Reference Macro`, kept its default 30 ms tap and 0 ms wait, added a second step, changed step 2 to `press`, and set its keycode to `B` (step 1 remains `tap A`). In the Function layer I assigned that macro to the actual selected `left-keys-SW2` (`matrix/left-keys/r0c1`). `left-keys-SW1` (`matrix/left-keys/r0c0`) was set through the public Keymap editor to `LC(LS(A))`.

I exported with the public Export → Save `.boardstudio project` control. `reference-layered-bindings.boardstudio` is that download (SHA-256 `c9aa6a4e387fc206fd3a4a0e944b140f095ba8774a8ab5bdc93a99d74e19c9ea`). `exported-project.json` is its embedded project record, revision 29. It preserves the Main and Function layers, the Function SW1 nested keycode expression, SW2 macro binding, and the two-step macro. The separate `saved-project-after-macro.json` is a readonly browser database capture from revision 26 before the invalid/custom keycode recovery probe; use the exported project as the final fixture.

## Public behavior observations

On Function / `left-keys-SW1`, the actual imported binding was Key press `Q`. The control exposes Key press, Mod tap, Layer tap, Momentary layer, Toggle layer, Go to layer, Sticky layer, Sticky key, Macro, Transparent, and Unassigned. Macro is disabled when the project has no macros. Defaults observed through real selections: Key press `A`; Mod tap `LSHIFT` + tap `A`; Layer tap `SPACE` + `Function`; each layer behavior targets `Function`; Sticky key `A`. With a macro present, Macro exposes a selector containing `Reference Macro`. The exact captured conditional control panels are in `behavior-defaults.txt`; `keymap-behaviors.png` and `macro-selected.png` show actual public UI states.

For invalid-expression recovery, I selected SW1, entered `A)` into its keycode input, and blurred by selecting SW3. The UI displayed `Error: Invalid keycode expression A)`; readonly IndexedDB showed SW1 remained `A` and revision stayed 26 (`invalid-after-blur.json`). I then entered `LC(LS(A))` through the same visible input and blurred; it saved at revision 27 (`valid-after-blur.json`). Public toolbar Undo restored `A` at revision 28 (`undo-binding.json`) and Redo restored `LC(LS(A))` at revision 29 (`redo-binding.json`). The alert was still represented in the page snapshot after those successful edits; this packet does not assert its clearing behavior. `invalid-recovery.txt` is the actual alert/selection snapshot. `valid-binding.png` and `redo-binding.png` are the captured screens.

## Boundaries

This packet only establishes React-side reference behavior and supplies an actual UI-created archive. It does not claim candidate parity, full behavior coverage for encoders, or any assistive-technology result. The archived data has one named macro with two steps and one nested keycode expression; it does not exercise all valid ZMK expressions.
