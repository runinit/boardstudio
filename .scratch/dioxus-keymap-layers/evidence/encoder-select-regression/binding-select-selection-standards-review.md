# Binding select selection — Standards review

Reviewed `frontend-keymap-layers-20261002` working-tree `binding_editor.rs` against `0302177a`, exact SHA-256 `01354a385e46ce283f73427f993bd74eb1f0d4385abd76a89d990c14909ff74e`.

No material Standards findings. The delta adds only four declarative option-selection comparisons: accepted behavior, hold modifier, stable layer ID, and stable macro ID. Each agrees with its existing parent select value and uses the existing projection; no new state, copied domain logic, callbacks, lifecycle effects, imperative DOM synchronization, or API visibility changes are introduced.

Installed Dioxus 0.7.10 directly supports the mechanism: `dioxus-html/src/elements.rs:1536` declares option `selected: Bool volatile`, and `dioxus-interpreter-js/src/js/common.js` applies it through `node.selected = truthy(value)`. This is current selectedness, rather than the separate `initial_selected` default. False comparisons also clear the property on later renders. The mapping therefore addresses dynamic-option insertion while preserving the controlled accepted-value model and existing operation admission/feedback behavior.

This follows CONSTRAINTS.md Rust/Dioxus requirements for version-matched framework decisions and deliberate ownership, and its UI requirement to preserve truthful semantic controls. The root-reported expected-red case (accepted None displayed as Keypress) supplies the defect oracle; no browser run, Cargo, or test execution was performed by this reviewer. Root must verify initial mount plus change/Undo/scope refresh in the actual production page, including the other three affected selects. No new refactoring takeaway observed.
