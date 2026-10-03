# F3.2c selected-matrix Add menu receipt

Reference: React source `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5173/` in named profile `f32c-react-fixture-99f70c00b2f7`.

Candidate RED: Dioxus source `bbd4da1b7cc0609dd4ae6d8ec0332031b0690ea1`, served at `http://127.0.0.1:34769/boardstudio/` (provenance SHA-256 `38e1980f3c38f31af1d66676a56d2d6cf57c7a3e4130dea6be2d1cb0eeb2f680`) in named profile `f32c-dioxus-component-99f70c00b2f7`.

Both pages loaded the same original archive, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df` (`keymap-layered-public/fixture/layered-sofle-export.boardstudio`), then selected `Key 1.1` in matrix `keys` and opened **Add object**. React shows its **Selected matrix** section and defaults **Part placement layout** to `keys`. Candidate 34769 omits **Selected matrix** and defaults **Part placement layout** to **Board / ungrouped**. These are the paired RED observations this source change addresses.

The paired Parts screenshots retain the existing `Apply to selected key` action on the same fixture/context. The candidate also retains its existing standalone Add Object placement path; that route is not changed by this correction.

No broad UI suite or heavy build was run. Root owns the integrated affected check and next-candidate changed GREEN.
