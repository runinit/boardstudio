# F3.3-C02: drag admission and stationary reflow

Paired browser qualification: pinned TypeScript `5a472a9426e6e38993361da402cd4ec730feb369` (`http://127.0.0.1:5175/`, session `f33c02-layout-qual-2318ea1a1376`) and Dioxus build `frontend-functional-controls-20261003` (`http://127.0.0.1:34778/`, session `bs-mig-6280ea4d22a7`, source `ad26b07a7ada49aed1d7ac686fb32a8dbcf2ded6`). Dioxus package proof is `../../frontend-functional-controls-20261003/package-proof.json`; it reports no source/route mismatches.

## Results

- **Alt + 1 CSS px part drag:** U1 moved freely by about 0.43 mm on each surface (world X: Dioxus `273.200012 → 273.630004`; TypeScript `273.20 → 273.626727`). This confirms nonzero client motion admits a drag despite the small world-space delta. One Undo restored the original position and one Redo restored the moved position on both surfaces.
- **Stationary press during Inspector reflow:** with the pointer held at a fixed client coordinate, keyboard activation expanded the Inspector and changed the canvas width from 1013 px to 725 px. The pressed part/key and U1 retained their world positions on both surfaces; the stationary release did not create another history change. This isolates client-coordinate motion from world-coordinate changes caused by panel reflow.
- **Shift/Ctrl selection:** trusted Shift and Ctrl selection sequences with 8 px pointer travel left target part positions unchanged on both surfaces. Selection occurred without a drag or position history change.

Input used trusted browser mouse and keyboard events. No app runtime state was injected. The Inspector was toggled by keyboard while the pointer was held, so this verifies stationary reflow and release behavior, not a selection gesture that itself opens the Inspector.

## Conclusion

F3.3-C02 passes for the tested candidate: 1 CSS px Alt movement starts a free drag; stationary panel reflow does not move the object or create a drag; Shift/Ctrl selection does not start a drag; and the movement is one Undo/Redo step.

## F3.3-C04: nudge keys and snap-guide feedback

Additional paired browser check on the same REVIUNG41 fixture: pinned TypeScript `5a472a9426e6e38993361da402cd4ec730feb369` (`http://127.0.0.1:5175/`, session `f33c04-key-nudge-ts-20261003`) and Dioxus build `frontend-module-attachment-repair-20261003` (`http://127.0.0.1:34782/`, session `f33c04-key-nudge-dx-20261003`, source `733c1da2abede39a617d2eca2e42d9bd437cea41`). Package proof is `../../frontend-module-attachment-repair-20261003/package-proof.json`; it reports no source/route mismatches.

- **Arrow nudge:** with the `main-U1` tree row focused, ArrowRight moved X from `273.2` to `273.3` mm in both builds (`+0.1 mm`). Shift+ArrowRight moved it from `273.3` to `274.3` mm (`+1.0 mm`) in both.
- **Editable field and canvas focus:** with the selected part's `X mm` spinbutton focused, ArrowLeft left the part at `274.3` mm on both builds. Focusing the Layout canvas/application and pressing ArrowRight also left it unchanged on both. The mounted nudge route is the focused tree row, not canvas focus; this matches the reference's tested behavior.
- **Snap guide:** moving U1 toward RST produced a live snapped preview at approximately `(273.2, -43.25)` mm. Dioxus exposed `main-RST · corner / midpoint / center` as a live status label. TypeScript rendered its `.wb-snap-guide` circle/path, with no text in that SVG marker. After pointer-up committed the move, the guide element/status disappeared on both builds.
- **Escape cancellation:** repeating the snapped preview, pressing Escape while the pointer was still held restored U1 to `(273.2, -15)` mm and removed the guide on both builds. Releasing the pointer afterward left the restored position unchanged.

Input was delivered through trusted browser mouse and keyboard actions. DOM reads were limited to visible rendered parts/guide nodes; no app runtime state was injected. This verifies the user-facing Escape cancel path on both paired builds. A later candidate-only native touch-cancel probe is reported separately to the root for C03; paired TypeScript/Dioxus `pointercancel` qualification remains unverified.

The tested C04 legs pass for the mounted tree-item nudge route, editable spinbutton behavior, live snap feedback, and guide removal on commit and Escape cancel. A paired native `pointercancel` event check remains outside this receipt.

## F3.3-C03 supplemental: candidate touch-cancel path

On Dioxus build `frontend-module-attachment-repair-20261003` only (`http://127.0.0.1:34782/`, REVIUNG41), a bounded trusted Chrome DevTools Protocol touch-cancel check exercised the candidate cancellation path. With touch emulation enabled, the input sequence was `Input.dispatchTouchEvent(touchStart, 902,244)`, `touchMove(900,300)`, `touchMove(898,312)`, then `touchCancel`. During the snapped preview the status read `main-RST · corner / midpoint / center` and U1 previewed at `(273.2, -43.25)` mm. After `touchCancel`, the guide was absent and U1 returned to `(273.2, -15)` mm. The TypeScript native `pointercancel` path was not tested.

Input was sent through the browser CDP `Input` domain after enabling `Emulation.setTouchEmulationEnabled`; only rendered DOM was read back. No page script or application runtime state was injected.
