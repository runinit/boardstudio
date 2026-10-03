# F6K.1c implementation evidence

**Source commit:** `be4c2fbb3a8b42f588690319b3894127254a9746` (`feat: add Keymap layers disclosure parity`)

The Keymap panel now wraps its existing layer list, layer controls, feedback, and help text in a native `<details>` disclosure titled “Layers”. It starts expanded on mount; native disclosure state survives layer selection and ordinary rerenders. Base omits the unavailable Remove action and the former Dioxus-only protection sentence. Styling is limited to the owned Layers section.

## Verification

- Expected-red mounted regression against the original static section: failed because `details.m1-keymap-layers` was absent, as expected. Log SHA-256: `b6d956713e7d484af4872e26810adf390d805f389f94f0b20d2aaa2903d61b62`.
- Production-component Chrome/WASM regression on the frozen source: 1 passed, 137 filtered. It checks initial expansion and summary, collapse/reopen, collapsed-state retention across layer selection/rerender, Base/non-Base controls, and that disclosure interaction submits no operation. Log SHA-256: `8b864e7cfcd82929fa37c63ee03b4d840e864d9649c752912d0b6bff5ff967af`.
- Focused native Keymap panel lifecycle tests: 7 passed. Log SHA-256: `036019cd8082666c69aa4c3463aeb6d84f983e04bf64d015e7974eb5ee1a03d4`.
- Strict all-target WASM Clippy (`-D warnings`): passed. Log SHA-256: `4d949e4ef985f1bd0831b54f95fbfb74b00f08725c01e615e448e359ae768e6e`.
- `cargo fmt --manifest-path web/Cargo.toml --all --check` and `git diff --check`: passed after the final source change.

The production-component mounted test is scoped to this panel; it does not claim persisted document/history behavior beyond confirming that the disclosure invokes no edit callback. The paired public-browser candidate comparison remains pending root integration and candidate build. Shared Keymap shell, Objects controls, footer, and parent F6K.1/F3.1 acceptance remain open. This slice introduces no new RF observation.

The planning packet is published as issue 10 at `.scratch/dioxus-keymap-layers/issues/10-layers-disclosure.md`; planning review is recorded there and in `source-audit.md`. This evidence records implementation verification only and does not close parent or paired acceptance gates.
