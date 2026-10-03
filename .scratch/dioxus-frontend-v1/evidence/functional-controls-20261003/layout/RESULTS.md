# F3.3-C02: drag admission and stationary reflow

Paired browser qualification: pinned TypeScript `5a472a9426e6e38993361da402cd4ec730feb369` (`http://127.0.0.1:5175/`, session `f33c02-layout-qual-2318ea1a1376`) and Dioxus build `frontend-functional-controls-20261003` (`http://127.0.0.1:34778/`, session `bs-mig-6280ea4d22a7`, source `ad26b07a7ada49aed1d7ac686fb32a8dbcf2ded6`). Dioxus package proof is `../../frontend-functional-controls-20261003/package-proof.json`; it reports no source/route mismatches.

## Results

- **Alt + 1 CSS px part drag:** U1 moved freely by about 0.43 mm on each surface (world X: Dioxus `273.200012 → 273.630004`; TypeScript `273.20 → 273.626727`). This confirms nonzero client motion admits a drag despite the small world-space delta. One Undo restored the original position and one Redo restored the moved position on both surfaces.
- **Stationary press during Inspector reflow:** with the pointer held at a fixed client coordinate, keyboard activation expanded the Inspector and changed the canvas width from 1013 px to 725 px. The pressed part/key and U1 retained their world positions on both surfaces; the stationary release did not create another history change. This isolates client-coordinate motion from world-coordinate changes caused by panel reflow.
- **Shift/Ctrl selection:** trusted Shift and Ctrl selection sequences with 8 px pointer travel left target part positions unchanged on both surfaces. Selection occurred without a drag or position history change.

Input used trusted browser mouse and keyboard events. No app runtime state was injected. The Inspector was toggled by keyboard while the pointer was held, so this verifies stationary reflow and release behavior, not a selection gesture that itself opens the Inspector.

## Conclusion

F3.3-C02 passes for the tested candidate: 1 CSS px Alt movement starts a free drag; stationary panel reflow does not move the object or create a drag; Shift/Ctrl selection does not start a drag; and the movement is one Undo/Redo step.
