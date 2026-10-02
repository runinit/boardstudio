## Standards

**Documented standards gaps**

- The new browser generator boundary is only partially documented. The F3a slice names the retained Ergogen service and says Rust owns SVG projection (`.scratch/dioxus-frontend-v1/issues/03a-layout-layers.md:20–24`), while `FootprintGraphics::drawing` dynamically imports it and invokes `render` after an await (`web/src/presentation/footprint_graphics.rs:25–95`). `CONSTRAINTS.md:89–95` requires the slice design and architecture documentation to record purpose, caller/callee, data contract, ownership, errors and cancellation, cleanup, performance cost, and retirement condition where relevant. The ticket omits cancellation/cleanup and cost; `docs/architecture.md` is unchanged. Record this boundary and its execution/lifecycle limits before treating F3a as accepted. This is a documentation/acceptance gap; the evidence does not show a public API or persistence change.

- Accessibility parity is still an open verification gate, not an observed code violation: `CONSTRAINTS.md:156–160` requires zero newly introduced axe violations plus manual keyboard and relevant screen-reader checks. F3a lists compact/Escape/focus checks (`issues/03a-layout-layers.md:31–36`), but no browser result was included in this review input. Avoid marking the layer UI accepted until those checks are recorded.

**Possible smell (judgement call)**

- Possible duplicated code: the pitch-minus-edge-gap fallback is independently implemented for ghost keycaps and scene-part keycaps (`web/src/presentation.rs:995–998` and `1023–1027`). A shared private helper would keep the two rendered outlines consistent when this sizing rule changes. This is a small duplication and not a documented-standard violation.

No other documented standards breach or material ownership/API/persistence violation found in the reviewed diff. `git diff --check` passes; no tooling-enforced issue is reported.
