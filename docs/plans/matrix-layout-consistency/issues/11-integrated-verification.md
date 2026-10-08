# 11: Integrated checks and a real product gesture pass

Status: ready-for-agent
Type: task
Blocked by: 06, 09, 10
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to do

1. Run the full gate sequentially, with one Chrome owner:

   ```bash
   CARGO_BUILD_JOBS=2 python3 scripts/check.py
   ```

   If the rustup proxy fails, put `/home/chris/.rustup-toolchain-shim` first on
   `PATH`. Keep the repo's browser timeout, and record any
   `WASM_BINDGEN_TEST_TIMEOUT` override.
2. Save/reopen: a native round-trip test, unless 02–09 already added one. Use a
   project with staggers, column and row offsets (including an old-toolbar
   `column_offsets[c].y = −9.525`), key local offsets and attached companions. After
   save and reopen through `CoreEngine`, the `Matrix` values and part poses are
   identical and nothing is normalized.
3. Check in the real product, in the user's preview
   (`http://127.0.0.1:4173/boardstudio/`):
   1. Create a blank 5×7 matrix. Relations is useful for the matrix, a column and a
      key. Hover highlights.
   2. Select Column 3, choose Transform → Stagger, and drag up about 5 mm. Stagger
      reads about 5 and Offset 0. Columns 3–7 move; columns 1–2 do not.
   3. Shift-drag the same handle. Only Offset changes.
   4. Splay column 2 by 10°. A Stagger drag on column 4 tracks its handle's axis.
   5. Transform → Offset on Row 2 moves that row only.
   6. Convert a column offset on an unsplayed column: nothing moves. On a splayed
      column, Convert is disabled with its reason.
   7. Undo each step. Escape mid-drag restores the start.
   8. On a mirrored pair, a right-half Stagger drag updates both halves.
4. Run `node .gitnexus/run.cjs detect-changes --scope compare --base-ref main --repo .`.
   `partial` or `truncated` is not clean.

## Outcome

Record the commands, pass/fail with output for any failure, screenshots of steps
3.1–3.4, and any follow-ups.
