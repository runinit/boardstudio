# Graphite usability corrections — 2026-10-13

This batch implements the five approved usability changes before further visual polish. The new header navigation explicitly supersedes the earlier decision to keep workspace navigation in Objects.

## Delivered

- Workspace navigation is back in the single header and removed from the Objects pane. Narrow headers scroll navigation horizontally and reveal the active workspace.
- Primary command toolbars are centered in the available viewer area. Keymap's previously hidden contextual toolbar is restored. Dock collapse and resize do not change toolbar alignment relative to the viewer.
- Layout and PCB visibility layers are expanded disclosures in the left dock. Assembly viewer controls render through a shared Dioxus layer outlet; existing viewer callbacks still own selection, visibility and colors. Standalone layer controls retain their previous initial state. The object tree and layers share a bounded scroll region; snapping remains outside it.
- Selected Layout columns expose Offset X/Y, Stagger and Splay together, with stagger, offset, splay and origin handles immediately available. Rows retain row offset; other supported matrix selections retain their existing move control. Handles remain after dragging, and the shared arbiter admits one gesture at a time. Canvas values display up to four decimals without turning untouched focus/blur into a rounded document edit.
- Short related left-pane actions use compact wrapping rows: board selection actions, matrix Add row/Add column, board geometry actions, Keymap operations, mirrored-pair actions and Parts actions. Tree entries and explanatory catalogue/menu rows retain full-width presentation.

No project schema, domain operations, palette or Case height/thickness handles were added. Existing shared controls and pinned primitives remain in use.

## Verification

The original isolated-profile browser regression was `scripts/test-graphite-usability.cjs` (now archived under `docs/archive/graphite/`). It first failed against the old UI for the expected navigation, layers, missing controls/handles, toolbar centering and Keymap visibility defects. The current manual command is `python3 scripts/run-graphite-usability.py`.

The completed checks cover typed edits through the Inspector owner; each drag's commit, Escape cancellation and single-step undo with all handles mounted; visibility independence; all six workbenches at 1586, 1024 and 390 pixels; toolbar centers within two pixels of viewer centers; and Layout toolbar alignment with collapsed/resized docks.

Additional checks: native Layout/shared tests; mounted transform, numeric Inspector, dock lifecycle and panel-scroll browser tests; Layout scene/layer integration; PCB scene layers; Case assembly layers; WASM page typecheck and preview build; scoped Impeccable layout detector and `git diff --check`. Exact outcomes are recorded in the accompanying verification text.

The first inspection discovered stale generated Tailwind CSS despite a live watcher. CSS was regenerated and the stalled watcher replaced with a scoped polling compiler for this worktree. The preview remains at http://127.0.0.1:8182/; its existing proxy/Dioxus processes were retained. Scratch tooling and logs live under `.scratch/graphite-usability/`.

## Screenshots

These captures use a fresh Corne demo copy, dark theme, and an MX Hotswap Parts preview. Browser storage belonging to the user was not changed.

| Workbench | Desktop | Intermediate | Narrow |
| --- | --- | --- | --- |
| Layout | [1586](Layout-1586.png) | [1024](Layout-1024.png) | [390](Layout-390.png) |
| PCB | [1586](PCB-1586.png) | [1024](PCB-1024.png) | [390](PCB-390.png) |
| Keymap | [1586](Keymap-1586.png) | [1024](Keymap-1024.png) | [390](Keymap-390.png) |
| Keycaps | [1586](Keycaps-1586.png) | [1024](Keycaps-1024.png) | [390](Keycaps-390.png) |
| Case | [1586](Case-1586.png) | [1024](Case-1024.png) | [390](Case-390.png) |
| Parts | [1586](Parts-1586.png) | [1024](Parts-1024.png) | [390](Parts-390.png) |

Compared with the approved Graphite references, the single shell and palette remain intact. The requested navigation relocation and always-visible geometry controls are intentional changes from the previous mockup composition.

## Remaining polish

The prior [drift assessment](../drift-2026-10-13/README.md) remains the broader backlog. This batch does not resolve the full-width Add object presentation, duplicate legacy Objects headings, noisy canvas reference labels, uneven Inspector spacing/borders, or Keycaps camera framing. Those should be handled as a separate focused polish pass; further onboarding interaction work remains separately scoped.
