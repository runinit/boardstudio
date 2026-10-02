# Mechanical settings responsive and input feedback QA

This packet records bounded public-browser checks for the fresh mechanical-settings build. It is not full F7.4 acceptance.

## Exact input and served build

- Candidate: source commit `0cad75775e84aba13dc2f88d513b9f662e037e5e`, build `frontend-mechanical-settings-20261002`, root URL `http://127.0.0.1:34675/`; the build provenance is copied to `provenance.json`. Root reported the matching `/boardstudio/` route, all seven build commands passing and 965 source hashes verified.
- Public test sessions: `mechanical-ui-layout-0cad7577` and `mechanical-ui-layout-mobile-0cad7577`; separate fresh React comparison session `mechanical-ui-layout-react` at `http://127.0.0.1:5173/`. Candidate test data was created by the visible REVIUNG41 copy action. No private runtime API was used and no browser storage was written through scripts.
- The candidate public Export action produced `candidate-reviung41.boardstudio` (SHA-256 `28fb3f3b989ea2eb6d3a7bb6f09e6997794696a0576dd98f99bb38a5e3dca324`). The extracted `candidate-exported-project.json` is exactly equal to the read-only IndexedDB ProjectDoc (`candidate-db-before.json`): project `edbec892-9952-4b6b-a7ba-cb6b0a514ac4`, revision 3, canonical sorted JSON SHA-256 `f3d5ae657e2ced9028720769684dc2666c7b21f41ad6530346a6ece23a0e778e`. The same candidate-produced archive was publicly imported into React before comparing the numeric field displays.

## Responsive viewer and settings

After the real public `Generate case` action resolved, the candidate showed `Exact case geometry ready.`, an enabled `Fit case` button, `Resolved stack · 58 layers`, 88 mechanical diagnostics, and the interactive 3D preview. The model is visibly rendered in both unobstructed screenshots:

- [Desktop 1280×577, generated preview](desktop-1280x577-preview-unobstructed.png)
- [Compact 390×844, generated preview](mobile-390x844-preview-unobstructed.png)

At 1280×577, the document and body fit the viewport (1280×577). The canvas bounds were x=247, y=164.8, width=701, height=266.1; the visible viewer and Case panel bounds also fit within the viewport. See `desktop-1280x577-preview-measurements.json`.

At 390×844, the document height is 924 px, so the page has about 80 px of vertical overflow. The canvas bounds were x=12, y=287.8, width=366, height=419.5 (bottom=707.3), fully inside both the viewport and viewer/panel bounds. The screenshot confirms visible mesh geometry rather than only an image role or canvas rectangle. See `mobile-390x844-measurements.json`.

The settings body remains independently scrollable when expanded: its measured scroll/client heights were 14,149/287 px on desktop and 15,085/420 px on compact. Scrolling the public Clearance control into view brought it within the viewport without losing the panel; the measurement and screenshot are `desktop-clearance-reachable.json` and `desktop-clearance-reachable.png`. Compact primary buttons, selects and numeric inputs measured 44 px high. The desktop counterparts measured 36 px high. The checkbox's native input is only 13 px wide; this packet does not claim that the checkbox meets any touch-target threshold.

The public Theme selector successfully switched between Light and Dark. Screenshots with the same compact viewport and settings scroll position are [Light](mobile-390x844-light.png) and [Dark](mobile-390x844-dark.png). The normal compact screenshot and numeric feedback captures are also retained; settings are closed in the two preview screenshots above so they do not cover the model.

## Numeric draft and validation behavior

Starting from the accepted revision-3 REVIUNG41 copy:

- No-op: filled Clearance with `0.45`, restored `0.3`, then pressed Enter. Read-only IndexedDB record remained revision 3 and byte-for-JSON equal to the baseline ProjectDoc (`mobile-db-after-noop.json`).
- Invalid: filled Clearance with `-0.1` and pressed Enter. The input exposed `aria-invalid="true"`; native validity was false with “Value must be greater than or equal to 0.”; the app displayed “Enter a finite value of zero or greater.” The accepted ProjectDoc stayed revision 3 and fully unchanged (`mobile-db-after-invalid.json`); the visible error is captured in `mobile-390x844-invalid-enter.png` and `mobile-invalid-enter-snapshot.txt`.
- Escape: focused Clearance, filled `0.45`, verified the input held focus, then pressed Escape. It returned immediately to `0.3`, cleared invalid state and feedback, and the accepted ProjectDoc remained exactly unchanged at revision 3 (`mobile-before-escape.json`, `mobile-after-escape.json`, `mobile-db-after-escape.json`).
- Exact-input display comparison: the candidate accessibility snapshot serialized `PCB thickness` and `Clearance` as long binary-float strings, but direct DOM reads from the same archive show `value`/`valueAsNumber` of `1.6`/`1.6` and `0.3`/`0.3` (`candidate-same-archive-input-dom.json`). React's DOM input values for the same publicly imported archive are also `1.6` and `0.3` (`react-same-archive-input-dom.json`). Therefore no visible field precision mismatch was observed; the long values are an accessibility-snapshot representation artifact.

## Generation timing and limits

Generation was not instantaneous. In the first candidate profile, the real Generate action still showed Cancel generation enabled and zero resolved layers at approximately 3 s and 13 s; later the browser resumed with 58 layers, 88 diagnostics and the preview image. A second profile was also ready by the next recorded poll. A separate read-only review of the first session found the ready state and no meaningful browser error, but no exact completion time or source cause was established. This is not classified as a generation regression or a performance pass.

This packet does not cover semantic Configure/Disable, split-instance ownership, Undo/Redo, generated-part acceptance, full assistive-technology behavior, other viewport sizes/zoom/DPR, or parent F7.4/F7.4e acceptance. Semantic mechanical settings and archive/history checks were assigned to the separate `tree_contract_verifier` profile.
