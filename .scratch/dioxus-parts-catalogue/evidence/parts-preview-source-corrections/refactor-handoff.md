# Parts preview source-correction handoff

This source-only correction reinforces existing RF-006 and RF-009; it does not establish a new refactoring finding or close a parent task.

## RF-006 — accepted resource identity and retained hook state

Dioxus 0.7.10 `ScopeContext::use_hook` retrieves an existing hook by cloning it (`dioxus-core-0.7.10/src/scope_context.rs`, `use_hook_inner`). Interior mutation of a plain `Cell` or `RefCell` returned by the hook therefore changes only that render's clone. The Parts preview now keeps `Rc<Cell<_>>` and `Rc<RefCell<_>>` hook values so request generation persists across renders. Its source fixture covers same-resource repaint, definition A→B→A, and accepted full-scope A→B→A transitions; a stale request cannot reclaim the returned identity.

The identity remains page-local and read-only. It carries the existing accepted `Scope`, snapshot token, and definition id; no new session owner, store, or public API was introduced.

## RF-009 — geometry frame and evidence accounting

`footprint_forms::point` already converts native KiCad Y to the projected preview coordinate (`Point(x, -nativeY)`). `GraphicElement` consumes those projected points beneath the preview's single outer SVG flip. Bounds now use the projected coordinates directly, including circle extents, and retain React's origin-inclusive viewBox policy. The asymmetric fixture checks the source-space line, keycap envelope, origin, and 3-unit margin instead of relying on symmetric MX geometry.

The same bounded source tests cover React's authored-keycap precedence, retained Ergogen width/height and `include_keycap` defaults, and the `is-mechanical` class for unplated drills. These are regression fixtures in source only. No Cargo/native/WASM/browser check was run in this worker; those gates remain with the integration owner.

The asymmetric bounds fixture covers projected graphics that the Dioxus renderer
actually draws. React's `ergogenPreviewPoints` additionally scans raw forms,
including hidden/reference text, before rendering. This correction does not
claim full raw-form framing parity; the paired MX fixture's raw-form extent
comparison remains open.

## RF-002/RF-009 — packaged service reachability and provenance

The preview loader needs the retained `parameters(source)` function from
`ergogen/src/index.ts`. The generated frontend entrypoint previously exposed
only `isErgogen` and `render`, even though the underlying source exported
`parameters`. The private build bridge now exports that existing function and
records the entrypoint hash in generator provenance. The packaging script
imports the actual generated entrypoint and checks its named exports and
`ceoloide/switch_mx` defaults (`18 × 18`, `include_keycap=true`) before it
reports success. This is the existing asset boundary, not a new engine or
Core API.

Source package check run from the Parts worker on 2026-10-02:

```text
node scripts/web/build-layout-generators.mjs /tmp/parts-preview-generator-package-05a1dac4-retest
Verified generated catalogue for 36 modules
Verified packaged generator service and ceoloide/switch_mx preview defaults
```

The generated `layout-generators.js` SHA-256 was
`c07cbe7aaff7f5da50e6db7cd952e35d316d87e847027bbf7dc5a4886139c547`; its
provenance records `ergogen/src/index.ts` SHA-256
`930dc6f8cd1d172d6ff181bf310d99a3ae5fe8ed51d820236bb3e388bd1146dd` and
`ergogen/generated/catalogue.mjs` SHA-256
`3acdf3c1000658409c0f288eb338668aa9783a0688c3a37ba91a7b0120ec2b97`.
This validates the generated package locally only. Root/subpath delivery,
offline service-worker reload, WASM compilation, and browser parity remain
integration gates.
