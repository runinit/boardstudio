# CAD job and session verification

Current candidate branch: `codex/m1-session-worker-20261001`.

The board-isolation regression first failed on the unscoped snap call: `geometry_snap_ignores_parts_owned_by_another_board` asserted that the preview position stayed at 19.4 mm, but snapping used a target owned by the other board. After `normalize_drag` began passing the active board into `snap_part_in_board`, the same test passed. Existing single-board gap snapping still passes, including legacy documents with an implicit board/part index.

Validation on the candidate:

- `cargo test --manifest-path application/Cargo.toml --locked` — 11 passed.
- `cargo test --manifest-path web/Cargo.toml --locked --lib cad_jobs::tests` — 5 passed.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features cad-worker` — passed.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page` — passed.
- Strict Clippy for application, page, and CAD-worker targets with `-D warnings` — passed.
- `git diff --check` — passed.

CAD job coverage includes captured provider preparation, stale scope/revision rejection, instance reflection restricted to the selected board, reply operation/identity checks, and payload validation. The worker uses the public generated Cadrum WASM functions through dynamic import. Browser integration, M1 fixture parity runs through the built browser workers, and end-to-end cancellation in a real browser remain for the owning integration/browser gate.
