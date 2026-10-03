# Paired encoder ZMK source output — 2026-10-03

This is one paired public export journey for the existing encoder-editor F6K.4 output criterion. It reuses the retained accepted encoder archive instead of repeating edits, history actions, or reload work.

## Identity and input

- React oracle: pinned TypeScript source `5a472a9426e6e38993361da402cd4ec730feb369`, `http://127.0.0.1:5173/`.
- Dioxus candidate: `frontend-contextual-panes-followup-20261003`, source `9e6f9b62e33d7dc22412cf5793a3f4b857e219c3`, `http://127.0.0.1:34768/boardstudio/`, provenance SHA-256 `0d04905a8dbd66e961595fbae3702124e2f7d2784cfbe5bc3e8dab85dc81d290`.
- Both fresh named browser sessions publicly imported `encoder-public-workflow/candidate-edited.boardstudio`, SHA-256 `887585d5bb011307908727afb47c19bb4f002191fe410e087f91deb0a9f1ccac`. Its prior accepted browser record stores Main/base clockwise `LC(LS(A))`, counterclockwise `Q`, and the reported `left/SW25/push` binding `SPACE`; the prior paired editing/history/archive evidence remains in `encoder-public-workflow/RESULTS.md`.
- Both pages entered Keymap → Encoders and showed `left-SW25`, clockwise `LC(LS(A))`, and counterclockwise `Q`. The retained snapshot/text and pre-export screenshot are under the `react/` and `dioxus/` directories here. Each app's visible `Export ZMK source` control was invoked once; both produced a 15-entry ZIP.

## Output comparison

The ZIP path sets match (15 entries). Fourteen entry contents match byte-for-byte. `config/boards/shields/boardstudio/boardstudio.keymap` is byte-identical in both archives, SHA-256 `d413f17868c21b8a479bfb0158b0c5319af89e15c95cfc74786e34556a0a036e`. It contains sensor bindings `&kp LC(LS(A))` and `&kp Q`, and the pushed key binding `&kp SPACE`.

The sole content-level difference is `electrical-plan.json`: each is 70,333 bytes, with different raw SHA-256 values (React `ddc756a79c915e398c1758f2a075520148e626e512270a46e4625c26554a9556`; Dioxus `1fa4e7a701e35471d8add662e647a283f929d2059ad7e6c88fd00da112d9ac78`), but parsed JSON values are equal. The ZIP file hashes also differ (React `f38e6d625923cdbcd4e28faff234d28b6f6579e1b2332c28beb7ea4ca38100ec`; Dioxus `1a172acaee1bc8ba217bacffddb7e013b55af40a5135341e82daa5242de67905`). The result is semantic archive parity, not byte-identical ZIPs; the raw electrical-plan serialization difference is retained as provider nondeterminism/equivalent serialization rather than hidden or normalized away.

## Exact F6K.4 criterion accounting

The canonical `F6K.4` acceptance in `.scratch/dioxus-frontend-v1/tasks.json` has these relevant clauses:

| Canonical clause | Evidence here / retained evidence | State |
| --- | --- | --- |
| Edit clockwise/counterclockwise actions per stable encoder ID and layer; show push editing only for reported push inputs | Retained `encoder-public-workflow/RESULTS.md` documents visible accepted edits for `left/SW25`, its reported push row, stable `base` layer ID, and accepted record fields. Fresh paired pages on 34768/5173 projected the saved values and exported them. | Covered for the retained supported fixture; F5 real-board handoff remains a separate join. |
| Preserve PCB Wiring-panel quick binding editor and legacy `SetKeyBinding` alongside typed Keymap surface | Not exercised by this Keymap-local export journey. | Open elsewhere; do not imply this packet closes it. |
| Fixture and real F5 data; existing electrical/firmware requests; truthful qualification failures; same package/output | This fixture output is covered: paired Keymap-local ZMK source archive path and semantic contents match. Real F5 data/electrical readiness and failure states were not exercised here. | Fixture source output covered; F5 handoff/readiness/failure joins remain open. |
| Verify rotation and push source output | Paired generated keymap contains both exact rotation expressions and `SPACE` push binding, with identical bytes/hash. | Covered for retained fixture. |
| Undo/Redo, save/reload, compact controls, recovery | Retained Dioxus public workflow in `encoder-public-workflow/RESULTS.md` covers accepted edits, Undo/Redo, exported archive and reimport. This paired output pass intentionally did not repeat them; it used the exact accepted archive produced there. Compact controls and recovery were not exercised by this pass. | Retained Dioxus evidence covers history/reload for the fixture; compact/recovery remain open. |

This is one bounded browser-output proof and is not full F6K.4/F6 acceptance. It does not close F5.2 or the keymap/PCB quick-binding join.
