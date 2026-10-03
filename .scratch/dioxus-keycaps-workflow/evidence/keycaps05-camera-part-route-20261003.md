# Keycaps issue 05 — camera contour and explicit Part route repair

This is a bounded source repair under issue [05](../issues/05-keycap-fit-finding-navigation.md). It preserves the F6C.4/INT.2 joins and does not claim public paired acceptance.

## Frozen source

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/keycaps-navigation-mounted-owner-20261002`
- Branch: `codex/keycaps-navigation-mounted-owner-20261002`
- Source commit: `7595f1dd2a9fd85aedfb54608d724de97e423ca8`
- Spec/ticket clarification commit: `acdf5bb7d327265e0224d1c5e9029fc826a7cfaf`
- `web/src/presentation.rs` SHA-256: `0fcd3c36c0c47ccef87ec93413b15bef8601886e2d8a85d30281fb625eeac543`
- `web/src/presentation/keycaps_fit.rs` SHA-256: `f9e0bc1f8fde2c2f180a806cbb1c3f234746d8cc18aa01f50698e4ed969abb2c`
- `web/src/presentation/keycaps_navigation.rs` SHA-256: `c827c371dd369ebbe785a2d9d2ec6241ee381453e1f3744ad536a6d2aca29aca`
- `web/src/presentation/objects.rs` SHA-256: `7ed948ae47ba072ca87511c96508da672801f776596bc6e7620a213719f6182f`
- `web/src/presentation/objects/tree.rs` SHA-256: `08d4c1b1b2af5afd7851ee59383b362d258c4adaca12740fa95dfeee91690278`
- `web/src/presentation/objects/tree_tests.rs` SHA-256: `a821fb49509044a0ae1e4504badf1e92b43a1b833b24684a70b4f8918549a9a7`

## Behavior change

- The destination Layout fit now prefers all non-empty Core marker contour points for the admitted finding and active board, with the existing target bounds as fallback. It filters non-finite coordinates and applies the existing 12 mm margin. This uses accepted Core scene output and does not recompute overlap geometry.
- A Keycaps `Part` finding now uses an explicit component/Part Inspector context, including a matrix-attached primary switch. Ordinary canvas selection still uses its existing Key context.
- The old mounted delayed-focus test now restores its modeled Keycaps workspace before submitting a second finding request. Its prior second request occurred while the first route had already switched that same test owner to Layout, where production admission correctly rejects new Keycaps requests.

## Regression evidence

All expected-red tests were produced by temporary test-only mutations, then the production implementation was restored:

- Marker fit expected-red log SHA-256 `62eec15eab0cc14b87609fbe730457ea8a803f1bde83bf776804ffc345b4e076`; marker selector mutated to target-only. The test failed because the part bounds `(28, 52, 58, 82)` were returned instead of full marker bounds `(-32, 92, -22, 122)`.
- Marker fit green log SHA-256 `fc246833dee3d73b7be46f9ba873f099e0f36f6bf3d48a6ea9bd947a23c891ac`; 1 passed.
- Explicit component-route expected-red log SHA-256 `fa9817d88bf514fc02d73b1ccfcdf0774888f223328e9f018d160a053f55bb0e`; projection mutated to retain the ordinary Key context. The test showed `Key { matrix_id: left-keys, row: 0, column: 0 }` where a Part action requires `Component { part_id: left-keys-SW1, matrix_id: None, ... }`.
- Explicit component-route green log SHA-256 `de031562b2ca9e98b424ba0052876e18481ac001f173111cb9083985b36cbe34`; 1 passed.
- Marker projection additionally passed its exact all-contours/same-finding/same-board and empty-marker fallback regression within the 8-test Keycaps fit suite.
- Production navigation-owner/lifetime suite: 9 passed, 0 failed; log SHA-256 `7717d91d79b5efd0cbc3899a5b0c74b1c3c37a1f58af0465e25a1106e6306f78`.
- Strict supported WASM Clippy (`--all-targets --features page -- -D warnings`) passed; log SHA-256 is recorded in the sibling `clippy-green.log`.
- `cargo fmt -- --check` and `git diff --check` passed on the frozen source.

All browser/WASM output logs are in this directory. The CodeGraph index was unavailable for this worktree, so source inspection used targeted reads.

## Still open

- The public paired c6 journey at 1280×577 is historical evidence against the pre-repair Dioxus build; it demonstrated the current discrepancy (React `Select: Part`, component Properties/Relations Inspector, full two-key contour framing versus Dioxus `Select: Key`, Key controls, one-key framing). This repaired source has not yet been packaged and replayed in the browser.
- The c6 fixture has an empty `case` object. Case body/generated-layer routing, original missing fixture f2 provenance, same-history/no-revision behavior, full Issue05 journey, and F6C.4/INT.2 parent joins remain open.
- No claim is made that a source regression substitutes for the paired TypeScript/Dioxus journey or full parent acceptance.

