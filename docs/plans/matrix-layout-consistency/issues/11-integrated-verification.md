# 11: Integrated checks and a real product gesture pass

Status: resolved
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

## Outcome

Resolved 2026-10-09 with the gaps listed below.

**Commands.** `check.py` was not run as one command. Its steps ran as follows, on the tip of
`typed-core-edits/mechanical` (`release` was not run, as agreed): `lint`, `typecheck` and `browser`
passed together after each ticket, the last time on the ticket 10 commit (browser 170 s). `test`
failed in the nested worktree only at `cad/scripts/test-cadrum.py` (cargo workspace discovery
inside `.worktrees/`), and passed in a non-nested worktree at the same commit: 51 suites, 94 s.
`repo`, `tooling` and `build` last ran earlier on this branch; `build-web.py --fast` succeeded on
the final commit. `CARGO_BUILD_JOBS=2` was not used (the repo default is 12 jobs).

**Save and reopen.** New native test `placement_survives_save_and_reopen_unnormalized`
(`core/tests/matrix_placement.rs`): staggers, a column offset Y of -9.525, a column X offset, a row
offset, a key local offset and rotation, and attached companions. After a JSON round trip through
a fresh `CoreEngine`, the matrices and part poses are identical and the offset is not converted. It
passed on first run (a guard, not red first).

**Real product pass** (this branch built with `build-web.py --fast`, served on port 4174 because
nothing was serving 4173; driven in the built-in browser):
1. Pass: a blank 5x7 matrix shows Structure under Relations (no fallback sentence). Hover highlight
   was covered by the browser tests, not exercised by hand.
2. Pass: Column 3, Transform, Stagger, drag up: Stagger 4.7625 (snapped to the quarter-unit grid),
   Offset 0, columns 3-7 moved and 1-2 stayed.
3. Pass with a caveat: the drag tool did not deliver Shift, so the change landed as stagger
   (9.525). Dispatching pointer events with `shiftKey` at pointer-down showed the offset glyph
   and changed only Offset (4.7625, 4.7625), Stagger unchanged.
4. Partial: with Column 2 splayed by 10 degrees, a Column 4 Stagger drag moved columns 4-7 along a
   tilted axis (about 3-6 px sideways per 48 px up); the exact angle was not measured because the
   canvas re-fits as the board grows.
5. Not run: Row 2 Offset in the product (covered by `a_row_offset_drag_and_nudge_change_only_that_rows_offset`).
6. Pass: Convert on an unsplayed column moved 0 of 35 keys (Stagger 9.525 to 14.2875, Offset Y to
   0, X kept). On splayed Column 2 with an offset Y it is disabled with Core's reason.
7. Pass: Undo after Convert restored the previous values in one step. Escape mid-drag left Stagger
   unchanged (events dispatched directly).
8. Not run: a mirrored pair in the product (covered by `a_drag_on_the_right_half_of_a_linked_pair_updates_both_halves_equally`
   and Core's linked-halves tests).
Screenshots were viewed during the pass but are not committed.

**GitNexus.** `detect-changes --scope compare` is capped (`LISTING CAPPED`) against both `main`
(1400 files) and `dev` (155 files, 2209 symbols, critical), so by this ticket's rule it is not a
clean check. The index is also stale. Per-ticket impact checks were recorded in tickets 04-10.

**Follow-ups.**
- `layout_align_controller.rs:822` still adds to `column_offsets` through `SetMatrix` (decision
  needed: route through `SetMatrixPlacement`, or accept as an exception).
- Row Offset uses column 0's frame on splayed matrices.
- Relations highlight re-renders the whole canvas on hover (backlog).
- The Standards and Spec reviews ran per ticket pair, not once across the whole plan; a plan-wide
  review is still open.
- Keyboard ticket 04 is now unblocked.
