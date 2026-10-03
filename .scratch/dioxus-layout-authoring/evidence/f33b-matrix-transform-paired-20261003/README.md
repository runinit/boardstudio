# F3.3b selected Column and Key transform journey

This focused public journey compares the pinned React Layout with the integrated Dioxus candidate on the same layered Sofle archive. It covers one Column offset and one Key-local edit, their Undo/Redo entries, and persistence after reload. It does not close F3.3b, F3.3, F3.7, or the wider 62-parent graph.

## Reproducible inputs

- React source pin: `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5173/`.
- Dioxus candidate before the focused composition repair: `37d81320712b16cd6901f8e8ea3c80a0833ce576`, served at `http://127.0.0.1:34758/boardstudio/`.
- Candidate provider provenance SHA-256 `540bccd3ef9eab93cdc521191acd918d3ca1b605ec0969ba6a139ec517fa391f`.
- Rebuilt composition-fix candidate: source `04c85b88eae2894b416979f785a1f0060d2eb27d`, `http://127.0.0.1:34759/boardstudio/`, provider provenance SHA-256 `81ee87aead500473ae22f79834a2bc921618442a081719860a08188c063e49aa`.
- Fixture: `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Independent browser sessions: `f33b-react-20261003` and `f33b-dioxus-20261003`.

## Journey and result

On Left PCB, both applications selected `keys · Column 1` and changed Offset X from `0` to `2.0 mm`. One Undo returned it to `0`; Redo restored `2.0`. Both then selected `Key 1.1 left-keys-SW1` and changed Local X from `0` to `0.5 mm`. One Undo returned it to `0`; Redo restored `0.5`. After page reload, the Column still showed Offset X `2.0` and the Key showed Local X `0.5` in both applications. Dioxus' selected Key world position readout was X `11.68 mm`, reflecting the Column offset plus the local Key offset.

The changed Column and Key-local values, undo, redo and reload behavior matched. The first candidate exposed one additional control on Key selection: a generic world Position X/Y form with Apply position above the Key-local fields. The pinned React Key inspector has no world Position form. The F3.3b ticket defines Key context edits as matrix-local X/Y and rotation, while the standalone-component position path remains separate. This is the focused RED receipt for the private composition correction.

The isolated correction limits the generic Position inspector to a selected standalone Component context; the Key context continues to use its matrix-local fields. The rebuilt candidate removed the world Position X/Y/Apply section for Key 1.1 while retaining Local X/Y, rotation and reset. Selecting standalone `left-U1` still showed Position X/Y in both React and Dioxus. The changed branches were checked on the same imported fixture; both browser error buffers were empty. This verifies the specific composition delta, not the remaining F3.3b fields/history/stale-owner matrix. The correction does not change standalone component positioning or the matrix transform operation owner.

## Evidence

- [React Column after reload](screenshots/react-column-reopened.png)
- [Dioxus Column after reload](screenshots/dioxus-column-reopened.png)
- [React Key after reload](screenshots/react-key-reopened.png)
- [Dioxus Key after reload, showing the extra Position form](screenshots/dioxus-key-reopened-with-position-mismatch.png)
- [React Key after composition correction](screenshots/react-key-composition-green.png)
- [Dioxus Key after composition correction](screenshots/dioxus-key-composition-green.png)
- [React standalone Component Position](screenshots/react-component-position-retained.png)
- [Dioxus standalone Component Position](screenshots/dioxus-component-position-retained.png)

## Limits

The original field journey uses one independent matrix and does not exercise linked/mirrored layouts, empty cells, Splay/origin policies, matrix-level orientation, draft conflicts, stale callback races, or invalid input. Root's combined affected page strict check passed after the composition fix; no new build/test was run by this leaf. These receipts do not close F3.3b or the F3.3/F3.7 parent joins.
