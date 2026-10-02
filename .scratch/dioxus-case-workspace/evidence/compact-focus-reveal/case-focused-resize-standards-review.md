# Compact Case focused-resize repair — Standards review

Reviewed integration `web/src/presentation/panels.rs` SHA-256 `aab64ce779f5980eb61e19f521fb9522bfb7c487538fd9542e417b710560fefb`.

**Clear; no material Standards finding.** The existing media-dependent effect closes only its own compact visibility Signal when entering compact mode and the current active element belongs to a workspace containing the direct Case panel. The DOM predicate uses current activeElement, closest workspace ancestor, and a direct-child selector; panel/sidebar focus and other workspaces do not match. Missing document/element or query failure safely yields false.

The change reuses the existing compact Signal, media listener and effect/drop lifecycle. It adds no listener, timer, imperative focus movement, remount, domain mutation, API or new state owner. Reading the DOM inside the effect does not introduce a reactive dependency on the panel visibility it writes, so the close does not self-trigger the media effect. Desktop transitions do not execute the close branch. Existing inert/aria-hidden and compact display rules still govern hidden drawers.

This complements focusin handling: resize can cover an already-focused workspace control without another focus event, and the media effect now reveals it while retaining that focus. Root must verify the built-page desktop-to-compact transition with real active controls, both-open drawers, panel-focused controls and other workspaces. Retained old red is diagnosis/regression evidence; source clearance is not browser acceptance. No source edits, Cargo or browser execution. No new refactoring takeaway observed.
