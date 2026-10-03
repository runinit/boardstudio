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

Result: **1 passed**, 16 filtered out. `git diff --check` also passed. The root owns the next combined WASM page check/package and final candidate browser qualification; this receipt does not claim those gates or the complete T1-12 / tranche acceptance.
