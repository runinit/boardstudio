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

## C05 public readiness failure and recovery — 2026-10-03

Bounded check against candidate root `http://127.0.0.1:34782/`, build `frontend-module-attachment-repair-20261003`, source `733c1da2abede39a617d2eca2e42d9bd437cea41`. Used the same public-import Sofle archive and SHA-256 recorded above (`5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`); no network interception or private runtime injection.

On the public PCB Wiring UI for Left PCB, changed `Pin for left/SW25/encoder-a` to `Unresolved`. The UI surfaced `left/SW25/encoder-a requires ; review the pin assignment before changing it`; the actual Keymap `Export ZMK source` request surfaced `No resolved GPIO for encoder left/SW25`. Capture: `c05-dioxus-export-failure-34782.png` and `.json`.

For one recovery attempt, restored the fixture's original encoder-A pin `P1` through the same visible selector and applied wiring. The UI showed `Wiring pin saved`; the subsequent public ZMK export downloaded `c05-dioxus-after-correction-34782.zip`. Its `electrical-plan.json` has Left PCB `peripheralTerminals` encoder-a `P1`, encoder-b `P0`, push `P2`, matching the imported fixture's retained positive F5 plan; the archive also includes the ZMK keymap and shield outputs. This establishes recovery at source-export/readiness generation; no local ZMK firmware build was run. The post-recovery page capture is `c05-dioxus-after-correction-34782.png` and `.json`.

Evidence is limited to this one Left PCB encoder-A failure/correction path; it does not claim all readiness failures or firmware compilation are qualified.

## C07 narrow encoder control operability — 2026-10-03

Reopened the same imported Sofle fixture on candidate `http://127.0.0.1:34782/` in an isolated public browser session and set the viewport to 430×900. In Keymap → Encoders → `left-SW25`, the CW, CCW, and Push button controls were reachable and editable without widening the viewport. Public controls were set to Key press with CW `F5`, CCW `Q`, and Push `SPACE`. CW and CCW each produced visible `Binding saved.` feedback; after blurring the Push input, the UI reflected `SPACE` with Key press. The immediate public DOM read recorded all three values at 430×900.

The subsequent public ZMK export download action did not complete in the browser session. A bounded same-handle poll stalled; there is no C07 ZIP at the requested path or in the default Downloads directory, so this leg does not claim accepted export/reopen confirmation for the three narrow-width edits. C05's full-size failure/recovery export remains separate evidence and is not counted as C07 export proof. Existing Undo/Redo and reimport evidence above remains the accepted-history/archive evidence for the editor generally.

Module-recovery remains unavailable in the same Sofle fixture: its Keymap panel exposes the actual encoder controls, including Push, but there are no attached-module rows to remove/recover. No module-specific recovery behavior is claimed. The browser session was closed after the bounded poll.

### C07 criterion reconciliation

C07 asks for Undo/Redo, save/reload, compact controls and recovery. The retained integrated real-plan review proves history/reload; the narrow-width journey above proves actual CW/CCW saves and reachability/editability of Push; C05 proves a genuine rejected request followed by successful correction/export, and earlier invalid-expression evidence proves editor recovery. These are complementary legs, not an additional requirement to repeat every downstream export/history operation at each viewport. C07 is therefore verified from the combined evidence. The stalled compact download remains an unqualified observation, not a claimed successful export or diagnosed application failure. No attached-module recovery claim is added; distinct module/provider lifecycle requirements remain with their owning criteria. F5.2 and F8.2 final parent joins remain open.
