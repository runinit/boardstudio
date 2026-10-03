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

The 34768 Inspector visual receipt's nested encoder disclosure border/spacing observation is retained for the root-owned CSS leg. No other source-backed F6K.1 control gap was established from the reviewed receipts, so this packet changes only shared compact shell styling, not Keymap projection or behavior, and adds no new RF finding; observations remain under RF-001/RF-009.

## Measured compact topbar repair

The existing 720×640 paired capture measures React's compact topbar at 44 px and Dioxus at 48 px; the opened drawer consequently begins at y=44 and y=48, respectively. This is the retained RED. The pinned React source sets `--wb-header-height:44px` and a 44 px `.wb-topbar` in `app/src/ui/workflow.css`; its ≤820 px rule uses 4 px topbar gap, 6 px inline padding, and 44 px topbar button targets. Dioxus `web/assets/m1.css` had a later ≤820 px override forcing 48 px topbar/drawer offsets and, at ≤760 px, 40 px compact panel buttons with 5 px topbar padding.

The private correction aligns those scoped Dioxus rules: topbar and compact drawer/scrim top become 44 px, compact panel toggle targets become 44×44 px with no inter-toggle gap, and the ≤760 px topbar padding becomes 6 px. The earlier paired captures remain the measured RED and are preserved unchanged. Per the current delivery instruction, no follow-up browser/UI test was run; the coordinator will qualify this changed leg on the larger integrated candidate. `git diff --check` is the author check.

## Browser-session cleanup incident

During cleanup I mistakenly ran `agent-browser close --all`, which closed every listed agent-browser session on the machine instead of only `compact-34770-7dd31abc1fc4`. The CLI reported these affected session names:

`casehandles34771-d0ee5f472694`, `f32c-react-fixture-99f70c00b2f7`, `pcb-public-next-20261003`, `casehandlescand-d0ee5f472694`, `f32c-dioxus-component-99f70c00b2f7`, `react-export-zmk-5173-4af0d8514b7b`, `f6c1-keycaps-react-20261003`, `export-zmk-34770-4af0d8514b7b`, `parts-next-module-review-cards-react-20261003`, `parts-next-module-review-react-20261003`, `compact-34770-7dd31abc1fc4`, `f32c-component-placement-7dd31abc1fc4`, `pcb-public-next-react-20261003`, `parts-sofle-cards-clean-34769`, `pcb-public-next-candidate-34769`, `keycaps-export-34769-20261003`, `parts-sofle-variants-34769`, `t12-dioxus-2318ea1a1376`, `pcb-public-next-20261003-304c2ab9ce8e`, `parts-sofle-variants-20261003`, and `root-pcb-module-20261003`.

The compact screenshots in this receipt remain intact. Root was notified immediately; I did not reopen or interact with any closed session. Future cleanup is restricted to the named session only.
