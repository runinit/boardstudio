# F3.4 Add object Board outline entry

## Reference and Red

- Pinned React source: `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5173/`.
- Dioxus RED candidate: source `2107f980e9b45c341bff10548c7156b28de0e388`, served at `http://127.0.0.1:34761/boardstudio/`, provider provenance SHA-256 `2d218936e847ba8f4d8680e5ecc4b3f5a05c3ce88c85a2152e2397cd84020ac9`.
- Dioxus GREEN candidate: source `900068a0df413059732f9f365abd598734b7f86c`, served at `http://127.0.0.1:34762/boardstudio/`, provider provenance SHA-256 `319f378ba0d2eb0855059d2368e1e8028c0c6267e0a50a22b9ed8e8a195e1251`.
- Fixture: `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Browser sessions: `f34-frontier-react-5173`, `f34-frontier-dioxus-34761`, and `f34-board-outline-green-34762`, all closed after capture with empty browser error buffers.

In React, Layout → Add object shows Board geometry with `Board outline…` and `Geometry scripts…`. Selecting Board outline closes the Add menu, selects the active Board's Outline tree row, and opens the existing contextual Board outline Inspector. The same action from PCB returns to Layout and opens that Inspector for the current board. The Inspector exposes current Generated/saved-version settings, gap controls, Edit perimeter points and the separate manual/saved geometry actions.

On candidate 34761, Add object showed Layouts and Parts, with no Board geometry entry. The accepted Outline tree/version Inspector and its perimeter-point route are already mounted elsewhere; the missing piece is this supported navigation entry. This implementation adds only Board outline routing through the current scoped `TreeSelectRequest`; the existing root `select_tree` callback remains the authority that switches from PCB to Layout and opens the accepted Inspector. It creates no settings/version store or new edit path.

- [React Add object Board geometry](screenshots/react-add-object-board-geometry-red.png)
- [React PCB-to-Layout Board outline route](screenshots/react-pcb-to-layout-outline-route.png)
- [Dioxus Add object missing Board geometry](screenshots/dioxus-add-object-missing-board-geometry.png)
- [Dioxus PCB-to-Layout Board outline route GREEN](screenshots/dioxus-pcb-add-outline-route-green.png)

On the 34762 GREEN candidate, I imported the pinned fixture, switched to PCB, opened Add object → Board geometry, and selected `Board outline…`. The app returned to Layout with the `Outline Generated` tree row selected and the existing Board outline Inspector mounted (`Active outline: Generated`; perimeter editing remains available). This verifies the supported PCB-to-Layout handoff without repeating the broader Outline workflow.

## Scope boundary

Geometry scripts is the existing F3.8 capability, not a decorative menu item. The Dioxus tree has no script Inspector/form adapter yet; that existing ticket now includes the React entry route and draft/history/save behavior. The Board outline entry is implemented first. Add the Geometry scripts button only with the real F3.8 panel/navigation owner.

The route preserves RF-001 (one Editor navigation/edit owner), RF-006 (current accepted Board and scope), and RF-009 (exact source/build/fixture proof). No new refactoring takeaway was observed. This is a child journey only; the remaining F3.4 manual feature/refinement workflows and F3.4/F3.7 joins remain open.

## Verification

The isolated implementation is `de177b75` (`Add Board outline entry to object menu`); root integrated it with shell/layout changes and the affected strict page check passed in the 34762 build. This single changed-journey GREEN is the only post-package UI check. No full-outline repeat or broad UI test matrix was run.
