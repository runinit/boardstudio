# Explicit Part navigation to the Component Inspector

## Fixture and source

- Isolated source worktree: `/home/chris/.local/share/boardstudio/worktrees/keycaps-component-inspector-20261003`.
- Starting integration source: `a201a96a76c0d908580793e36e4c7d315155fbb3`.
- Browser repro fixture: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- Paired pre-fix public repro is recorded in `public-a201-vs-react-repro.md`; that receipt identifies the pinned React source, candidate provenance, route and screenshots.

## Defect and change

A matrix primary part has an ordinary hit-test context of `Key`, while Keycaps finding navigation deliberately publishes a `Component` context with no matrix/row/column identity. The production selection handler preserved that explicit intent, but `layout_component_inspector_projection` compared it to `context_for_part` and rejected it. The Inspector then fell through to key controls. The same route left the toolbar selection kind at `Key`, unlike the pinned React `Select: Part` state.

The projection now validates the selected context against `component_context_for_finding_part`, while keeping the accepted-scope/current-context and single-selected-part guards. The shared tree-selection path changes the toolbar mode to `Part` only for that exact explicit Component context. Normal Key, Column, Row and Matrix contexts remain unchanged.

## Regression evidence

`expected-red.log` records the new mounted WASM browser test failing before the projection correction: explicit finding navigation produced no Component projection (`None`, expected `U1`). `green.log` records the six-test `presentation::layout_component_inspector_tests` suite passing after the correction, including the regression that asserts ordinary matrix selection remains Key and explicit finding navigation publishes Part and mounts the Component Inspector.

Command used for the green suite (reusing the existing WASM target):

```sh
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/keycaps-navigation-mounted-owner-20261002/web/target \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/chris/.cache/.wasm-pack/wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen-test-runner \
CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 \
cargo test --target wasm32-unknown-unknown --no-default-features --features page \
  --bin boardstudio-web -- presentation::layout_component_inspector_tests --nocapture
```

Formatting and `git diff --check` pass. Strict all-target WASM Clippy passed with `-D warnings` using the existing target directory. A rebuilt public candidate is still pending; the public a201 repro is historical and remains a red result, not post-fix acceptance.

Strict check command:

```sh
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/keycaps-navigation-mounted-owner-20261002/web/target \
cargo clippy --target wasm32-unknown-unknown --no-default-features --features page \
  --all-targets -- -D warnings
```

## Refactor takeaway

Ordinary hit-test context and explicit finding-navigation context are distinct selection intents for the same Part. Consumers that canonicalize every Part through `context_for_part` can erase a valid explicit Component route. Preserve this as a future selection/projection consolidation opportunity: define and document which consumers require hit-test context versus explicit inspector intent, while keeping current safety validation at each consumer. This immediate patch only corrects the authorized Keycaps route and does not widen public APIs.
