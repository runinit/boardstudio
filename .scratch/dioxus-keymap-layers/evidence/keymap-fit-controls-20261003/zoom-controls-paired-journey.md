# Keymap zoom footer paired journey

Reference: pinned React `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/`. Candidate: `http://127.0.0.1:34757/`, integrated source `d7742d46ad0e4dc05d5eb229c7c7de50e6f7945c`, build provenance SHA-256 `f5e98f1b521a06c1b8cc91631e8fcdee550972489d8d5e3830a1a4c9e1c0f2e3`. Root reports eight fresh commands plus 98.04 seconds, 1365 source inputs, 145 assets per route, zero source/asset drift; combined strict checks passed in 17.66 seconds.

Both named browser sessions opened the same Sofle v2 project, Keymap / Base, Left PCB at 1280 × 577. After `Fit board`, both zoom labels showed 80%. The board outline bounds were Dioxus `(406.4, 184.8, 382.2 × 303.5 px)` and React `(407.5, 186.6, 380.1 × 301.8 px)`. Dioxus now normalizes its label against the pinned React physical `getBounds` camera basis while retaining its existing Keymap camera basis and `SetCamera` owner.

Clicking `Zoom in` at the browser viewport midpoint changed both labels to 96%. The board outline measured Dioxus `(359.7, 163.9, 458.6 × 364.2 px)` and React `(360.9, 166.2, 456.1 × 362.2 px)`, within roughly 2.5 px. The screen midpoint's world anchor was preserved by both callbacks: React exactly at the measured precision; Dioxus differed by about 0.03 mm vertically (less than one pixel).

For camera limits, a cancelable standard `WheelEvent` was dispatched on each live SVG canvas so the production wheel handler ran. A large positive delta reached the 25% lower limit in both apps; a large negative delta reached 400%. Clicking Zoom out at 25% and Zoom in at 400% retained the respective limits. The low-level `agent-browser mouse wheel` command did not produce a camera change in either session, so this receipt records DOM-dispatched production-listener coverage only; it does not claim a physical wheel gesture.

Dioxus remained Revision 3 / Saved throughout. Fit and zoom only submitted the existing camera event; no document edit was observed. The React and Dioxus Undo/Redo controls were present and no editing command was used.

Captures:

- Fit board: [Dioxus](dioxus-34757-fit-board.png), SHA-256 `b43fb66fabe82d327379fca7f0cfe9c6d8ded027bd8d5954d7fdad4d28222dd4`; [React](react-5173-fit-board.png), `e37459e4bad215d4de31d7771612a08e6f3324f53bb53ec84ab07bf77c1768c2`.
- Zoom in at 96%: [Dioxus](dioxus-34757-zoom-in-96.png), SHA-256 `18e086c244cbccf92c3e204e99b40ef71d5d2c6e1fb61f008c59290931419caf`; [React](react-5173-zoom-in-96.png), `ac5752ac9d44fdbf286277af28e9f73fcf36ebcf187dff606dde09e2730efaca`.
- Wheel minimum 25%: [Dioxus](dioxus-34757-wheel-min.png), SHA-256 `230fff50f4c59930365cee9a1991c316dcd81a0c114aab4fc9537a3348cec13f`; [React](react-5173-wheel-min.png), `c3cd1f95a7f7c59e1a0b3f0fd31b241a9df42372bbf0a8152383bbc40793e8a4`.

The scale conversion is a presentation mapping between the React physical camera basis and existing Dioxus Keymap basis. It does not introduce another camera owner or change Layout, PCB, Keycaps, or 3D camera behavior. Reconcile this source-accounting choice under the existing Keymap RF-009 handoff; no separate architecture finding is asserted. Full F6K.1/F3.1 and broader workspace/viewer acceptance remain open.
