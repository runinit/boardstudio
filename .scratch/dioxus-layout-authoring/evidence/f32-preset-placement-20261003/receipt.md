# F3.2a preset assembly placement journey

**Reference source:** React `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/`.

**Candidate:** `http://127.0.0.1:34767/boardstudio/`, source `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd`, provider/build provenance SHA-256 `96d7bc6289aedaadd8198bb9038c67d24fc659fde847362b24c7504313cbc733`.

**Fixture:** `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. The reference and candidate used separate named browser sessions and isolated profiles.

## Changed journey

Both apps imported the same saved Sofle fixture and selected the `MX Solder` assembly. In React, **Place key assembly** switched to Layout with the 1×1 ghost. Escape removed the ghost without adding a matrix. A second placement moved with ArrowRight and Enter committed `Matrix 5`; the board changed from 70 to 72 parts. Undo removed that matrix and returned the count to 70; Redo restored the selected `Matrix 5` and 72 parts. Reload retained the matrix.

In the Dioxus candidate, **Place assembly** switched to Layout and exposed the matrix-placement canvas. The preview SVG group `.m1-matrix-placement-preview` contained one 18.05 mm cell. Pointer movement set the preview transform to `translate(71.44762375249502 -57.79437125748501)`; ArrowRight changed it to `translate(76.21012375249502 -57.79437125748501)`. Escape removed the preview and the tree remained at 70 parts. A second placement moved to `translate(88.50259381237525 -49.2668862275449)` and primary pointer-up committed the selected `Matrix 5`; the tree showed 72 parts and one 1-key matrix. Undo returned to 70 parts with no matrix, Redo restored `Matrix 5` and 72 parts, and reload retained it.

No page errors were reported in either session. Candidate console output contained only Vite connection debug messages. The screenshot set is in `screenshots/`.

## Source-reviewed race follow-up

The journey did not discriminate the narrow completion race in which a newer same-board selection arrives while save is pending; no executed regression is claimed. Isolated follow-up commits `e959bd02b2ddf00d1fd9792f65994f1a467d33ca` and `2901d3162dba711a535e5962528d9be52013384f` capture the selection context and selected IDs at commit and suppress stale completion selection/feedback while retaining Layout/scope-generation guards. Those commits are not in candidate `34767` and still need inclusion in a subsequent qualified candidate.

This journey advances F3.2 evidence only; it does not close F3.2 or other Layout parent criteria.
