# Layout command pill icon parity evidence

Candidate: `40dc6d99b60d5062bb21e7793d244df576a5922e` (isolated worktree based on `9d34f3672f4f05cb3771bc5cd358508b13e9c31f`). The change is limited to the Layout command Select/Transform/Align/Snap trigger decorations. React shows a scope/tool icon and a disclosure chevron on each trigger; the full-build Dioxus baseline showed only the labels. This candidate adds the same icon geometry, keeps the accessible trigger text and existing menu behavior, and scales the decorations at the existing compact breakpoint.

The paired before captures used the same durable archive `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`, in a separate named browser session. Both captures are 1280×577. The Dioxus page was served from the verified full baseline at `http://localhost:34732/`; the React reference was served at `http://localhost:5173/`.

- React reference: [react-reference.png](react-reference.png), SHA-256 `88c6b509e53d7a975f216b5a0df3ca12a0c711b3a8b3849a39ae02e671ff5c7b`.
- Dioxus before: [dioxus-before.png](dioxus-before.png), SHA-256 `32f4c1a5e6ab2af184ecda0ee2da7c7b5548529210b635699802bb1712ff43f2`.
- The exact full-build lineage and matching source/asset checks are in [baseline-and-capture-receipt.json](baseline-and-capture-receipt.json). It records the integration checkout, build directory, provenance path and SHA, source commit, all 22 command receipts and hashes, tool versions, and both route asset-manifest path/hash totals.

The baseline was independently accepted with `scripts/build-m1.py`'s `checked_baseline('frontend-workbench-parity-joined-20261002')` in the integration checkout. Its 1328-path source manifest exactly matched `sources()`. The root and `/boardstudio/` route manifests each contain 145 exact asset paths; `checked_baseline` verified every provider/site asset hash. The candidate has not been built in this packet; real provider reuse, route construction, and post-build browser qualification remain with the integration owner.

Validation before source freeze:

- `rustfmt --edition 2024 --check web/src/presentation/objects/layout_toolbar.rs web/src/presentation/objects/layout_transform_toolbar.rs` — pass.
- `git diff --check` — pass.
- `python3 scripts/test-build-m1-sources.py` — pass (1 test).
- `python3 scripts/test-build-m1-reuse.py` — pass (17 tests, including the full-build stub and eight-command fixture).
- Both Python test commands under `python3 -O` — pass.
- `impeccable detect --json` ran once for the three changed targets; its output included existing stylesheet advisories. No detector-suppression changes were made.

The source delta is exactly the two allowlisted Rust leaves and the allowlisted stylesheet. Both changed Rust leaves retain the baseline module-registration signature. No full worker or backend inputs changed.
