# F3.2d Right PCB duplicate context on candidate 34760

Candidate: `http://127.0.0.1:34760/` (`/boardstudio/`), build `frontend-board-case-matrix-settlement-20261003`, served source `9718ea4aa3b29f1ae2f53d049a411928e65656bc`; provenance `d4e061e44423839546c370a71f4548b81db8dc7bbb5a27474dff39456928243c`. Parent reports strict Clippy PASS (14.78 s), 1,367 source hashes and 145 assets per route, zero route drift.

Fixture: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

In isolated `agent-browser` session `layout-matrix-context-2318ea1a1376`, imported the fixture, selected `Right PCB` and its `right half` physical instance, then selected the `keys` matrix (4 × 6). Opened Key assembly and chose Duplicate design as variant. On the saved `Sofle v2 (variant)` copy, the Board selector remained `Right PCB` and Physical instance remained `right half`. The tree selection cleared on project open; reselecting `keys` showed its 4 × 6 inspector. Session closed after the observation.

This receipt covers the changed normal duplicate-context path only. The original structural Add row/column, Undo/Redo, Duplicate/Delete journey remains in [the candidate 37d receipt](structural-journey-candidate37d.md); neither receipt covers an asynchronous stale-owner fault injection.
