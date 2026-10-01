# P2 version-matched API verification

Verified2026-10-01. Context7 resolved /dioxuslabs/dioxus/v0.7.10; its tagged
effect documentation was sufficient for reactivity but omitted downcast/drop.
Five official v0.7.10 source files were fetched and exactly match installed
crate bytes. api-verification.json retains URLs, hashes, dates and gzip archives.
These are source facts; no probe compile, mount, paint or teardown is certified.

- Mounted conversion requires the explicit mounted feature with minimal/web.
  WebEventExt::try_as_web_event on MountedData returns an optional web_sys::Element,
  then wasm_bindgen::JsCast::dyn_into can check HtmlCanvasElement or SvgElement.
  Handle absent/wrong element as attachment failure. Do not cast directly from
  MountedData to HtmlCanvasElement. Sources: [converter](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/web/src/events/mod.rs),
  [mounted backing](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/web/src/events/mounted.rs).
- PointerData's WebEventExt mapping returns a web_sys::PointerEvent with matching
  pointer ID, coordinates and modifiers. Capture/release are Element methods in
  installed web-sys0.3.106. Pointerup's own coordinates, unrelated-pointer guards
  and pointercancel/Escape policy remain session/host responsibilities.
  [Pointer source](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/web/src/events/pointer.rs).
- use_effect tracks signal reads, queues post-render callbacks and deduplicates
  pending effects. An effect rerun is not a resource teardown protocol; persistent
  host ownership and explicit cancellation are still required.
  [Effect source](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/hooks/src/use_effect.rs).
- use_drop installs a lifecycle closure via a hook-owned Rc; its lifecycle object's
  Drop takes and calls the closure. Releasing host listeners/observers/RAF and
  renderer dispose/free remains explicit work in that closure or host Drop.
  [Lifecycle source](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/core/src/global_context.rs).
- The generic mounted set_focus implementation requires HtmlElement; SVG cannot
  use that conversion. Installed web-sys0.3.106 has SvgElement::focus and observer
  disconnect/Window RAF cancellation APIs. Record browser errors and test real
  SVG tab/focus behavior when P2 resumes; source availability proves no parity.

Existing renderer resize/dispose plus generated free are the public seam. Source
inspection supports feasibility, not GPU lifetime/resource acceptance. Toolchain
and pins remain unchanged; full runtime evidence is still blocked/unperformed.
