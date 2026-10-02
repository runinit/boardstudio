# Compact Case focus and resize public-browser verification

This packet verifies the reviewed compact focus-entry and viewport-retention behavior against the final combined public build. It does not claim general accessibility or responsive coverage beyond the specific fixtures, states, and viewport sizes below.

## Served build and accepted fixture

- Source: `e2a84d8b00418eab4d1ec47e0bf5b6957da9ffe3`
- Build: `frontend-case-focus-resize-final-20261002`; exact build provenance is retained in `provenance.json`.
- Root route: `http://127.0.0.1:34683/`; served HTML and app bundle are `served-root.html` and `served-root-app.js`.
- Scoped route: `http://127.0.0.1:34683/boardstudio/`; served HTML and app bundle are `served-subpath.html` and `served-subpath-app.js`.
- The served bundle identities were checked against the build output. Root bundle is `boardstudio-web-dxh368001b4410731d.js`, SHA-256 `1513db70af380b11d56e2a62892c964dced324dac52757f39186653e66cf9214`; scoped bundle is `boardstudio-web-dxhc411b8c283727da5.js`, SHA-256 `f2ed772aa2c49dba975c7ecf47a29220127fdff4b1615850f9c0360e15f7c3b7`.
- Fixture: `../mechanical-ui-layout/candidate-reviung41.boardstudio`, SHA-256 `28fb3f3b989ea2eb6d3a7bb6f09e6997794696a0576dd98f99bb38a5e3dca324`. It was imported through the public file picker in isolated browser state at each route, then generated through the public Generate case control.
- Both sessions reached “Exact case geometry ready.” The rendered case preview remained present after panel and focus interactions. Screenshots with drawers closed are `case-390x640-closed.png`, `case-390x844-closed.png`, `subpath-case-390x640-closed.png`, and `subpath-case-390x844-closed.png`.

## Compact panel matrix

The read-only retained `measure.py --assert-fit` oracle was run for both served routes at 390×640 and 390×844, in all four states: both drawers closed, Objects only, Inspector only, and both open. The 16 distinct matrix cases are named `root-390x*.json` and `subpath-390x*.json`; final closed-state checkpoints add duplicate measurements. Every run reports document dimensions equal to the exact viewport dimensions, with no page overflow. `both-open-390x640.png` captures both compact overlays. The generated preview remains ready in each route’s closed-drawer state.

## Focus entry and reverse navigation

On the root route at 390×640, with both drawers open, a real Tab from the final Objects tree control moves focus to the actual “Generate case” button. The browser reports that button as active, both panel toggles as collapsed, and `elementFromPoint` at the focused control’s center as “Generate case.” Evidence is in `both-open-tab-entry.json`; `both-open-tab-entry-390x640.png` shows the resulting focus ring, closed drawers, and rendered mesh. Shift+Tab returns focus to the visible Inspector toggle while both drawers remain closed (`tab-entry-states.json`).

Equivalent public sequences on the scoped route cover both-open, Objects-only, and Inspector-only states. In each state, Tab reaches the visible Generate case button with both drawers closed, and Shift+Tab returns to the visible Inspector toggle. See `subpath-both-open-tab-entry.json`, `subpath-objects-only-tab-entry.json`, `subpath-inspector-only-tab-entry.json`, and the corresponding `*-shift-tab.json` results.

`keyboard-toggle-probe.py` focuses the actual public Objects and Inspector toggle buttons and uses real Enter/Space key presses. `keyboard-toggle-states.json` shows each panel opening and closing while focus stays on its toggle.

## Focus entry and resize retention

The desktop-to-compact regression sequence was run on the scoped route. Starting at 390×640 with both drawers open, the viewport was resized to 1280×577, then the actual “Generate case” workspace button was focused and the viewport returned to 390×640. Focus remained connected to the same button. The immediate read after the resize still saw both compact drawers open and the button center covered by Inspector; after an explicitly issued 600ms browser-driver wait (an observation delay, not an acceptance threshold), both `aria-expanded` states were false, both slots were `display:none`, and hit-testing at the retained focus landed on Generate case. The initial and later observations are both recorded in `desktop-focus-return-compact-states.json`, driven by `desktop-focus-return-compact-probe.py`. Independent read-only timing runs on the same final build measured resize-event-to-closed/uncovered at 3.6, 3.1, and 3.3ms, with zero covered samples across 30 compact animation-frame reads per run. The earlier immediate driver read is therefore classified as observing queued state/render settlement, not a persistent focus-occlusion defect. Exact assessment, raw captures, and its observer are copied under `resize-settlement/`. These are observations, not a response-time guarantee or a new product threshold. For the public green check, the harness observes until the same workspace child retains focus, both drawers report closed, and hit-testing is unobscured.

A separate panel-focused resize control starts with both drawers open and the real Objects button “right keys Independent” focused at 390×640. That element remains connected and focusable under hit testing after resizing to 390×844 and 1280×577; the generated preview remains ready and document dimensions fit. See `retained-focus-resize-states.json` and `root-1280x577-both-open.json`.

## Scope limits

The packet covers the REVIUNG41 accepted archive, the Case workspace, public import and generation, the two named routes, 390×640 and 390×844 compact panel combinations, and the single targeted resize sequence to 1280×577. It does not establish full application responsiveness, other workspace behavior, assistive-technology behavior, or all keyboard navigation paths. No DOM/CSS override, private provider API, or direct storage manipulation was used. Public import and generation used the isolated browser profile and saved the fixture through the normal app flow.
