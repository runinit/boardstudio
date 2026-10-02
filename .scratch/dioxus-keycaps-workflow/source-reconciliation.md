# F6C.2 source reconciliation

This evidence packet supports the F6C.2 child tickets and their [settings specification](settings-spec.md). The separate issue01 F6C.4 fit-findings spec retains `spec.md`; these are distinct contracts. This evidence does not edit the task graph, parent criteria, parent status, or shared run/RF ledgers.

## Confirmed TypeScript contract

- Board colors section exposes board keycap color, legend color, and minimum clearance.
- Matrix profiles section exposes profile, first profile row, wall thickness, and socket. The socket can inherit from the switch profile or explicitly select MX, Choc v1, Choc v2, or Alps.
- The per-key section exposes legend inheritance/explicit blank, color inheritance, profile/socket/row overrides, and width/depth overrides.
- The command path uses existing `SetKeycapBoard`, `SetMatrixKeycaps`, and `SetKeycapKey` operations. This plan introduces no new authority or persistence model.

Primary source examined: `app/src/ui/KeycapPanel.tsx`; edit domain types are provided by existing contracts. The existing shared Keymap view supplies active board, matrices, keys and selected identity.

## Current Dioxus contract and gap

- `keycaps_scene` projects the active board, keys, matrices, physical dimensions, color and legend for the canvas.
- `keycaps_workspace` renders matrix names, searchable/selectable keys, selected-key summary/editor, and fit findings in the contextual Inspector.
- `keycaps_settings` contains the selected-key edit lifecycle and uses the existing key-specific change operations.
- No board-level keycap settings or matrix-level keycap settings are exposed by the current Dioxus Inspector. These are the independent next implementation slices.

The source inspection locates this gap in the Keycaps presentation; no new Rust API, visibility, schema, or resolver work is indicated by the evidence.

## Paired visual evidence

- Correct Dioxus Keycaps screenshot on original Sofle fixture: `evidence/issue01-implementation/keycaps-dioxus-keycaps-correct-4d.png`, SHA-256 `d18da3ab6290931608b423e035f8fbe528c1acdc934b8a71a2999ac372791f4e`.
- TypeScript default Keycaps screenshot: `evidence/issue01-implementation/react-keycaps-settings.png`, SHA-256 `d8c39cfb6ab2542f9eee7c0523b29f4b46fe77328729422b68c65e2fa5bdd46f`.
- The original archive is `/tmp/keycaps-fit-fixture.boardstudio`, SHA-256 `f2c38c70edbb02994c98b8fbd1eadfe99143b5d637916414dbeb4546162324bb`; project ID `39fea3a6-de47-40b6-a052-37ce2cab7c34`, Sofle v2.
- A TypeScript-authored derived archive with the `keys` matrix set to Cherry has SHA-256 `0c788bbae3e81a5607b1e385c9cf78a814d620ebaded0c96c0d90085582c2e0e`, same project ID, revision 9. Commit-4d bounded Dioxus parity screenshot: `evidence/issue01-implementation/keycaps-dioxus-derived-cherry-4d.png`, SHA-256 `f49f627d398d7b3bbd41b4fc65d3ca7c38afee51e2c4f622c17d658fa75564a1`.

The final source-guarded browser attempt used root's integrated production package `frontend-keycaps-integrated-20261002`, source commit `bd671ae8db8897388d26ebf973380c69efb9bffd`, provenance SHA-256 `08af632d191dd549298981f2e6ea63b7685b4bf84416fb51694a4af687235e38`. Its clean and Cherry-derived captures are `evidence/issue01-implementation/keycaps-dioxus-clean-integrated-bd671ae8.png` (SHA-256 `d18da3ab6290931608b423e035f8fbe528c1acdc934b8a71a2999ac372791f4e`) and `evidence/issue01-implementation/keycaps-dioxus-derived-cherry-integrated-bd671ae8.png` (SHA-256 `f49f627d398d7b3bbd41b4fc65d3ca7c38afee51e2c4f622c17d658fa75564a1`). The Cherry-derived state showed the same 24 findings as the pinned TypeScript reference. This verifies fixture activation and clean/populated findings presentation on final integrated source; it does not complete edit/history/reopen acceptance.

## Preserved parent and RF authority

The tickets below refine F6C.2 locally. Existing task graph start and acceptance joins remain authoritative. The wrong-project capture correction and lifecycle refactoring observations remain in the issue01 implementation report for the shared ledger owner to reconcile. This source audit creates no RF ID and does not edit shared ledgers.
