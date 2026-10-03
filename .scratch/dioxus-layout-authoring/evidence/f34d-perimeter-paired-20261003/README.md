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

## Negative half-grid policy follow-up

Review also found a policy mismatch at negative half-grid coordinates: React `Math.round(-0.5)` produces negative zero and `Math.round(-1.5)` produces `-1`, while Rust `f64::round()` produces `-1` and `-2`. The focused production-source regression `outline_grid_rounding_tests::tests::negative_half_grid_matches_javascript_math_round` first failed on the uncorrected behavior (`-0.5` snapped to `-1.0`) and passed after the grid rounding port used the JavaScript tie rule. The shared `presentation/outline_grid_rounding.rs` helper is called by the WASM page snap policy and compiled directly by the native focused regression, without widening API visibility or suppressing lints. Final focused test: 1 passed; `cargo fmt -- --check` and `git diff --check` passed.

The integration places the unchanged native test adapter in the existing binary presentation stand-in. This preserves the provider library root byte for byte, allowing the packaging guard to prove page-only ownership and reuse the verified full providers. The production helper and regression assertions remain unchanged.

## Post-repair Dioxus journey

Fresh frozen page candidate: `http://127.0.0.1:34749/boardstudio/`, source `dd7697ed99864040697c905b3d0e28209301d11b`, provider SHA-256 `1f3584f7b41b9c2d912381a302657ccf202abc7ebcbf04622ef68bf62c0f932e`. Session: `f34d-dioxus-final-20261003`. This build contains the pointer/capture/final-sample repair (`f9aadf64`); it predates the separate negative half-grid source follow-up (`57e94966` and test-seam cleanup `e3eb2633`).

The same fixture was imported. A trusted fast mouse sequence on point 2 at `(336,417)` moved to `(360,417)` and released immediately, then waited 500 ms. Point 2 changed from `(-3.82,-91.048)` to `(4.7625,-91.048)`, demonstrating that delegated target capture began and the fast final sample committed. `dioxus-after-fast-commit-green.png` records the changed point. One Undo restored x `-3.82`; Redo restored x `4.7625`.

An out-and-back gesture moved point 2 24 screen pixels out, returned it to its original screen position, and released; the durable field remained `4.7625`. A second drag moved outward and pressed Escape while the circle was focused; after pointer release the durable field remained `4.7625`. The next single Undo still restored the original `-3.82`, and Redo restored `4.7625`, so neither cancellation added history. `dioxus-live-preview-green.png` shows the in-progress preview; `dioxus-after-escape-cancel-green.png` shows the canceled state.

Finally the page was reloaded in the same isolated browser profile. The CAD tree showed “Edited outline 1” active; reopening point editing showed point 2 persisted at `(4.7625,-91.048)`. `dioxus-after-reopen-green.png` captures the reopened edited outline. No broader neighboring workflow was exercised.
