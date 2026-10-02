# F3.2b mirrored-pair setup placement

This packet records a narrowly observed workspace presentation parity gap and amends the existing F3.2b issue/draft criterion. It does not duplicate the ticket, change pair behavior, or claim F3.2/F3.2b browser acceptance.

## Paired public surfaces

- React reference: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`, isolated session `mirror-pair-parity-20261002-2318ea1a1376`.
- Dioxus candidate: `http://127.0.0.1:34732/`, integrated source `9d34f3672f4f05cb3771bc5cd358508b13e9c31f`; isolated session `mirror-dioxus-20261002-2318ea1a1376`.
- Candidate full build receipt: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-workbench-parity-joined-20261002/provenance.json`, SHA-256 `4bc856174b33998c5cf99f4c05a8ba35ad7ab4ce7705bfece6510cdd4467544e`.
- Both sessions imported the same original public Sofle fixture, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

I opened Layout → Add object → Mirrored pair in both apps. React presents the 3×5, MX solder, 24 mm form as a centered bounded panel over the active Layout canvas, leaving the Objects panel intact. Dioxus renders the same fields, defaults and actions inside the Objects panel's left rail. In the candidate capture, the panel crowds the board selector and the later fields/actions fall below the Objects pane, while the canvas remains a separate central region. The forms are not in equivalent workspace locations.

Screenshots:

- `react-mirror-pair-form-overlay.png`
- `candidate-mirror-pair-form-sidebar.png`

The served comparison is sufficient to establish this visual difference. Root separately paired same-fixture create/Enter/Undo/Redo/reload in both apps and reports that functional sequence passed; pointer placement and all cancellation/focus/semantic cross-order scenarios remain open. This packet does not replay or extend that functional evidence.

## Source and correction seam

The isolated implementation uses a private canvas-overlay composition around the existing `MirroredPairForm`, keyed by `projection.owner.open_id`. It accepts only the existing typed projection and Cancel/Preview callbacks. The production mount now sits directly inside the Layout `.m1-workspace-content`; the Objects rail still owns the menu and opening callback, but no longer owns or renders the form. Escape inside the focused form is consumed by the wrapper and sent through the existing current-owner Cancel callback. The controller, arbiter, owner types and Core operations are unchanged. CSS bounds the overlay to a 380 px panel within the workspace, with a 20 px desktop inset and compact 8 px inset/padding rules.

Mounted Chrome/WASM coverage mounts the production `LayoutAddObjectEntry` inside the Objects rail alongside the production overlay wrapper in a focused test shell. It verifies that the former no longer renders the form, that the wrapper is a direct child of the workspace shell, and that edited values reach Preview and return for the same owner; Escape and Cancel forward the exact current owner, while a new `open_id` resets the keyed local form state. A companion browser test checks the CSS source rules for bounded desktop and compact geometry. Both tests pass (2/2). This does not exercise the full production Editor composition or measure computed geometry; root will verify those in the packaged public journey. The final immutable output is `/home/chris/.local/share/boardstudio/retained-tmp/20261002/mirrored-pair-overlay/browser-tests-selector-correction.log`, SHA-256 `b17cb71f7bfc81e00d6d5f1379720a26fd3bb6fc94222a11eabdbd085efd3c28`. The earlier fe2ccf22 receipt is retained at `browser-tests-fe2ccf22-preselector.log` (SHA `8ab60c44d43fd18399379d2e8d8cc19abd7bc1262bc2e9ea3a5f4af18fe2b7f4`); `browser-tests.log` also currently has those same bytes. The earlier 277/d0 run recorded SHA `609be7277bc534807e61b4e90b64553a8dfe7ec61d0c80d71161901aa38fe6a`, but its bytes were overwritten and are no longer retained at that path. The selector-correction run supersedes prior receipts for current coverage. Strict wasm32 `cargo check`, wasm32 `cargo clippy -D warnings`, `cargo fmt --check`, and `git diff --check` pass. Sol's source review is CLEAR; root has joined the production source. Packaged public behavior and geometry remain open pending the public journey.

## Validation gates

After the serial package join, verify the same fixture at root and subpath public routes: the form stays centered within canvas workspace and bounded on compact widths; Objects remains visible; controls remain reachable; first-field focus, Cancel focus return, validation, form-value retention, Preview, Enter/Escape and existing pointer placement behavior remain unchanged. Preserve source/build provenance and the remaining open F3.2b and F3.2 gates.
