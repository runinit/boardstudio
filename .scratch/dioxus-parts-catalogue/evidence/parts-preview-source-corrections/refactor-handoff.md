# Parts preview source-correction handoff

This source-only correction reinforces existing RF-006 and RF-009; it does not establish a new refactoring finding or close a parent task.

## RF-006 — accepted resource identity and retained hook state

Dioxus 0.7.10 `ScopeContext::use_hook` retrieves an existing hook by cloning it (`dioxus-core-0.7.10/src/scope_context.rs`, `use_hook_inner`). Interior mutation of a plain `Cell` or `RefCell` returned by the hook therefore changes only that render's clone. The Parts preview now keeps `Rc<Cell<_>>` and `Rc<RefCell<_>>` hook values so request generation persists across renders. Its source fixture covers same-resource repaint, definition A→B→A, and accepted full-scope A→B→A transitions; a stale request cannot reclaim the returned identity.

The identity remains page-local and read-only. It carries the existing accepted `Scope`, snapshot token, and definition id; no new session owner, store, or public API was introduced.

## RF-009 — geometry frame and evidence accounting

`footprint_forms::point` already converts native KiCad Y to the projected preview coordinate (`Point(x, -nativeY)`). `GraphicElement` consumes those projected points beneath the preview's single outer SVG flip. Bounds now use the projected coordinates directly, including circle extents, and retain React's origin-inclusive viewBox policy. The asymmetric fixture checks the source-space line, keycap envelope, origin, and 3-unit margin instead of relying on symmetric MX geometry.

The same bounded source tests cover React's authored-keycap precedence, retained Ergogen width/height and `include_keycap` defaults, and the `is-mechanical` class for unplated drills. These are regression fixtures in source only. No Cargo/native/WASM/browser check was run in this worker; those gates remain with the integration owner.
