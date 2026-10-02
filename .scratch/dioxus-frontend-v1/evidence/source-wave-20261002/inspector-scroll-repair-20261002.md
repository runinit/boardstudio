# Shared Inspector scroll repair — 2026-10-02

Base: adb8a442ac3014fdc11b3fd3c0773e5e6380fab9. Private production changes are InspectorPanel's content wrapper and its desktop/compact CSS overflow rules. Heading and panel options remain outside that body. ObjectsPanel and canvas structure are unchanged.

## Actual failure and diagnosis

Fresh Sofle copy, Keycaps, selected SW1, public candidate34723 (d425a2df), viewport1280×940: editor frame834px versus content1778px; frame overflow hidden and Keycaps content overflow visible. Actual coordinate-specific native wheel at Inspector center could not scroll. `wheel-probe.mjs ... scroll` failed as expected; native-red JSON/log retained. The installed agent-browser mouse wheel command dispatched at the topbar despite a preceding mouse move, so that initial CLI wheel attempt is not counted as wheel evidence. The probe uses the CDP endpoint of the agent-browser-owned session, records coordinates and DOM metrics, and dispatches native Input.dispatchMouseEvent mouseWheel; it never assigns scrollTop or calls scrollIntoView.

Three predictions were distinguished: a missing shared scroll owner should respond to a bounded wrapper; a leaf-only fix would leave Layout sibling sections unhandled; a global editor overflow change would move canvas/Objects. The wrapper fixed the desktop DOM/CSS prototype with Inspector scroll0→944 and stable canvas/Objects. Native keyboard traversal reached Find a key/Selected key with controls visible. This prototype is supporting diagnosis, not a packaged fixed-source acceptance claim.

Compact testing revealed an additional wheel trap from overscroll containment on unbounded ordinary page content. At640×900 the native wheel stayed at pageY0; preserving ordinary compact overflow visible/overscroll auto restored pageY1200 with no horizontal overflow. Compact Case has a deliberately bounded overlay and retains its own body scroll owner.

## Production regression seam

`panels_scroll_tests.rs` mounts actual InspectorPanel and ObjectsPanel with actual m1.css, two sibling content sections and forty focusable controls. Desktop test proves a bounded scroll ancestor before the clipped frame, native focus scrolls the last control into view, heading remains fixed, options menu stays outside the scroll body and canvas dimensions do not change. Old production code failed with “long Inspector content must have a wheel-scrollable ancestor before the clipped editor frame”; corrected production passes.

Compact tests use the production media rules and separate ordinary page flow from a Case overlay. The fixture root has a desktop-only height constraint so it does not override production compact auto-height. The initial compact fixture incorrectly imposed a fixed inline root height; diagnostic focus scrolled a body ancestor with document723px/content2138px, so its window.scrollY assertion was not an application failure. This fixture was corrected, not production behavior weakened. An earlier real compact regression asserts overflow visible; it failed with actual auto before the compact exception.

## Evidence limits

Native page-bin91 tests passed. Final production desktop browser test passed. Final desktop1/1 and compact2/2 production Chromium tests pass. True WASM page all-target strict Clippy (`--all-targets -- -D warnings`), formatter and source/evidence diff checks pass. Copied log whitespace is normalized; original raw outputs remain under /tmp. Independent source review and fresh packaged candidate native-wheel/keyboard/context-menu checks remain required. No parent/full UI acceptance claim, no public API or visibility widening, no suppressions. RF-006/RF-009 handoff: shared Inspector owner and trustworthy wheel coordinates/provenance; root owns canonical ledger.

## Frozen source hashes

- `web/src/presentation/panels.rs`: `e1f1a92de4da0f5cee8aa0ec0cc59474c51446b0396ce2ed96397287ca7820b2`
- `web/assets/m1.css`: `86d082391e4061ba321b86315c2f42042831ac94dad65db5fb7135cd29b39e24`
- `web/src/presentation/panels_scroll_tests.rs`: `732d0dd7acbd6f6504d27ebafd072715589a4a376b69f02bc14de826c42e47e9`
