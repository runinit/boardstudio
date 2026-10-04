# Macro accessible names and ZMK export — root-package public qualification

This paired browser run qualifies Macro09 and the F8 Export-route ZMK row against the verified root package. It is separate from the isolated page-package proof in `../09-macro-step-accessible-names/RESULTS.md` and does not close F6K.3, F8.4, F8.6, or their acceptance joins.

## Identity and fixture

React oracle: pinned source `5a472a9426e6e38993361da402cd4ec730feb369`, served from `http://127.0.0.1:5173/`. Dioxus candidate: root package source `7d09d0a60fbb2cc12e259541614b9483ee618d29`, build `frontend-authoring-layers-integrated-20261002`, served from `http://127.0.0.1:34735/` and `/boardstudio/`. Build provenance SHA-256: `aabc367a69f40dce0601226a7909a21f71950824534137a2d41a7253d66b7a66`.

Fresh isolated sessions used `macro-zmk-react-exact-20261002` and `macro-zmk-dioxus-20261002`, each importing the same retained Sofle v2 archive (SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`). Both browsers were Headless Chrome 154, viewport 1280×577 CSS px, device scale factor 1. Screenshots use the System theme, which resolved to the light palette.

## Paired macro behavior

In each session the action sequence was: open Keymap → Macros, add one macro, rename it to `Kitchen macro`, add a second step, and change that step to `wait` (100 ms). Both pages exposed the same accepted accessible names and values: `Macro name Kitchen macro`, `Kitchen macro step 1` (tap, keycode A), `Kitchen macro step 2` (wait, delay 100 ms), `Keycode`, and `Delay (ms)`. After a full page reload, the accepted name and step remained in both profiles. Dioxus showed revision 13 and Saved after reload.

The paired snapshots and screenshots are under `react-exact/` and `dioxus/`. The exploratory `react/` folder is from an earlier interaction pass that accidentally submitted Add macro twice before waiting for the UI to settle; it is not used in the paired comparison below.

## Export row, downloads, and package contents

The React Export workspace presents a ready `ZMK firmware` row with description `ZMK v0.3.0 configuration and editable starter keymap` and an enabled `Export ZMK firmware` button. Dioxus presents the same row description and accessible button name, also enabled. In both applications the row export downloaded a real 15-entry ZIP after the macro edit survived reload. Each application’s Keymap-local `Export ZMK source` download was byte-identical to its own Export-route ZIP.

The React and Dioxus ZIP entry path sets are equal (15 entries). All entry contents match except the raw `electrical-plan.json` serialization. The parsed electrical plans are equal, including central and peripheral revision 13. `boardstudio.keymap` is byte-identical (SHA-256 `0c90b67a3775e68e50875d68846381691a94bfade05da3ff09168f398696aa9c`) and includes the macro tap-A followed by a 100 ms wait. The two overall ZIP hashes differ because of the raw plan serialization; this is semantic archive parity, not byte-for-byte ZIP parity. Exact package and entry hashes are in `comparison.json`; downloaded ZIPs and extracted files are retained under each profile directory.

One visual mismatch remains in the ready indicator. React’s `.wb-ready-dot.is-ready` computes to `rgb(226, 242, 233)` in Light and `rgb(24, 53, 45)` in Dark, matching `--wb-status-success-surface`. Dioxus currently uses its accent token, computing to `rgb(56, 88, 214)` in Light and `rgb(154, 173, 255)` in Dark. Both buttons are enabled and both exports succeed. The code-level correction is isolated to using the already-defined `--wb-status-success-surface` token for `.m1-export-ready-dot.is-ready`; visual green evidence must be repeated against a rebuilt root package before claiming the correction.

The row read text, interactive snapshots, ready-row screenshots, macro snapshots, downloads, extracted ZIP contents, build identity, and hashes are retained beside this report. The not-ready/error browser state was not exercised in this successful-ready journey.

## Unchanged macro keycode blur and one accepted edit

On 2026-10-04, I compared the pinned React app at `http://127.0.0.1:5175/` (TS source `5a472a94`) with the Dioxus root package at `http://127.0.0.1:34801/` (source `9eda1b5d4c194a97b7216a43544036b5709a65f8`). Both isolated Headless Chrome sessions used a 1280×577 CSS-pixel viewport at device scale 1 and imported the same saved archive, `../public-binding-editor/reference-layered-bindings.boardstudio` (SHA-256 `c9aa6a4e387fc206fd3a4a0e944b140f095ba8774a8ab5bdc93a99d74e19c9ea`). The imported project was `m1-sofle-v2-copy`, revision 29, with `Reference Macro` step 1 Tap A and step 2 Press B.

I focused the first step’s unchanged `A` keycode and pressed Tab to blur it in each app. Visible values remained A/B. A public `.boardstudio` export after the blur remained at revision 29 in both apps; each app’s before/after `project.json` SHA-256 was unchanged (React `d1fcbf4f4854ac16bd05172670c48ff4e550906e4fec386efb55a770d71d12a3`; Dioxus `96c03bb7a7408288bea1a776d2a89f9df95a1b1010450131b61d6791c0cc38c2`). The mounted regression `mounted_accessible_name_tests::unchanged_valid_step_keycode_blur_does_not_submit_a_macro_edit` confirms Dioxus emits no edit request for this unchanged valid keycode; its retained RED failed on the old unconditional commit, and GREEN passed after the fix. The same implementation still commits unchanged fields when normalizing an unsupported legacy binding.

I then changed step 1 from A to C and blurred it once in each app. Both accepted C/B, and public exports advanced exactly one revision, 29 → 30, with the same stable macro ID and step kinds. This confirms the no-op blur creates no extra accepted edit while a changed value still follows the existing edit path. Browser error buffers were empty in both sessions; both isolated sessions were closed after the journey.
