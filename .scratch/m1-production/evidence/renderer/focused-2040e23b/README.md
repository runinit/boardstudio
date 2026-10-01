# Focused CasePanel green-stage browser QA

This focused artifact is for source page `2040e23b` / source commit `e4b019a1072a1a2f0964400119fb60758068dd5e`, as identified in `web/target/builds/m1-focused-2040e23b/provenance.json`. It was served with Python's static HTTP server at `http://127.0.0.1:46655/`. This stage intentionally omits the offline policy and is not the final complete release. All checks used Chromium through agent-browser session `m1green2040-7dd31abc1fc4`.

## Green results

- **REVIUNG41:** opened the editable copy, clicked **Add case settings**, then **Generate case**. The UI reported “Exact case geometry ready.” Canvas mounted active at 1210×320 CSS/backing pixels, DPR 1, render submissions 2. Screenshot: `casepanel-reviung41-ready.png`.
- **Resize and DPR:** set the viewport to 980×720 at device scale factor 2. Canvas became 910×320 CSS pixels and 1820×640 backing pixels; `data-dpr=2`, resize count advanced to 2, and submissions advanced to 3.
- **Sofle v2, Left PCB:** opened the editable copy and generated without adding settings. Exact geometry succeeded; a new active canvas rendered at 1820×640 backing pixels / 910×320 CSS pixels at DPR 2. Screenshot: `casepanel-sofle-left-ready-dpr2.png`.
- **Scope change/remount:** changed Sofle from Left PCB to Right PCB. The old canvas unmounted immediately, the ready message cleared, and the CasePanel returned to its settings prompt. After **Add case settings** and **Generate case**, Right PCB returned “Exact case geometry ready” with a new active canvas and render submissions 2. Screenshot: `casepanel-sofle-right-ready-dpr2.png`.
- **Cancel and retry:** while generating Sofle Right, the test observed the enabled **Cancel generation** button within a 16ms polling step and clicked it. UI said “Case generation cancelled”; the existing canvas stayed active. An immediate retry succeeded and advanced render submissions to 4, showing that cancellation did not poison subsequent work.
- **WebGL context loss:** the `WEBGL_lose_context` extension produced `webglcontextlost` and renderer state `context-lost`. Calling `restoreContext()` emitted `webglcontextrestored`, but the mounted canvas remained `context-lost` with no new submissions. Switching to a fresh Sofle copy unmounted that canvas; its newly generated canvas worked. The verified recovery route is remount/re-generation; automatic in-place restoration was not observed.
- **Axe:** the active Sofle Right canvas state had 39 passes, zero violations, and one incomplete `color-contrast` rule over 10 SVG text labels. Raw axe 4.12.1 output is `axe-sofle-right-ready-dpr2.json`; those labels’ foreground/background were manually checked in the old release companion accessibility report.

## Scope and remaining limits

This proves exact generation for both fixture families, DPR-aware resize, scope invalidation, renderer remount, cancellation, and successful retry on the focused artifact. It does not prove offline worker registration/cache behavior; this stage deliberately has no service worker. It also does not prove automatic WebGL restoration without remount, export URL cleanup, screen-reader output, or final build provenance. The corrected full release still needs startup/root-subpath and freshness-sensitive acceptance checks.
