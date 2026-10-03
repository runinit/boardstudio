# F6C.2 per-key legend history and persistence — 2026-10-03

This is the focused owner-valid paired journey for Issue09. It checks the selected key's inherited, explicit-blank, and typed legend operations through the existing Keymap/Keycaps UI and history owner. It does not accept F6C.2 or any broader parent join.

## Provenance and setup

- React reference: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34765/boardstudio/`, source `17ba32b984c13f3621a2145efe1dd478862ab5cd`, provenance `273ca2fa2315b32fec91b0490d2f70fb03b5426d759310bced6e934ad73947d5`.
- Both applications used separate named `agent-browser` sessions and the same retained archive `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`. Embedded project ID `m1-sofle-v2-copy`, name Sofle v2, initial revision 12; active board ID `left` / Left PCB. Key ID `matrix/left-keys/r0c0` / SW1; active Keymap layer ID `base` / Main. Viewport 1280×577.
- The imported fixture had a `keys` matrix profile of SA and SW1's separate saved profile of SA with 7×1 units. Neither profile nor sizing was edited. The archive's SW1 legend began as `null`; its active-layer binding began unassigned.

## Establishing an accepted history baseline

On each app, I selected SW1 in Keymap/Main and chose **Key press**, which set its keycode to A through the normal Keymap editor. React showed SW1/A at revision 13. Undo showed Unassigned at revision 14; Redo restored SW1/A at revision 15. Dioxus likewise showed Unassigned after Undo and SW1/A after Redo, with the browser-local saved indicator set. This proves a valid active document-history action before interpreting legend Undo/Redo.

## Legend history behavior

In each app's Keycaps editor, SW1 remained selected and displayed the inherited editor state (empty value with placeholder A) and rendered A on the physical cap.

- **Blank keycap** stored the explicit empty legend and rendered a dash. React advanced from revision 15 to 16; Undo advanced to 17 and restored rendered A while leaving the binding A intact; Redo advanced to 18 and restored the dash. Dioxus had the same visible and Undo/Redo transitions, and its saved status remained set.
- Typing `Z` and leaving the React legend input committed through its blur behavior at revision 19. Undo at 20 restored the blank dash; Redo at 21 restored Z. Dioxus typing `Z` and leaving the input produced the same visible accepted state; Undo restored blank and Redo restored Z.
- **Use binding legend** restored inheritance (empty input with placeholder A and rendered A). React advanced to revision 22; Undo at 23 restored Z; Redo at 24 restored inheritance. A second paired blank→inherit sequence verified Undo returned to explicit blank and Redo returned to inheritance in both apps.
- The earlier React “History is empty” message did not reproduce. It followed an attempt without an established accepted-history baseline; here both Blank keycap and typed legend edits were directly undone and redone as the most recent accepted operations.

## Persistence and ownership

After reload, both apps returned to the Layout workspace, so I reopened Keycaps and selected SW1 explicitly. The inherited state still rendered A with the empty input and A placeholder. I then saved an explicit blank through **Blank keycap**, reloaded, and verified the dash; after **Use binding legend**, I reloaded again and verified rendered A. React ended at revision 30 after the final inherited edit/reopen. Dioxus reported the browser-local save state as `saved` after each action and reopen. The Dioxus public DOM does not expose a numeric accepted revision counter; its accepted edit/history states are evidenced by real Undo/Redo and saved-state/reopen observations instead.

The Keymap binding remained A throughout the legend-only actions, and the matrix profile remained SA. No error or rejection was reproduced. No document schema, edit owner, history path, UI code, or fixture file was changed.

## Captures

All captures are 1280×577. Hashes are SHA-256.

- React explicit blank: [`react-blank.png`](react-blank.png), `189b73762014e5df9afe6fa3f0a714fa0e1899d9eb587283a995a41cd1769c3a`.
- React Undo blank→inherited: [`react-after-blank-undo.png`](react-after-blank-undo.png), `8f5b7e0a6cebb2f308a779b6a6ce26c5f1dfa44ed5e78daf57e789d708046497`.
- React Redo inherited→blank: [`react-after-blank-redo.png`](react-after-blank-redo.png), `d752b62dd509df74ce506fee7f10978641bba195a5948d7ea402c648bc7fe109`.
- React typed Z: [`react-typed-z.png`](react-typed-z.png), `46a7b1494245b66828efdff7ed32d5f47d1e8359e42ae9dfdb220c7f0169784c`.
- React typed Z Undo: [`react-typed-z-undo.png`](react-typed-z-undo.png), `e0621a158cc8a07fcc1151d5c89c0cac78d5ef0596705695c23b620b630ae592`.
- React Use binding: [`react-use-binding.png`](react-use-binding.png), `5f423e2e112e68afa6672cce09f07e536b97c0db6eefd6ce6d7d505e58b4b019`.
- React Use binding Undo: [`react-use-binding-undo.png`](react-use-binding-undo.png), `2a271b8eb1be064315c2a2f2b69f37f8a482b4ddfe3577c01401a3e4b28c710f`.
- React explicit blank after reload: [`react-blank-reopen.png`](react-blank-reopen.png), `985933be2ec36e36b3345bf3b32c63b4be2c7e570ea6ff0c068e599a8825cfbd`.
- React inherited after reload: [`react-inherited-reopen.png`](react-inherited-reopen.png), `14d059339be99df584b27008e3402dd0ab976c67d0f106a4ee0db1214b778ad3`.
- Dioxus explicit blank: [`dioxus-blank.png`](dioxus-blank.png), `2cb5122c377eb3934a29a5ac509d5960e4195ab2933418b5421a5f53233618f7`.
- Dioxus Undo blank→inherited: [`dioxus-blank-undo.png`](dioxus-blank-undo.png), `bc2ae8b085d07c65ab595cc00db734c54ae83e725c74e47e46c62325406e4a7d`.
- Dioxus Redo inherited→blank: [`dioxus-blank-redo.png`](dioxus-blank-redo.png), `a6f9007be133c187609f8635115f56451966874962675397c8f48d53a5e826cb`.
- Dioxus typed Z: [`dioxus-typed-z.png`](dioxus-typed-z.png), `d56bd762eb603252647f9aad22badfe4776543c809cc86eb59a82d49add51c82`.
- Dioxus typed Z Undo: [`dioxus-typed-z-undo.png`](dioxus-typed-z-undo.png), `3244668df7f89db9b0378740a1ea1633ffbbdba504c416f47be877a47880c443`.
- Dioxus Use binding: [`dioxus-use-binding.png`](dioxus-use-binding.png), `cb1ad3d359f304d580c1be66045649703067ceb7b8a785af522554e6c192accb`.
- Dioxus Use binding Undo: [`dioxus-use-binding-undo.png`](dioxus-use-binding-undo.png), `b62754a88d984b7562d41f45e2983af108aafdcbc628d970a398710256ebf8af`.
- Dioxus explicit blank after reload: [`dioxus-blank-reopen.png`](dioxus-blank-reopen.png), `4a4699aed8332fe53988c87532cf3335c165afa037c17473728bd0bd9381453b`.
- Dioxus inherited after reload: [`dioxus-inherited-reopen.png`](dioxus-inherited-reopen.png), `168d0d4f9ca65f5034b6978b360ccbc6980c92d40d9bdb8850407a6e0d063ebe`.

Issue09 records a focused behavior pass only. Keep F6C.2, F6, F6C.4, INT.2, RF-001, and the rest of the parent acceptance ledger open.
