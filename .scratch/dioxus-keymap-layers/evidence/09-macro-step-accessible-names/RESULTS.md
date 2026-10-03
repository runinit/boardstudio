# F6K.3a macro accessible-name correction

## Source and scope

Implemented the reviewed bounded change in source commit `fdc8529198011ca562f549c36fb00a0072746bd9`, based on integration source `f1d991769cb247b13cfd7a45a63400fd816e5841`. The step-kind control now uses the accepted macro display name plus its one-based position; step-value fields use the visible `Keycode` and `Delay (ms)` labels. Stable macro ID, scope, step index, typed edit requests, values and history operations are unchanged.

Planning review: Sol 6.1 High CLEAR for spec SHA-256 `ac181365d5f25e01ab8ef7839898d9dfc39b671bf6a69d8f556d27b152c21c40` and ticket SHA-256 `44b5bc4a77f9e60eb91a0391fa6e43e2ffadd6531b57c955ae184a99ec607005`, with provenance receipt SHA `70de341ace3838132de666c8f094ccef7758c87c787517a78b4f9661fa789976`. The parent F6K.3 and its acceptance joins remain open.

## Red/green evidence

The retained pre-fix Dioxus snapshot contains `keymap-macro-11-131081 step 1` and `keymap-macro-11-131081 step 1 keycode`, while React's paired snapshot says `Macro 1 step 1` and `Keycode`. A mounted WASM regression run against a disposable detached copy of source commit `fdc8529198011ca562f549c36fb00a0072746bd9` with only the three production aria-label expressions restored to their pre-fix ID-based forms failed at the expected first control-name assertion: `select[aria-label='Macro 1 step 1']` was absent. The exact raw run is compressed in `expected-red-mounted-test.raw.log.gz` (raw SHA-256 `c4c6d2307a398617ea0c3a504bd644ca9f26f73ef55afd23c0c4447d8d3e30ac`); `expected-red-mounted-test.log` is its whitespace-normalized readable copy. The disposable checkout was not committed and the frozen source remains unchanged. After the fix, fresh paired browser sessions imported the same Sofle archive (SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`) and exercised the existing editor:

- Both showed default `Macro 1 step 1`, tap-A, 30 ms tap, and 0 ms between-actions wait.
- After accepting a rename to `Kitchen macro`, both showed `Kitchen macro step 1`.
- After adding a step, both showed `Kitchen macro step 2`; switching it to wait showed `Delay (ms)` at 100 ms, and switching back to press showed `Keycode` at A.
- Removing the added step returned both to a single `Kitchen macro step 1`. No generated macro ID appeared in the candidate's step-control accessible names.

React source oracle was pinned commit `5a472a9426e6e38993361da402cd4ec730feb369`. Dioxus browser candidate was release build `keymap-macro-labels-fdc85291`, source commit above, served at root port 34735 and subpath port 34736. The full build reused provider/static inputs from `frontend-keycaps-integrated-20261002` source `bd671ae8db8897388d26ebf973380c69efb9bffd`; exact provenance is recorded in `build-provenance.json` (SHA-256 `ae68835ebfd26ce47547e11cd268181743a20a6d48a15ca0329c6dd6a9f3d6c4`). All 77 checked Core, application, CAD, renderer and worker source inputs match that base. Both routes returned the expected page, JS, wasm, service-worker and offline-worker assets; route and provider-input checks are in `route-smoke.log`.

The feature-level native label tests passed (2/2). The mounted production MacroEditor WASM test passed in headless Chrome (1/1). Strict page WASM Clippy, `cargo fmt --check`, and `git diff --check` passed. Logs are stored beside this report. The release build reports two pre-existing unused-variable warnings in `keymap/encoder_editor.rs`; the strict WASM Clippy check is clean.

## Evidence files

- `expected-red-dioxus-macro.snapshot.txt`, `react-final.snapshot.txt`, `dioxus-final.snapshot.txt`, `react-wait.snapshot.txt`, `dioxus-wait.snapshot.txt`, `react-press.snapshot.txt`, and `dioxus-press.snapshot.txt`
- `react-macro-final.png` and `dioxus-macro-final.png`
- `native-tests.log`, `wasm-mounted-test.log`, `expected-red-mounted-test.log`, `expected-red-mounted-test.raw.log.gz`, `wasm-clippy.log`, `page-root-build.log`, `page-subpath-build.log`, and `route-smoke.log`
- `build-provenance.json` (SHA-256 `ae68835ebfd26ce47547e11cd268181743a20a6d48a15ca0329c6dd6a9f3d6c4`)
