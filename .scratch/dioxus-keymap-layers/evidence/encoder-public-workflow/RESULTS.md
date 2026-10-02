# Public encoder editor QA — initial candidate e82c039b

This is a bounded public-browser workflow against `http://127.0.0.1:34685/`, served as root route, build `frontend-encoder-bindings-20261002`, source `e82c039b5486d934de96931238e65622ad0ffb1d`. Build provenance summary is adjacent. Browser sessions use isolated disk-backed profiles:

- `encoder-public-workflow-e82c039b` at `/var/tmp/frontend-run/profiles/encoder-public-workflow-e82c039b`
- `encoder-public-reload-e82c039b` at `/var/tmp/frontend-run/profiles/encoder-public-reload-e82c039b`

The input was a public import of the real archive `../binding-editor-reference/imported-layered-sofle.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. Before edits, the accepted IndexedDB project was `m1-sofle-v2-copy`, revision 9, with Main layer id `base`, Function layer id `dc238a57-6d6c-4eaa-b6c2-061d2eedec00`, and `base.sensors={}` / `base.bindings={}`. `candidate-baseline-project.json` is a read-only snapshot.

## Findings

**Expected-red: untouched None controls render as Key press.** With no selected key and an empty accepted Main sensor map, the visible DOM `<select>` values for clockwise, counterclockwise, and the public Core-reported push control were all `key-press`, not `none`. This is confirmed by `candidate-baseline-read.txt`, `encoder-control-baseline.json`, and `candidate-baseline.png`; it is not inferred from accessibility text alone. Choosing `Unassigned` normalized a control to `none` without changing the accepted document; the initial editor default therefore diverges from the saved empty/None state. Root independently reproduced this and assigned the shared editor correction. Do not count initial default parity as green on this build.

**Real edit, validation, history, and archive round trip.** Through visible editor controls on left encoder `left/SW25`, Main/base:

- clockwise was changed to `LC(LS(A))`
- counterclockwise was changed to `Q`
- the visible Core-reported Push button row was changed to `SPACE`, accepted under binding id `left/SW25/push`

The final accepted revision 16 snapshot is `recovery-record.json`; exact serialized values are in `base.sensors["left/SW25"]` and `base.bindings["left/SW25/push"]`. Function bindings remained unchanged. `A)` produced `aria-invalid=true`, visible `Invalid keycode expression A)`, and did not alter accepted revision 15 or the prior F5 value (`invalid-expression-dom.json`, `invalid-expression-record.json`). Correcting the same input to `LC(LS(A))` saved revision 16 and cleared the invalid state (`recovery-dom.json`).

Public Undo restored clockwise F5 at revision 17; public Redo restored `LC(LS(A))` at revision 18. Counterclockwise Q and push SPACE remained unchanged. Read-only records are `undo-record.json` and `redo-record.json`; the contemporaneous DOM input state is `undo-dom.json`.

The actual public Export archive download is `candidate-edited.boardstudio`, SHA-256 `887585d5bb011307908727afb47c19bb4f002191fe410e087f91deb0a9f1ccac`. A second isolated profile publicly imported that archive; its read-only IndexedDB snapshot `reimport-project.json` preserves the accepted encoder values, push id, stable layer ids, and revision 18. `reimport-keymap.png` shows the reloaded editor with the saved rotations and no selected key. This establishes public export/import round-trip only; firmware/compiler output is outside this check.

## Harness notes and limits

Initial editor screenshot shows the accepted document before interaction. Subsequent saved edits used actual editor controls and blur to commit the focused text entry; keycode values use Core spelling (`SPACE`), not display spelling (`Space`). Earlier exploratory attempts using capitalized `Space` produced a visible validation error and were not counted as accepted behavior. Some consecutive control actions happened while the editor showed `Saving binding…`; snapshots are labeled by accepted IndexedDB revision rather than assuming every immediate DOM value committed.

The actual Core push row was visible and expanded in the public UI; the accepted archive gained only its reported `left/SW25/push` id. No key was selected in the layout. The archive has no module rows, so module-specific behavior was unavailable. This packet does not exercise an F5 accepted physical plan/fingerprint or F8 firmware output; both integration gates remain open. It does not claim reference parity or full encoder acceptance while the untouched-None default remains red.

## Corrected default build retest

The follow-up candidate is root URL `http://127.0.0.1:34687/`, build `frontend-encoder-select-fixed-20261002`, source `868edfcbdf93315e866962c9c57543d26672f379`. A new disk-backed profile/session (`encoder-public-workflow-868edfcb`) publicly imported the original SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df` archive. Before any edit, the active accepted Main/base record remained revision 9 with `sensors={}` and `bindings={}`; the DOM values for clockwise, counterclockwise, and push behavior were each `none` / “Unassigned”, with no selected key. See `fixed-default-dom.json` and `fixed-default-record.json`. The corrected untouched-None default is green against the initial red above.

On this fresh candidate, public Key press selections and real keyboard edits produced clockwise F5, counterclockwise Q, and the visible Core-reported left/SW25 push SPACE. The accepted record `fixed-edited-record.json` (revision 15) contains only the expected encoder values under `base.sensors["left/SW25"]` and push under `base.bindings["left/SW25/push"]`. One public Undo removed the final push edit (revision 16); Redo restored it (revision 17). Exact snapshots: `fixed-undo-record.json`, `fixed-redo-record.json`.

A separate fresh profile publicly reimported the prior accepted export archive SHA-256 `887585d5bb011307908727afb47c19bb4f002191fe410e087f91deb0a9f1ccac` using the corrected candidate. The resulting DOM and accepted record (`fixed-reimport-dom.json`, `fixed-reimport-record.json`) retain clockwise `LC(LS(A))`, counterclockwise Q, push SPACE, stable layer IDs, and no selected key. This verifies the corrected behavior default does not overwrite existing accepted bindings on archive import. `fixed-reimport-encoder-visible.png` is the actual browser capture with encoder details in view.

The earlier candidate’s detailed validation/recovery and full Undo/Redo/export/reimport checks are preserved above for source e82c039b. The correction is restricted to initial option selection; it does not change the binding editing, parsing, persistence, history, or archive logic. New-build coverage deliberately repeats the regression, actual edits, Undo/Redo, and nonempty accepted archive import, rather than claiming all earlier scenarios were independently rerun. No browser error was used to infer behavior.

Open coverage for this bounded packet: no board or physical-instance switch was exercised; this imported Sofle has only left/`left half` selected in the tested screen. No module rows exist in the fixture. A keyboard-only navigation sequence, actual assistive technology, F5 physical plan/fingerprint, and F8 firmware output were not exercised. The saved editor displays CW/CCW expanded and Push collapsed after reimport (`fixed-reimport-dom.json`); screenshot `fixed-reimport-all-controls.png` was captured at 1280×1200 with no selected key. The initial default screenshot `fixed-default.png` was captured at 1280×900.
