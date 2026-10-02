# Compact Case containment public-browser evidence

This packet records a public-UI run of the compact Case containment candidate before the keyboard-focus correction. It is a partial green result: layout fit, panel access, and generated preview were verified; keyboard focus is red, so the candidate is not accepted as a complete fix.

## Served build and fixture

- Source: `b6d2af49e519b282abaf8e026dcaca1232a0fd19`
- Build: `frontend-case-compact-containment-20261002`; build provenance is in `provenance.json`.
- Root route: `http://127.0.0.1:34679/`; served HTML and app bundle are retained as `served-root.html` and `served-root-app.js`.
- Scoped route: `http://127.0.0.1:34679/boardstudio/`; served HTML and bundle are retained as `served-subpath.html` and `served-subpath-app.js`.
- Offline manifests: `offline-manifest-root.json`, `offline-manifest-subpath.json`.
- Fixture: `../mechanical-ui-layout/candidate-reviung41.boardstudio`, SHA-256 `28fb3f3b989ea2eb6d3a7bb6f09e6997794696a0576dd98f99bb38a5e3dca324`. It was imported through the public import UI in isolated candidate profiles, then generated through the public Generate case control.
- The generated case reached “Exact case geometry ready” with 58 layers and 88 mechanical diagnostics. The mesh is visible with drawers closed in `case-390x844-both-closed.png`, `case-390x640-both-closed.png`, and the subpath screenshot.

## Fit and panels

The retained `measure.py --assert-fit` oracle was run for both served routes, at 390×844 and 390×640, for all four Objects/Inspect states (both closed, Objects only, Inspect only, both open). The result JSONs are `candidate-390x*.json` for root and `subpath-candidate-390x*.json` for the scoped route. All document dimensions fit the exact viewport with no document scrolling. The root and subpath both-open geometry uses two 195px columns at 390px; the panel rectangles do not overlap each other. Closed drawers leave the generated mesh, top navigation, footer, and status visible.

At 390×640, the Objects tree was expanded and scrolled to its last control. `object-scroll-containers.json`, `object-scroll-last-control.json`, and `expanded-tree-scroll.json` show the tree's own scroll container (1088px content in a 251px viewport) reached the last control. `keyboard-enter-toggle.json` and `keyboard-space-toggle.json` record Enter and Space toggling the Objects/Inspect navigation controls while retaining focus.

## Keyboard focus red

The “no overlapping focusable controls” condition did not pass. In a fresh real keyboard sequence at 390×640 with both drawers open and the Objects tree expanded, focusing its last visible tree button (`thumbs Independent`, snapshot ref `@e181`) and pressing Tab focused `Generate case` in the workspace. Its rectangle was x=154.06, y=99, w=98.13, h=44; the point at its center hit the overlaid `Authored case bodies` Inspector region. This independently reproduces the keyboard-occlusion issue; `all-focusable-hit-test-390x640.json` inventories the underlying focusable workspace elements and hit targets. The panel-fit result must not be treated as full compact-containment acceptance until this is fixed and the same sequence is green on a fresh build.

## Scope limits

This packet proves the two listed viewport sizes, the two served routes, these panel states, tree scrolling, and keyboard toggles. It does not establish general responsive behavior at other dimensions, other workspaces, assistive-technology behavior, or that every control outside this fixture is usable. No DOM/CSS override or persistent storage write was used for measurement; app controls and public import/generation were used.
