# F3.3b selected Column and Key transform journey

This focused public journey compares the pinned React Layout with the integrated Dioxus candidate on the same layered Sofle archive. It covers one Column offset and one Key-local edit, their Undo/Redo entries, and persistence after reload. It does not close F3.3b, F3.3, F3.7, or the wider 62-parent graph.

## Reproducible inputs

- React source pin: `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5173/`.
- Dioxus candidate before the focused composition repair: `37d81320712b16cd6901f8e8ea3c80a0833ce576`, served at `http://127.0.0.1:34758/boardstudio/`.
- Candidate build: `frontend-layout-matrix-20261003`; provider provenance SHA-256 `540bccd3ef9eab93cdc521191acd918d3ca1b605ec0969ba6a139ec517fa391f`.
- Fixture: `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Independent browser sessions: `f33b-react-20261003` and `f33b-dioxus-20261003`.

## Journey and result

On Left PCB, both applications selected `keys · Column 1` and changed Offset X from `0` to `2.0 mm`. One Undo returned it to `0`; Redo restored `2.0`. Both then selected `Key 1.1 left-keys-SW1` and changed Local X from `0` to `0.5 mm`. One Undo returned it to `0`; Redo restored `0.5`. After page reload, the Column still showed Offset X `2.0` and the Key showed Local X `0.5` in both applications. Dioxus' selected Key world position readout was X `11.68 mm`, reflecting the Column offset plus the local Key offset.

The changed Column and Key-local values, undo, redo and reload behavior matched. The candidate exposed one additional control on Key selection: a generic world Position X/Y form with Apply position above the Key-local fields. The pinned React Key inspector has no world Position form. The F3.3b ticket defines Key context edits as matrix-local X/Y and rotation, while the standalone-component position path remains separate. This is the focused RED receipt for a private composition correction; the correction is included in this branch but still needs a rebuilt candidate and one changed Key-inspector check before it can be marked GREEN.

The isolated correction limits the generic Position inspector to a selected standalone Component context; the Key context continues to use its matrix-local fields. It does not change standalone component positioning or the matrix transform operation owner.

## Evidence

- [React Column after reload](screenshots/react-column-reopened.png)
- [Dioxus Column after reload](screenshots/dioxus-column-reopened.png)
- [React Key after reload](screenshots/react-key-reopened.png)
- [Dioxus Key after reload, showing the extra Position form](screenshots/dioxus-key-reopened-with-position-mismatch.png)

## Limits

The journey uses one independent matrix and does not exercise linked/mirrored layouts, empty cells, Splay/origin policies, matrix-level orientation, draft conflicts, stale callback races, or invalid input. No new broad suite or compiler run was performed for this packet. The root-owned combined affected page check remains the source qualification gate.
