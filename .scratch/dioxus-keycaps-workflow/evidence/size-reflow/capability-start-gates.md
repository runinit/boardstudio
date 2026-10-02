# F6C.3 capability start gates — 2026-10-02

**Disposition:** the named capabilities are present on the selected implementation fixture and source baseline `c701233ea7bab6a4c794cba38cabf13be3a0b498`; F6C.3 source work may start. This is a bounded start receipt, not a feature acceptance or closure of F6C.1/F3.2/F3.5.

## Fixture

- Archive: `/tmp/keycaps-fit-fixture.boardstudio`, SHA-256 `f2c38c70edbb02994c98b8fbd1eadfe99143b5d637916414dbeb4546162324bb`.
- Project: `39fea3a6-de47-40b6-a052-37ce2cab7c34`, “Sofle v2”, revision 6.
- Active board: `left`, “Left PCB”, 70 board parts. `left-keys` has 24 enabled key cells at pitch 19.05×19.05mm and `left-thumbs` has 5 enabled cells; the accepted document also contains paired right-side matrices. The fixture does not include a mirrored `Layout.mirrorLink`, so source-level linked-pair capability exists, but paired-mirror behavior still needs a purpose-built acceptance fixture.
- Current geometry fallback in the pinned React Layout path is `part.keycap`, then definition keycap, then `max(1, pitch - edgeGap)` per axis (default edge gap 1mm); it is sourced from `app/src/ui/Workbench.tsx`, not inferred from per-key Keycaps settings.

## Capability receipts

1. **Accepted key/matrix projection.** `AcceptedSnapshot` supplies one accepted `ProjectDoc` plus `SceneDelta`; the latter carries stable `MatrixScene` IDs, enabled cell membership, resolved cell pose, and per-column projected axes. The existing `keycaps_scene::project` rejects a mismatched session epoch, document ID, or active board and produces supported active-board keys with stable part IDs/effective size. `tree::resolve_selection` resolves Matrix, Row, Column, and Key contexts through the same accepted board, enabled matrix cells, and live document membership. The resize implementation must use those accepted inputs and the Layout-specific React size fallback above; it must not create a second writable projection.
2. **Mounted Layout selection and linked metadata.** The shared presentation calls `objects::use_matrix_inspector` unconditionally with the current selected `ScopedTreeContext`, workspace signal, and selection generation. That context carries the active `Scope` and Matrix/Row/Column/Key selection; `resolve_selection` produces ordered stable member IDs. Accepted `ProjectDoc.layouts` provides board-owned layout and mirror-link source/partner metadata. The current projection only mounts matrix fields for a whole Matrix context; exposing the reviewed Key size section for key/row/column contexts is part of this slice. Source: `web/src/presentation.rs`, `web/src/presentation/objects/matrix_inspector_controller.rs`, and `web/src/presentation/objects/tree.rs`.
3. **Existing accepted-edit/history path.** Layout Inspector fields already create `EditCommand` with accepted base revision, transaction ID, target IDs and `EditPhase::Commit`, then submit `Event::Edit` through `Runtime`. Existing presentation code uses `EditOperation::ReplaceDocument` for whole-document accepted changes. F6C.3 can submit its one planned document plus the complete target-ID set through this existing command path; no public API, schema or second document owner is needed. Source: `web/src/presentation/objects/matrix_inspector_controller.rs`, `web/src/presentation.rs`, and `core/src/model.rs`.

## Exact inspected source hashes

| Source | SHA-256 |
| --- | --- |
| `web/src/presentation/objects/matrix_inspector_controller.rs` | `d3a5b3da853f80ec5f680dd8603babd5e7c47798cbe601ed88df690949f16216` |
| `web/src/presentation/objects/matrix_inspector.rs` | `df5bbd01ee8a125dc3d81a8598f779b496238394b48b1a1653e6d4b3d8b65cea` |
| `web/src/presentation/objects/tree.rs` | `4addc489dee426411718b15abaaff006ac9b5432403102291090b630c8df3594` |
| `web/src/presentation/keycaps_scene.rs` | `bd0a03414453958e48a7525320dce652434a497f8d670db1fac133404bef52f1` |
| `web/src/presentation.rs` | `08b12411edb1d95713b708123a5acc40bd27b573191d206a864e12b436d87d07` |
| `core/src/model.rs` | `e2ea75daa133195a25a877ef33e03e9a971f49ead3d70ef10a8f1d51dcb2efea` |
| `application/src/session.rs` | `403c5eb0a375bb3d2e9456e6605d6d04573d5a9d1908f1eef523cc4f03f93fb3` |

The accepted fixture and source establish the bounded start. A paired public user journey, mixed-size source characterization, a mirrored fixture, current-source review, mounted timing/history tests, and save/reload remain implementation and acceptance work.
