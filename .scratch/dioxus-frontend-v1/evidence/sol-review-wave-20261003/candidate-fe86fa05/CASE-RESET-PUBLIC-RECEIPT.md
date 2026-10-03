# Reset gasket placement — public reference receipt

Reference app: pinned React 5173, separate `case-reset-react-reference-20261003` browser profile. Input was the existing accepted post-redo archive from Issue 14, `../14-case-contextual-gasket-support-unlink/react-configured-after-redo.boardstudio` (SHA-256 `df38eec77d8cbd2953a8a01449b41819ec8c79638d190fd14dfc7f47e7f6de38`). The original 5b fixture and Issue 14 evidence were not modified.

In Case, I selected the Gaskets tree row and clicked **Reset gasket placement**. Before the action the archive had one manually positioned support override (`left-keys-layout:0`, length 70 mm, width 3 mm, `unlinked: true`). The saved archive after the action has an empty `gasketLayout.supports` array. Other gasket parameters remain `autoSize: true`, length 80 mm, width 3 mm, thickness 2 mm, and compression 0.15. The UI continues to show 17 supports per region because reset clears saved placement overrides and returns placement to generated defaults; the hint says “Reset releases manually positioned supports. Closure positions stay fixed.”

Undo restores that saved 70 × 3 mm unlinked override; Redo clears it again. Captures:

- Before: `react-before-reset.png` (SHA-256 `7479e4fde0d2cb8de0bfd08df8da95fffa57f6caf68a927e459f69b8e4f877f1`); `react-before-reset.snapshot.txt`.
- After: `react-after-reset.png` (SHA-256 `18dc2cfbd02dea352031efbd13c57a8c69c8a5558144894cad0d4248348018fd`); `react-after-reset.snapshot.txt`; saved archive `react-after-reset.boardstudio` (SHA-256 `f7404148d8aa14b11f01816b3e47a76d01b6a455798de8b06e7f84d820a50da9`).
- History: `react-after-undo.snapshot.txt` and `react-after-redo.snapshot.txt`.

## Dioxus candidate journey

Candidate `fe86fa05598dee6ef8ffb02f9f6fcfaa6810f10d`, served at `http://127.0.0.1:34750/boardstudio/` with provenance SHA-256 `3ee7a6c10ba260d41233d5fc1cd1b7f1f518b0bc6b6f029bacae553a59f74740`. In a fresh named browser profile, I imported the same accepted Issue 14 post-redo archive, selected Case → Gaskets, and clicked **Reset gasket placement**. Before Reset the saved support override was `left-keys-layout:0` (70 × 3 mm, unlinked). The saved Dioxus archive after Reset has `gasketLayout.supports: []`; `autoSize`, length 80 mm, width 3 mm, thickness 2 mm, and compression 0.15 remain unchanged. The generated default remains visible as 17 pairs / 40 resolved stack layers. The mechanical diagnostics count was 71 before and after this action.

Undo restored the exact saved override (including its ID, dimensions, and `unlinked: true`); Redo cleared it again. I then imported the Redo archive into a second fresh browser profile. Reopen showed the Case view with the same empty saved supports, generated 17 pairs, and the Reset hint. This bounded journey confirms current save, Undo/Redo, and portable archive import for the reset action; it does not claim broader gasket geometry acceptance.

- Before screenshot/snapshot: `dioxus-before-reset.png` (SHA-256 `68304fa9a038335fdaeab8e174078a5f4ae1d1e1cbbb61683873378450301cea`); `dioxus-before-reset.snapshot.txt`.
- After screenshot/snapshot: `dioxus-after-reset.png` (SHA-256 `838c90b7aca2ec3b315d4a21a63b45478060f950e513ecb5217278e0de49eb4a`); `dioxus-after-reset.snapshot.txt`; portable archive `dioxus-after-reset.boardstudio` (SHA-256 `6a30c7c85a94d856f76c1e5417c7ae909c1f0ccf50307170b55f99596b2abc1d`).
- Undo portable archive: `dioxus-after-undo.boardstudio` (SHA-256 `d151daa6dcc8ebdbd2d81a5d7f03278185daa681e107e979e9c2b54161fd37c4`).
- Redo portable archive: `dioxus-after-redo.boardstudio` (SHA-256 `62c58fa94dcd623c7c6159a53b7f42c2e87c4b4081dad35cedbe239dcd7e68ab`).
- Reopened screenshot/snapshot: `dioxus-reopened.png` (same SHA-256 as after screenshot); `dioxus-reopened.snapshot.txt`.
