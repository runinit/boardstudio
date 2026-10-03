# F6C.4 Marker07 shared navigation-owner regression

## Frozen source

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/keycaps-navigation-mounted-owner-20261002`
- Branch: `codex/keycaps-navigation-mounted-owner-20261002`
- Source commit: `d1ca3634b7504244858d166212fa732e59231099`
- Parent: `27c210cb4197dc561e77061a6e5ae2d5465d7205` (Marker07 leaf implementation receipt commit; source parent includes `b521ce939062ae09b485f4127ed4bc1b0015bbf9`).

| Source file | SHA-256 |
| --- | --- |
| `web/src/presentation.rs` | `293a30303cf13e41365029d6cbe5c16e735449fa409a3a18a0baaf3237a2e951` |
| `web/src/presentation/keycaps_finding_marker.rs` | `e10cc940cec1abb2c22af9e0a8465e4ead6fa79f9d2ef5bc8836e2983f8474e3` |
| `web/src/presentation/keycaps_navigation.rs` | `e5a303d38e18a30e996ce2c8103d04a3e2cd15d89a423190d44f5c2cda2d39f9` |

## Change and test seam

The Editor now calls the shared `focused_finding_for_admitted_route` projection after accepted request admission. The same shared `use_retire_stale_finding` hook owns clearing the identity when the accepted Layout workspace/scope/token/revision/board changes. The focused browser-mounted navigation probe uses both production helpers together with the production accepted-request admission, route dispatch, pending Layout-fit hook, and marker renderer.

The mounted test clicks the Keycaps route action against a valid accepted source and board target, verifies Layout routing and outline selection, asserts the accepted finding identity and contour are rendered, and verifies the queued destination-fit effect settles. It then leaves Layout, verifies the production retirement hook clears the identity, and confirms the marker remains absent. This is a mounted Dioxus production-seam test with controlled accepted-source, destination geometry, and frame ports; it is not a full packaged Editor journey or a real canvas/Inspector DOM click.

## Red/green and checks

- Expected red: temporarily changed only `focused_finding_for_admitted_route` to return `None`. The mounted test failed at `accepted Layout route renders its finding marker` after the accepted route. Restored the production helper and reran green.
- Green: `wasm-pack test --headless --chrome . --no-default-features --features page --bin boardstudio-web -- presentation::keycaps_navigation::tests::mounted_accepted_route_publishes_layout_marker_then_retires_it_on_workspace_change` — 1 passed, 32 filtered.
- `cargo clippy --manifest-path Cargo.toml --target wasm32-unknown-unknown --all-targets --features page -- -D warnings` — passed.
- `cargo fmt --manifest-path web/Cargo.toml -- --check` — passed.
- `git diff --check` — passed.

The initial browser harness run failed before interaction because the test clicked before the asynchronous Dioxus mount had rendered its button. Adding an initial settle fixed the harness; the later expected-red and green tests exercise the route behavior itself.

## Paired browser observation carried forward

The separate public journey receipt is `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-nav-c6-public-journey-34739.md` (SHA-256 `5941ac1faf632658227db3ddfbb8c7b72cbd960c362b996d44c163bed483fcc7`). It records the same retained c6 archive in React 5173 and Dioxus 34739. The two apps route the first SW1/SW2 clearance action to Layout, but Dioxus selects the key-cell Inspector (`Select: Key`, “keys · Key 1.1”) while React shows the component Inspector (`Select: Part`, “left-keys-SW1”); the Dioxus camera is also about twice as close and clips more geometry. The c6 archive contains no Case data, so generated Case layer/body Inspector ownership remains untested. These are open paired-parity gates for Issue05/F6C.4, not waived by this source test.

The candidate predates Marker07, so its missing focused marker in that paired screenshot is expected. The original missing F6C.4 fixture, camera/selection/Inspector behavior, Case ownership, full parent joins, and paired history/reopen gates remain open. No shared RUN/task/RF ledger was edited.
