# F3.5 selected Key does not return after Undo

## Reproduction and diagnosis

The served candidate `frontend-review-fixes-20261004` (`df2abad6`, port 34820) reproduced the failure twice through Layout: add an object, mirror the existing half, select a target Key, uncheck **Key enabled**, then Undo. After Undo the cell is enabled, but the candidate shows zero selected keys, disables Fit selection, and omits the selected placement relationship. The TypeScript reference retains the selection. The coordinator also minimized the same failure on mirrored Key1.2 without the earlier assembly changes.

`App`'s runtime subscriber reconciled selected part IDs against `eligible_live_ids`. Core's `SetMatrix` omits a disabled generated cell and removes its generated part from `document.parts` and `board.part_ids`; Session prunes that selected ID before the subscriber sees the accepted snapshot. The selected semantic Key context remains valid for the disabled cell (its resolution is an empty list), but the previous part ID was gone. Undo restores the part and cell, while the session selection remains empty.

## Repair

`SelectionRetention` in `web/src/matrix_transform_lifecycle.rs` remembers the last eligible selection only for the current semantic Key context. When the same live scoped Key context resolves to no eligible member, it suspends that selection and restores it after the Key becomes eligible again. `web/src/presentation.rs` passes only Key contexts; the full `ScopedTreeContext` carries session epoch, document, board, instance and key identity. Context/scope loss clears retention. An explicit deselection while the key remains eligible also clears remembered state.

## Native regression and result

The focused native test uses the real application `Session` and `CoreEngine`: create a one-cell matrix, select its generated Key, commit `SetMatrix(enabled=false)`, assert that Core removes the key from the accepted document/board and Session clears selection, then Undo and assert the accepted key and scene member return. It primes and exercises the same selection-retention policy between those accepted snapshots.

The regression was run against the previous filter-only reconciliation and failed after Undo: actual selection `[]`, expected `matrix/matrix-main/r0c0`. After the repair, the focused selection-retention suite passed 3/3 tests, including explicit clear and changed session/key owner controls. `git diff --check` passed.

The native Core/Session fixture is an ordinary one-cell matrix; it does not construct a linked pair. The reported public/browser repro is on a mirrored target key, and the repair is keyed by the semantic selected Key context rather than pair-specific identity. The deletion control covers context loss; a separate native RemoveParts transition was not added.

## Remaining coordinator gates

No WASM page compiler check, WASM test runner, package, browser replay, commit, or ledger update was run in this author packet. Run the existing page check and affected WASM test selection before integration:

- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web`
- `python3 scripts/run-wasm-tests.py --files web/src/presentation.rs web/src/matrix_transform_lifecycle.rs`
