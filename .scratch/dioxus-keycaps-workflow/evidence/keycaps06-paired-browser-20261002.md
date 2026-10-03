# F6C.4 Keycaps selected-key disclosure paired journey — 2026-10-02

This is bounded browser evidence for the selected-key disclosure and existing Keycaps settings owner. It is not full F6C.4 acceptance and does not close any parent join.

## Pinned inputs and provenance

- React: `http://127.0.0.1:5173/`, pinned source commit `5a472a9426e6e38993361da402cd4ec730feb369`, isolated browser session `keycaps-settings-react-pinned` (CDP 42461).
- Dioxus: `http://127.0.0.1:34734/`, integrated candidate source `f261a327858f51a1de4928374bc65b669e2792a3`; build provenance `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-contextual-case-mirror-keycaps-20261002/provenance.json`, SHA-256 `f77c5920371ae6a69e42ae42e20f0eef2519f7822f1eb404f4b580c17b38e066`; isolated browser session `keycaps-settings-dioxus-paired` (CDP 37135).
- Both apps imported the byte-identical retained Sofle fixture `/home/chris/.local/share/boardstudio/retained-tmp/20261002/dioxus-34726-fixture-models-false.boardstudio`, SHA-256 `c6ea3c0f72f999ee6736d1e65b6b2c36105b52c5a8d511285ffdc01695aa9d7e`. The missing historical `f2c38c…` fixture remains an independent, unwaived provenance gate.
- Both final captures use viewport 1280×577. React screenshot SHA-256 `a98d9058b74fac2eb4cb74cd70b2c834d202b4a6147c049a858d743c6926057a` at `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-settings-react-paired-final.png`; Dioxus screenshot SHA-256 `9667b241bdab7c7ec4ade9ac4ddbe69cae6a447288ecf10b4ba6366a1441dbdd` at `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-settings-dioxus-paired-final.png`.

## Actions and observed result

In each app: open the imported project, choose Keycaps, focus the selected-key disclosure summary and press Space to collapse/reopen it, then select `left-keys-SW1` from the canvas. The selected disclosure title updates to `left-keys-SW1 · key` and remains collapsed until reopened. Scroll the Inspector with a trusted CDP wheel event at its actual on-screen position, open Keycap overrides, select profile DSA, set Width to 2 using keyboard select-all/type followed by Tab, and leave Depth at 1. Undo returns Width to its unset state; Redo restores 2. Reload, reselect SW1 in the canvas, and confirm DSA and width 2 persist.

The Inspector wheel was dispatched through CDP `Input.dispatchMouseEvent` with `type: mouseMoved` and `type: mouseWheel`, using coordinates inside the inspector (x=1100, y=470). This is required because the CLI wheel helper dispatched at (0,0), which had falsely appeared to indicate a product scroll defect. Independent trusted-event reproduction and correction are recorded at `/home/chris/.local/share/boardstudio/reviews/inspector-wheel-native-interaction-20261002.md`, SHA-256 `600003dce9f93e2b898ff23fe52c62f318e5be318c9898d26fc5116d6116be972`.

Both versions accepted the DSA/width edits, supported Undo/Redo and retained the saved override after reload. Dioxus exposed saved revisions 11–13 through edit/history/reload; React displayed its local-saved status without a revision number. Neither app reported a browser page error in this journey.

## Visual parity notes and open gates

At the same viewport and fixture, the keycaps geometry is broadly comparable. Dioxus still lacks the React Keycaps subheading and its `2D / 3D assembly / Footprints` view controls. Its left Objects pane also has extra Physical instance and Group objects selectors absent from the compared React Keycaps pane, and the canvas/Inspector composition differs. These are remaining Keycaps frontend parity gaps; this issue06 slice does not close them. Keyboard search/picker coverage, a full accepted settings edit matrix, pending-operation behavior and the wider F6C.4 edit/Undo/Redo/save/reopen journey remain open.

This journey does not prove navigation from fit findings, stale target behavior, camera framing, all selection/context paths, original f2 fixture replay, or full parent acceptance.
