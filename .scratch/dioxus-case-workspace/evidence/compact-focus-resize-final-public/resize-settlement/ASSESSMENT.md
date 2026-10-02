# Final compact resize settlement assessment

**No persistent focus-occlusion defect was reproduced on the final34683 build. No further source fix is proposed.** An immediate post-resize DOM read can observe the intermediate compact geometry before both existing panel instances have completed their state updates. The bounded observations below retain that transient rather than treating it as a settled UI state.

## Input and actual workflow

Own fresh browser session `case-focus-final`, profile `/var/tmp/frontend-run/case-focus-final-profile`, final root URL http://127.0.0.1:34683/. Publicly imported exact configured REVIUNG41 archive SHA28fb3f3b989ea2eb6d3a7bb6f09e6997794696a0576dd98f99bb38a5e3dca324. Selected Case, closed the initial Inspector to access Generate, clicked real Generate, and observed Cancel disabled plus interactive3D preview before timing. No fake providers, storage writes, application state injection, or source edits.

The first attempt to click Generate while Inspector covered it was rejected by the driver; closing Inspector was the ordinary public correction. One first observer retrieval used unsupported top-level await and failed in the driver; it was corrected to return a Promise. Neither tooling/setup event is an app regression.

`python3 /tmp/frontend-run/case-resize-settlement/observe.py case-focus-final OUTPUT.json` uses public controls to open both drawers at390×640, resizes desktop1280×577, focuses the actual Generate button, then resizes390×640. Read-only instrumentation records performance.now timestamps at resize/media events, panel aria-expanded mutations, and30compact requestAnimationFrame callbacks. It only observes DOM/focus/hit tests; it makes no app/DOM/style/storage mutations. Temporary observation data and listeners are removed after capture. Observer measurement overhead remains a limitation.

## Observed results

First3runs sampled partial-close→fullyclosed intervals2.2/1.6/1.5ms; these do not measure the start of resize. After adding an explicit resize-event observation, the next3runs measured resize-event→both drawers closed and focused Generate uncovered at3.6/3.1/3.3ms. In all6runs, Generate retained focus; all30compact rAF samples per run were already uncovered with both drawers closed. Raw generated-root-01…06.json and summary.json retain the complete observations. Do not claim literal paint timestamps or a general performance guarantee from rAF observations.

The independent verifier's subpath probe recorded an immediate covered focus and an uncovered state after a600ms driver wait. It did not measure closure duration and never established600ms as a threshold. The new event/frame observations explain why the immediate read can differ from settled state without waiting600ms.

## Source interpretation

Current panels.rs SHAaab64ce779f5980eb61e19f521fb9522bfb7c487538fd9542e417b710560fefb: each panel has its existing media-query listener. The listener sets compact; the existing compact effect checks current Case workspace focus and sets that instance's compact_open false. Dioxus then renders the associated classes/expanded state. The two panels update independently; mutation records actually captured one flag false before both became false. CSS viewport matching can change the drawer layout before the resulting state/render updates finish. No additional observer or focus move was added to production.

## Classification and limits

This evidence supports normal queued media/effect/render settlement, not the original persistent resize defect. Preserve the original old-build red and final immediate transient as distinct facts. For public green, wait for the semantic settled predicate (same focused workspace child, both expanded=false, unobscured hit test), retaining a bounded harness timeout solely to avoid hanging. Do not convert600ms or this machine's3.1–3.6ms observations into a new acceptance budget. Root's other controls—focus elsewhere preserving open drawers, actual Tab/Shift+Tab behavior, both panel states, viewport fit, other workspaces—remain governed by their separate evidence. No Cargo/build/source edits were performed; no new RF entry is warranted by this observation.
