# F34d perimeter-point paired journey

This is a focused authoring receipt for the perimeter drag/cancel journey. The browser sessions were isolated and named; both imported the same original layered Sofle archive.

- Fixture: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/layered-sofle-export.boardstudio`
- Fixture SHA-256: `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`
- React reference: `http://127.0.0.1:5173/`, pinned source commit `5a472a9426e6e38993361da402cd4ec730feb369`
- Dioxus pre-repair candidate: `http://127.0.0.1:34748/boardstudio/`, source `a76fa2bdee3379be1d9d2c2133f9428a87a75fb0`, provider SHA-256 `7e764747832a5ce20ba2c9091938eeeaee06283583a9071a89d8321c6dc88299`
- Browser sessions: `f34d-reference-20261003` and `f34d-dioxus-20261003`

## Journey and receipt

On the React reference, a trusted mouse drag started at the visible closure handle near `(336,326)` and moved to `(360,326)`. The overlapping first/last polygon coordinates make the visible target point 34; it changed from x `-3.77` to x `4.7625`. One Undo restored `-3.77`, and Redo restored `4.7625`, demonstrating the drag as one history action. `react-live-preview.png` captures the changed point; `react-after-cancel.png` captures the Escape cancellation state. A second trusted drag on point 2 was cancelled with Escape; coordinates remained `(-3.82,-91.048)`. After reload, the tree showed “Edited outline 1” as active and point 34 remained at x `4.7625`; `react-after-reopen.png` records that reopened state.

On the pre-repair Dioxus candidate, the same imported fixture and perimeter editor were used. A trusted fast mouse sequence on point 2 from `(336,417)` through `(360,417)` to release, followed by a 350 ms wait, left its x coordinate at `-3.82` and produced no drag/capture. `dioxus-fast-drag-red.png` records the visible state. This discriminated the delegated `currentTarget` cast defect from the working React journey.

These screenshots preserve only the focused paired reference and pre-repair RED receipt. The post-repair GREEN journey is recorded separately against the newly served candidate so this archive does not imply that the old candidate was repaired in place.
