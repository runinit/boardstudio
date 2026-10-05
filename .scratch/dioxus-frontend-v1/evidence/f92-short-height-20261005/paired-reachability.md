# F9.2 paired short-height reachability observation

Candidate `frontend-layout-untouched-coordinate-20261005` / `df6b04b9e8903381029a137fdfcef8d797305fb8` at `http://127.0.0.1:34829/` was paired with TypeScript reference `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5175/`.
Both imported `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/f86-integrated-20261005/fixed-candidate/candidate-joined.boardstudio` (SHA-256 `00b0d66e2adba13c6e9372764675e71b39da3da9b6e54d22728f4f62d82d60b2`, project revision 24) at **1280 × 500 CSS px** in separate named `agent-browser` sessions.

For each route, the named representative control was visible and its center-point `elementFromPoint` hit test returned the control or its descendant. Coordinates below are CSS pixels relative to the viewport; no scroll was needed. Controls were not activated.

| Workspace | Candidate control (x, y, w, h) | Reference control (x, y, w, h) |
|---|---|---|
| Layout | Board name (976, 236.266, 288, 32) | Board name (985, 228.469, 271, 32) |
| Parts | Search footprints (12, 165.391, 211, 34.844) | Search footprints (12, 171.391, 210, 34) |
| PCB | Wiring mode (976, 217.281, 288, 32) | Wiring mode Matrix Direct GPIO (985, 213.641, 271, 40) |
| Keymap | Layer name (984, 387.672, 272, 36) | Layer name (985, 398.266, 271, 36) |
| Keycaps | Board legend color (984, 305.672, 50, 36) | Board legend color (1208, 274.469, 48, 32) |
| Case | Live preview (558.656, 65.5, 13, 13) | Live preview (340.781, 108, 16, 16) |
| Export | Export Draft KiCad board (996, 360.328, 104, 36) | Export Draft KiCad board (998.578, 409, 61.422, 36) |

The Case representative is the enabled **Live preview** checkbox; Export geometry was visible but disabled in both apps for this fixture state. The Export representative is the enabled **Export Draft KiCad board** button; it was not clicked. The initial partial-stop note remains an incomplete earlier attempt; this paired run is the full seven-route observation. No model data was changed, no history or export/download action was invoked, and no screenshots, browser zoom, mobile, or accessibility checks were performed. These are reachability observations only, not criterion acceptance.

Full command logs, per-route interactive snapshots, viewport dimensions, and measured control inventories are in `run-20261005-paired/`.
