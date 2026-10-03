# Keymap compact drawer and Export-selector receipt — 2026-10-03

## Source and fixture

The TypeScript oracle is pinned at `5a472a9426e6e38993361da402cd4ec730feb369` on `http://127.0.0.1:5173/`. The first Dioxus compact drawer/focus capture used package 34770, source `47623521` and provenance `a4677af663c0e8286ae5e923bf1884b0230bcc6dc852a3030cc35a68d453365c`. The changed Export-selector replay used package 34771, source `7938a895a6d2a9c3b8c09dda25ce70cd45f7d328` and provenance `717968702d722ed257f4bf5096360b7f8776eaf70ee3f056223f1fd7d0ece48f`. Both React and Dioxus used the author-owned `compact-34770-7dd31abc1fc4` browser session/profile at 720×640, Light theme. The drawer journey opened the Sofle v2 Keymap workspace with its 29 supported keys and two layers.

## Drawer and keyboard journey

Opening Objects closes Inspect in both apps. The Objects drawer measures 290 px in both. Opening Inspect closes Objects; the Inspector begins at x=350 and measures 370 px in both. React's Inspector is y=44 through y=595; Dioxus is y=48 through y=606. The topbar is 44 px in React and 48 px in Dioxus, a remaining 4 px difference. The drawer widths and exclusive behavior match.

With the Inspector open, focus its Close inspector control and press Escape. Both apps close the Inspector and move focus to the Inspect toggle. The before/open/Escape captures are in `screenshots/react-objects-open.png`, `screenshots/dioxus-objects-open.png`, `screenshots/react-inspect-open.png`, `screenshots/dioxus-inspect-open.png`, `screenshots/react-inspect-escape.png`, and `screenshots/dioxus-inspect-escape.png`. This is one compact route and focus journey, not an exhaustive keyboard or accessibility matrix.

## Changed Export-selector leg

The earlier 34770 browser journey observed a selector defect while Export was active: the content changed to Export, but the Workspace selector still displayed Layout. Root repaired the owner synchronization in source commit `5d2b4a98`. On 34771, entering Export sets the Dioxus select value to `Export` and marks the Export option selected; invoking Export again returns to Layout and restores the Layout value. The paired React path enters Export with the selector at `Export` and returns to its prior Keymap workspace with that value restored. Current screenshots and DOM-derived results are `screenshots/dioxus-export-active.png`, `screenshots/dioxus-export-return.png`, `screenshots/react-export-active.png`, and `screenshots/react-export-return.png`.

Root also corrected the desktop scrim's media gating in `d4870a1b`; this compact receipt does not repeat the desktop viewport check. The root owns that check. The 34771 source is the changed-selector package, not a replay of every unchanged drawer or Export behavior.

## F6K.1 disposition

F3.1 is accepted by the coordinator's decision in `.scratch/dioxus-frontend-v1/evidence/layout-f31-acceptance-20261003/DECISION.json` (candidate 34769/source `bbd4da1b7cc0609dd4ae6d8ec0332031b0690ea1`). Do not repeat its unchanged tree/selection matrix or describe its acceptance join as still open. F6K.1 itself remains implementing pending one consolidated Sol disposition; this receipt does not change canonical task status or claim whole-workspace acceptance.

The retained F6K.1 record is `.scratch/dioxus-keymap-layers/evidence/keymap-frontier-audit-20261003/criterion-accounting.md`. Paired/public evidence already covers the supported key/layer projection, stable identities, selection/search, active labels and empty states; paired fit/zoom receipts cover rendered camera bounds and display-only history neutrality; the current 34771 receipt adds compact drawer exclusivity/focus and synchronized Export selector. The shared 2D footer's Layout/PCB Snap and Keymap/Keycaps disabled Grid route remains in `shared-footer-20261003/RESULTS.md`; the selected Keymap 3D consumer receipt remains in `keymap-3d-consumer-20261003/paired-receipt.md`. Outline/version/bridge tree ownership remains shared T1-11/F3.4 and appears in current paired tree evidence; no duplicate Keymap child is proposed.

The 34768 Inspector visual receipt's nested encoder disclosure border/spacing observation is retained for the root-owned CSS leg. The 4 px topbar height difference above remains explicit. No other source-backed F6K.1 control gap was established from the reviewed receipts, so this packet makes no Keymap production change and adds no new RF finding; observations remain under RF-001/RF-009.
