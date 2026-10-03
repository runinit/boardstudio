# F3.2d matrix configuration reference pin

Pinned React source: `5a472a9426e6e38993361da402cd4ec730feb369`, served from the public React route at `http://127.0.0.1:5173/`.

Fixture: layered Sofle archive `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

Using a named isolated browser session (`issue04-reference-eeb225b34b3e`), I imported the fixture, selected the `keys` matrix in the Left PCB tree, and expanded the matrix inspector's Key assembly, Keycap spacing, and Matrix actions sections. The mounted controls were:

- Layout name, rows, columns, pitch X/Y, and position/orientation controls.
- Assembly preset (`MX Solder`, `MX Hotswap`, `Choc V1 Solder`, `Choc V1 Hotswap`, `MX RGB`, `Choc V1 RGB`, `MX Hotswap RGB`, `Choc V1 Hotswap RGB`), switch orientation, Update assembly preset, and Duplicate design as variant.
- Switch footprint, diode direction, Edge gap X/Y, and a preview computed as `max(0, pitch - edge gap)` per axis.
- Delete matrix.
- Add object offers Add row and Add column for the selected matrix.

The screenshot `react-matrix-settings.png` captures these expanded sections. No project edit was submitted; the accepted document remained unchanged. Current structural Add/Delete/variant behavior will be verified on the integrated Dioxus candidate by the owning helper.

## Matrix spacing, history, and reopen journey

Candidate: Dioxus source `37d81320712b16cd6901f8e8ea3c80a0833ce576`, served at `http://127.0.0.1:34758/boardstudio/`, provider provenance SHA-256 `540bccd3ef9eab93cdc521191acd918d3ca1b605ec0969ba6a139ec517fa391f`. The parent strict page all-target Clippy check passed (15.81 s); package validation reported 8 fresh and 22 inherited commands, 1,365 sources with zero drift, and 145 assets with no root/subpath mismatch.

The paired journey used pinned React (`5a472a9426e6e38993361da402cd4ec730feb369`) at `http://127.0.0.1:5173/` and the same original archive above. Browser sessions were `f32d-react-config-eeb225b34b3e` and `f32d-dioxus-config-eeb225b34b3e`.

On the selected Left PCB `keys` matrix, I changed Edge gap X from `1.0` to `2.5` mm in each frontend. Both saved the value and updated the preview from `18.1 × 18.1 mm` to `16.6 × 18.1 mm`. Undo restored `1.0` and the original preview in both; Redo restored `2.5`. After a browser reload and reopening the saved Dioxus project, both frontends still showed Edge gap X `2.5` and the same preview. The candidate showed revision 10 after edit, 11 after Undo, and 12 after Redo; the reopened candidate remained saved at revision 12. No other matrix field was changed. Browser error output was empty.

Committed and reopened captures are `react-edge-gap-committed.png`, `dioxus-edge-gap-committed.png`, `react-edge-gap-reopened.png`, and `dioxus-edge-gap-reopened.png`. The structural Add row/column, Delete, and Duplicate variant leg is owned separately and remains pending in this receipt.
