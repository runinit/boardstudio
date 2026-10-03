# F3.2d structural actions journey on candidate 37d

Candidate: `http://127.0.0.1:34758/` (`/boardstudio/`), served source `37d81320712b16cd6901f8e8ea3c80a0833ce576`; provenance receipt `540bccd3ef9eab93cdc521191acd918d3ca1b605ec0969ba6a139ec517fa391f` (8 fresh + 22 inherited routes, 94.92 s; 1,365 sources and 145 assets; zero route drift; HTTP 200 with COEP).

Fixture: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

Ran in the isolated `agent-browser` session `layout-matrix-structural-run`. Imported the fixture, left Board set to `Left PCB` and Physical instance set to `left half`, then selected the `keys Independent` matrix. The mounted matrix inspector began at 4 rows × 6 columns.

The Add object menu exposed Add row and Add column for the selected matrix. Add row changed the inspector from 4 to 5 rows (tree count 70 to 82 parts); Undo restored 4 rows and 70 parts, and Redo restored 5 rows and 82 parts. Add column changed 6 to 7 columns (tree count 82 to 92 parts); Undo restored 6 columns and Redo restored 7 columns.

Duplicate design as variant opened and saved `Sofle v2 (variant)`. Selecting its `keys` matrix showed the copied 5 × 7 dimensions. Delete matrix removed that matrix from the variant tree; the matrix inspector and selected-context section disappeared, leaving the general Position inspector. The imported source archive was not modified.

This journey exercised the normal mounted structural flow on candidate 37d. It did not test switching projects during the asynchronous duplicate preparation; that owner race is tracked and repaired in commit `8e9a4450e3633aa8ccdb31e313a45b10e68b1ab1` for replay on the next candidate.
