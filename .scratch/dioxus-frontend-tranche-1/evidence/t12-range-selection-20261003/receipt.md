# T1-12 focused Session range-selection receipt

Date: 2026-10-03  
Private worktree base: `75e4007027409b5cef710c8094f00505d0669d93`  
Reference: `app/src/ui/useWorkbenchSelection.ts` SHA-256 `4a27d1d5096846f00b0a3edaee0b36b7115b11bd330ca5186486367db53a3536`, lines 37–54.

The pinned React path uses the stored clicked matrix cell as the Shift anchor, verifies the anchor belongs to the same matrix and active board, then visits the row/column rectangle and retains only live primary matrix members. The anchor stays fixed during repeated Shift selections. The prior Dioxus Session `SelectParts/Range` path instead sliced the flattened supplied ID list and updated the anchor from `incoming.first()`.

## Expected RED

Before the repair, the single `application/tests/durable_session.rs::matrix_range_selects_rectangle_of_live_primary_members_and_keeps_anchor` regression exercised the old `SelectParts/Range` path with a 3×3 matrix, a disabled center cell and an enabled diode companion. The vertical r0c1→r2c1 rectangle should contain two primary members; flattened interval selection returned seven entries, including the diode and unrelated row members:

```text
left: ["matrix/main/r0c1", "matrix/main/r0c1/diode", "matrix/main/r0c2", "matrix/main/r1c0", "matrix/main/r1c2", "matrix/main/r2c0", "matrix/main/r2c1"]
right: ["matrix/main/r0c1", "matrix/main/r2c1"]
```

## Repair and GREEN

Added `Event::SelectMatrixCell` without changing generic `SelectParts` semantics. The typed event carries the active `Scope`, matrix ID, actual clicked primary member ID, projected selection IDs and selection mode. Session validates the current document/session/board/matrix and live primary target; Range computes a rectangular set from enabled scene cells that are present in the matrix, board and document. A valid Range retains its original clicked anchor; an invalid/cross-matrix anchor falls back to Replace on the clicked scope. Existing tree/canvas adapter paths route primary matrix hits and same-matrix Matrix/Row/Column projections through this event; other selections keep using `SelectParts`.

The focused fixture deliberately makes the first ID of the initial projected matrix extent differ from the clicked r0c1 anchor. It then checks the r0c1→r2c1 rectangle, omission of the disabled center and diode, repeated Shift toward r2c0 using the original anchor, and `SelectionMode::Range` synchronization. In the mounted adapter, successful Shift changes the selected context to the target Key; invalid anchors use the current projected scope and Replace behavior.

Command (assigned shared cache):

```text
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/case-current-result-status-repair-20261003/web/target cargo test --manifest-path application/Cargo.toml --test durable_session matrix_range_selects_rectangle_of_live_primary_members_and_keeps_anchor -- --exact --nocapture
```

Result: **1 passed**, 16 filtered out. `git diff --check` also passed. The integrated source was then checked and packaged on candidate 34769; its package proof records 1,373 source inputs, 154 assets per route, no drift, COOP/COEP, and the canonical command lineage. The remaining open T1-12 criteria and review are listed below; this does not certify the whole ticket or tranche.

## Pinned React reference leg

React source served at `http://127.0.0.1:5173/`, source commit `5a472a9426e6e38993361da402cd4ec730feb369`. In an isolated `agent-browser` session, imported the original layered Sofle archive (SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`). On the Layout canvas, selected primary key `left-keys-SW2` at row 1, column 2, then Shift-selected `left-keys-SW14` at row 3, column 2. React selected exactly the same-column rectangle: SW2, SW8 and SW14; selected context became row 3, column 2. The project remained saved; selection did not change geometry/history. The receipt screenshot is [react-range-green.png](react-range-green.png).

The browser CLI cannot hold Shift across trusted mouse input, so this pinned baseline dispatched `MouseEvent` clicks with `shiftKey` to the mounted ScenePart elements. The events reached React's real `choosePart` UI handler and selection state; they were `isTrusted: false`. The later Dioxus candidate journey will use ordinary browser input if supported and will record that limitation if it is not.

React screenshot SHA-256: `b10a22d31d16f925f46d8422e49817ff3dd46f20cc28bb652b956daa0ea6c4a3`.

## Integrated Dioxus changed leg

Candidate: `http://127.0.0.1:34769/boardstudio/`, source `bbd4da1b7cc0609dd4ae6d8ec0332031b0690ea1`, package provenance SHA-256 `38e1980f3c38f31af1d66676a56d2d6cf57c7a3e4130dea6be2d1cb0eeb2f680`. Browser session: `t12-dioxus-2318ea1a1376`, isolated from React. Imported the same original archive and verified 70 board parts.

An ordinary pointer selected matrix `left-keys`, row 0, column 1 (displayed as Key 2.1). I first tried holding Shift with `agent-browser keydown Shift` and sending a real mouse pointer sequence to row 2, column 1; the captured trusted pointer event had `shiftKey: false`, so that attempt selected only the target. After reselecting the anchor, I dispatched a bubbling `PointerEvent` with `shiftKey: true` to the mounted `m1-matrix-key` element at row 2, column 1. It reached the application handler (`isTrusted: false`) and selected exactly rows 0, 1 and 2 in column 1. The selected context became `keys · Key 2.3`, and the Inspector reported `3 keys selected`. The board remained at 70 parts; no browser page errors or console output were reported. Screenshot: [dioxus-range-green.png](dioxus-range-green.png).

Dioxus screenshot SHA-256: `0acb66eb289092f0382374a01d747b70a4e1fafb07bdf7a165613018d320eaca`.

This is the one focused browser confirmation for the changed rectangle and anchor flow. Both React and Dioxus used synthetic modifier events because this CLI's held-key state was not reflected in trusted mouse/pointer events. The native Session regression independently verifies rectangle membership, live primary filtering and repeated-anchor retention; the remaining broad modifier/component/scope matrix and tranche review are still open.

## F3.1 criterion accounting

- **T1-12 now has bounded RED/GREEN evidence:** the old flattened interval failed, the focused Session regression passes, and the same-column React/Dioxus workflow selects the same three primary keys with Key context on the target. The click anchor differs from the first ID in the Matrix projection. Existing generic `SelectParts` callers remain compatible.
- **T1-12 remains open:** Ctrl/Cmd component/key modifiers, same-ID/cross-board/cross-matrix fallback, canceled-gesture selection, desktop/compact coverage, and independent review were not replayed here. The untrusted modifier-event fallback is explicitly recorded above.
- **F3.1 remains open:** this packet does not change the parent gate or the neighboring T1-10/T1-11 acceptance. Reuse T1-10's existing tree/public evidence under `evidence/tree-and-parts-4b05d451/`; do not infer the remaining F3.1 public scope/accessibility gates from this one range journey.
