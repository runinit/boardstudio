# Keymap parity audit (public paired trace)

Date: 2026-10-02. Scope: Keymap only; Keycaps was assigned separately. This is an exploratory comparison, not a source change, acceptance sign-off, or closure of any parent gate.

## Reproduction identities

- Pinned React reference: `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5175/`.
- Dioxus integration worktree: `89b1de8a28fdf02db91d972c90a69235bfbbbffb`; compared served candidate artifact `frontend-encoder-select-fixed-20261002`, source `868edfcbdf93315e866962c9c57543d26672f379`, at `http://127.0.0.1:34687/`. Keymap/tree source did not differ between the candidate source and integration HEAD. Do not infer that this audit describes later builds.
- Isolated named browser sessions: `parity-keymap-react-20261002` and `parity-keymap-candidate-20261002`, with disk profiles under `/var/tmp/frontend-run/profiles/`.
- Both sides imported through the public import control the same archive, `imported-layered-sofle.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. The profiles are isolated; no user project/profile was changed.
- Read-only IndexedDB observation after settled imports and one public Add macro action per side: both active project IDs `m1-sofle-v2-copy`, names `Sofle v2`, revision 10, layers `Main`/`base` and `Function`/`dc238a57-6d6c-4eaa-b6c2-061d2eedec00`, same two Function-layer bindings. Macro IDs are generated independently. This supports comparison of the imported state but does not claim byte-identical records after those separate macro mutations.

Screenshots, accessible snapshots, DOM observations, downloaded React ZMK archive, and source pointers are retained beside this report. See `ACTION-TRAIL.md` and SHA-256 manifest.

## Observed parity and gaps

| Area | Paired public observation | Classification / existing owner |
|---|---|---|
| Layer/key projection | Both show 2 layers and 29 keys, Main/Function labels/order, the same imported Function bindings, and `left-keys-SW1 · Q` when that key is selected. Both accept real canvas-key selection and list selection. A no-match search retains the previously selected-key editor while the dropdown has no matching item. | Bounded green for this fixture. Does not close F6K.1 or its F3.1 shared-selection join. Evidence: `react-function-layer.txt`, `candidate-function-layer.txt`, `*-function-key-selection.txt`, `*-key-search-no-match.txt`. |
| Main Keymap composition | React shows the `Keymap editors` group with Keys/Macros/Encoders tabs and one selected editor surface. Dioxus shows Layers, Selected key, Encoders, and Macros stacked together; there is no matching Keys/Macros/Encoders navigation group. Copy and control hierarchy also differ (React's layer precedence explanation is absent in the candidate view). | A direct UI placement/hierarchy parity gap under the user's confirmed requirement to preserve TypeScript composition. Candidate already has the child control surfaces, so this is primarily presentation composition/wiring, not an absent Core capability. Existing Keymap panel owner/coordinator must reconcile it; no explicit approved platform exception was found. |
| Board outline in Keymap/tree | React tree includes `Outline` with `Generated`, and the Keymap canvas shows the generated teal board contour. Candidate has no Outline tree item and no board contour; it renders matrix keys only. | Missing presentation/read-model adaptation, with shared tree/canvas ownership. React construction is in `app/src/ui/useWorkbenchTree.ts:135-154`; candidate `web/src/presentation/objects/tree.rs` `TreeKind`/`build_tree` has no outline entry; `web/src/presentation/keymap/view.rs` projects only layers and keys; `web/src/presentation/keymap/canvas.rs` renders key targets only. This is not evidence that Core lacks outline data. It is a visible parity blocker for an exact Keymap demo. |
| Encoder editor | Both show CW/CCW values for the real imported encoder and expose Push only for the reported push input. On empty Main bindings, both show Unassigned/None. Expanding Push exposes the respective editor. | Bounded green for this fixture and candidate build; F6K.4a's actual F5.2 physical-plan join, attached-module rejection, full keyboard/focus, provider output, and F8.2 remain open. Candidate accessibility label says “push button behavior” where React says “push behavior”. Evidence: `*-encoder-main.txt`, `*-encoder-editor.txt`, `react-encoders.png`. |
| Macro editor | Public Add macro on each side produced a `Macro 1` with the same visible defaults (30 ms tap, 0 ms wait, one tap of A). | Smoke parity only; not full edit/remove/validation/reopen coverage in this audit. Dioxus is always stacked rather than opened through the React Macros tab. Candidate step labels include generated IDs where React uses `Macro 1 step 1`. Existing F6K.3 evidence/ticket governs remaining gates; do not close from this smoke. |
| ZMK source export | React's public `Export ZMK source` action downloaded a ZIP. Candidate Keymap has no corresponding action; its generic Export area exposed archive and STEP only. Candidate `web/src` search has no firmware action path. | Missing Dioxus UI/adapter/delivery path, not proof of missing Core firmware capability. Existing F6K.4c Issue 08 owns the bounded implementation; F5.2 electrical/encoder handoff and F8.2 provider/delivery remain explicit joins. Do not add a client-side generator. |
| PCB firmware-position editor | Not exposed by the candidate Keymap workspace surface. | Existing Issue 07 (F6K.4b) is the tracked child. Its separate Wiring/F5 integration is not validated by this Keymap trace. |

Visual geometry is not claimed pixel-identical: the screenshots show different composition and no outline, and the candidate's key cluster appears narrower. The available DOM measurements used different canvas selectors/coordinate frames, so they are not a valid quantitative paired-size assertion.

## Ticket reconciliation and demoable order

- **F6K.1 / Issue 01** is the earliest Keymap projection/selection slice. This fixture gives useful bounded evidence for layers, counts, binding labels, selection, and no-match behavior. It remains open: exact composed panel parity and outline tree/canvas adaptation are visible gaps, and F3.1 shared selection is a required acceptance join. Legacy/default-board and other scope coverage was not performed here.
- **F6K.2 / Issue 03** is the next vertical slice after F6K.1. A selected-key binding editor is present and the broader prior public evidence is recorded elsewhere, but this audit only checked representative imported bindings and selection. Keep the ticket's full behavior, accepted-state/history/save-reopen, and parent gates as written.
- **F6K.3 / Issue 04** has a bounded editor packet already documented in that issue; this audit adds only default Add macro smoke. Firmware-provider output, pending-document race and its stated remaining acceptance criteria stay open.
- **F6K.4a / Issue 06** has a bounded editor packet recorded by its issue, but the present trace proves only visible values and the Push disclosure on this accepted fixture. F5.2 and F8.2 remain joins.
- **F6K.4b / Issue 07** remains the distinct PCB firmware-position editor child; not tested here.
- **F6K.4c / Issue 08** is a real demo blocker if the demo promises the React ZMK-source action. Implement only via the existing firmware-provider path; F5.2/F8.2 remain acceptance joins.

Earliest truthful capability demo using today's UI is a **bounded Keymap data/selection demo** (layer switch, key selection and binding labels) on this archive. It is not an exact-composition parity demo. For the first exact-reference Keymap demo, the smallest coherent visible slice is F6K.1's mounted Keymap composition plus its outline tree/canvas projection and shared selection, followed by representative read-only binding labels. F6K.2 edits, F6K.3 macros, F6K.4a encoder edits, and F6K.4c export can then be demonstrated as separate slices with their own joins; do not make export a prerequisite for the first read-only slice.

## Evidence limits

- No browser instrumentation, provider injection, private runtime API, or storage writes were used. Interactions were public clicks, fills and downloads. IndexedDB reads were readonly.
- Only the imported layered Sofle archive and representative Main/Function states were compared. No legacy/default projection, multiple-board/scope switch, full macro behavior matrix, full encoder edit matrix, module rejection, firmware-ready/provider parity, or full keyboard/focus/AT audit is claimed here.
- RF handoff: no new refactoring takeaway established by this comparison. Preserve the existing RF-009 parity/source-accounting reconciliation; this audit does not resolve it.
- No acceptance threshold or oracle was changed.
