# F6C.4 focused finding marker implementation receipt

## Frozen source

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/keycaps-navigation-mounted-owner-20261002`
- Branch: `codex/keycaps-navigation-mounted-owner-20261002`
- Source commit: `b521ce939062ae09b485f4127ed4bc1b0015bbf9`
- Reviewed planning packet: commit `1ff36a50aa4f407eff7a0431d128300794d8ee4d`
- Spec blob SHA-256: `ea3d11ae84fb97255adeffcf09d381a69832300ea7a938bc2ba961735e4bc61c`
- Issue 07 blob SHA-256: `9791e360959ce728b93ebb061aa96dc07b04248ea328024f31bb367584c4bb61`

| Source file | SHA-256 |
| --- | --- |
| `web/src/presentation.rs` | `596e00bbd016a63f686634b4ebd850db7b63a9211130e59ac1fa0e1b25b597c4` |
| `web/src/presentation/keycaps_finding_marker.rs` | `ec736c85a60e0fd04085257a37777419726abb6d8e23326141c3c1fe69c31196` |
| `web/src/presentation/keycaps_finding_marker_tests.rs` | `dd599c11441f867fec225507753cc7b2b9ff1027b048750eeb0d1377989a2af4` |
| `web/assets/m1.css` | `63d868de87b32f3d6d67886cb8daf07341fa3f9133dba890ba96c121c05cbdc7` |

## Behavior implemented

The Editor stores one focused finding only after the existing accepted-navigation admission succeeds and the destination is Layout. The projection matches that identity against `snapshot.scene.finding_markers` and requires the active Layout, exact accepted scope, snapshot token, revision, and board. It renders Core-provided contours inside the current flipped Layout SVG group using the TypeScript `wb-outline-finding is-focused` class and `data-finding-id`. A new navigation replaces focus; owner/workspace/board changes clear it. Missing or stale markers render no contour. The change creates no geometry, Core operation, document edit, or history entry.

The mounted Dioxus projection test uses two finding IDs and two board IDs, checks exact focused identity and contour count, and verifies suppression for changed board/workspace/scope/token/revision or missing marker data. This test mounts the production marker component, not the full Editor or packaged browser workspace.

## Verification

- Expected-red run before the projection implementation: 0 passed / 1 failed because no focused marker DOM node was rendered. Log: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-marker-expected-red.log`, SHA-256 `748732a85b0bc20d3f8ee5737187b45ea3833aed9e06a873c0438227e8c8b4e6`.
- Green: `wasm-pack test --headless --chrome . --no-default-features --features page --bin boardstudio-web -- presentation::keycaps_finding_marker::tests` — 1 passed. Log SHA-256 `4ffbe022ca0745795744a129f6deb890b4474f8bc19e2d4d13c1f8c65e313824`.
- `cargo clippy --manifest-path Cargo.toml --target wasm32-unknown-unknown --all-targets --features page -- -D warnings` — passed. Log SHA-256 `e65f988241531ec62884a9dd8da8a48bffc70a11b6d2c5948e72e58453ecfcde`.
- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` — passed.
- `git diff --check` — passed.

## Open joins

The source commit is ready for independent review and isolated integration. This receipt does not close the paired pinned React/Dioxus browser journey, the original missing fixture identity, actual destination camera/focus/history acceptance, Case 3D marker behavior, issue 05, F6C.4, INT.2, or other parent joins. No shared RUN/task/RF ledger was edited. No new refactoring issue was found beyond the already tracked Editor-level accepted-owner/concentration observation.
