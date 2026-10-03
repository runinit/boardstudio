# F3.1 Board Inspector and F3.8 Geometry scripts paired journey

Candidate: `34764`, source `661296fe5633e2327488cd56738c14ab81c22341`, provider provenance SHA-256 `f597c5d1cd45a8222b5ce8628ffbb5508e7400094cf1f53bcf6518e1681519c0`.

Reference: React source `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/`. Candidate: `http://127.0.0.1:34764/boardstudio/`. Browser sessions were isolated: `layout-board-react-2318ea1a1376` and `layout-board-candidate-2318ea1a1376`.

Fixture: original layered Sofle archive at `.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

React and candidate both accepted a trimmed Board rename to `F31 Board Test`; the board selector, Objects row and Inspector reflected it. Undo and Redo each restored the expected accepted value. Escape and empty blur retained the accepted name. Candidate reload retained the board name.

Both implementations added an enabled script named `F31 bounded hole` with source `rect("extra", "hole", 30.0, -10.0, 3.0, 3.0, 0.0, "subtract");`. Apply produced one hole outline. Candidate reload retained the script and generated hole.

Screenshots in this directory capture the renamed Board and applied Geometry script on React and candidate, the reopened candidate script, and the candidate with the Main board row selected. `agent-browser errors --clear` returned no browser errors for either session.

## Remaining observed gap

On candidate with the Objects tree selection cleared, the Board Inspector displays its guidance and fields but lacks React's `Main board` heading/context. Selecting the explicit Board row shows the candidate selected-context heading. This candidate therefore provides partial journey evidence only; F3.1 remains open. The follow-on Layout packet owns the default Board title projection and the same-context Board/Geometry draft-owner corrections, plus safe Geometry script ID allocation and the missing preset assembly placement workflow.

No broad tests were run by this author. The integrated affected page check and candidate review remain with root.
