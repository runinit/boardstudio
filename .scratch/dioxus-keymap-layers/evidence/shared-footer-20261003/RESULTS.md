# Shared footer paired journey and placement correction

Reference: React `5a472a9426e6e38993361da402cd4ec730feb369`. Before-correction candidate: `http://127.0.0.1:34762/`, source `900068a0df413059732f9f365abd598734b7f86c`, build `frontend-footer-empty-assemblies-shell-20261003`, provenance SHA-256 `319f378ba0d2eb0855059d2368e1e8028c0c6267e0a50a22b9ed8e8a195e1251`.

The paired 2D journey exercised Layout Grid → the existing Snap menu → `½u`, PCB Grid → the same Snap menu → `1u`, then Fit board, Fit selection, and one zoom action. Layout and PCB footer Grid reflect their accepted snap fractions. Keymap and Keycaps show Grid disabled while Fit selection is enabled for selected `left-keys-SW1`. React ended at 332% and the candidate at 334% after the same fit/zoom sequence; this is a measured two-percentage-point display difference with near-identical rendered framing, not exact zoom equivalence. The candidate remained Revision 3 / Saved through these footer actions. No broader camera, edit-history, or parent acceptance is claimed.

Screenshots retained from source `900068a0...`:

- `candidate-layout-baseline.png` — SHA-256 `5c3fc7499b3ea907c2ec3bacf2c140e8ef39fe92e6a2d1fd6dd4a2750e160590`
- `candidate-layout-grid-snap-menu.png` — SHA-256 `e2971742d34ed03cbefca7d2d3ae9450738b44094d20621d48a84896677a364f`
- `candidate-layout-fit-selection.png` — SHA-256 `63caa6c47493c867b7210cb696a72510b866fec8c63118e18dec884c7f6c53fe`
- `candidate-layout-fit-selection-zoom.png` — SHA-256 `2c61d9acf18b2a4dc5d10ec97a973049dd2942a0c1374d3332c53e2e2019db89`
- `react-layout-fit-selection-zoom.png` — SHA-256 `94feee9aa4967b57aab74d85ec38df25ce26b4769a9001dae2603f5c631468ac`
- `candidate-keycaps-fit-selection.png` — SHA-256 `d6efafa58bdef0b521144e0bc005bb56ac2ade679e091ee803f62dafcf3f2d7a`

The paired 1280×577 footer comparison showed the candidate's `Project · Revision · Saved` text consuming the flexible footer space: coordinates began around x785 and Grid around x968. React places the history actions at left, coordinates immediately after, Grid/Snap and fit/zoom in a centered group, and its actual Layout findings action at far right. The source correction removes the long footer text, centers the existing Grid/fit/zoom group using the two flex margins, and moves `Revision · durability` into the existing Current project disclosure. The topbar save-state indicator remains. No Layout findings button was fabricated: a general Layout findings entry is a separate missing route/control and remains open under its existing Layout ownership.

The source change is not yet browser-qualified. These screenshots are explicitly before the correction; the next integrated package must confirm the revised placement. No new test or compiler run was added for this reversible CSS/markup change, per the shared candidate workflow. Root's combined check/package remains the source qualification gate.

A separate pinned React 5173 inspection of the advanced Keymap pane found the existing Macro editor and encoder CW/CCW binding controls present with matching default values/control kinds on candidate 34761. Add Macro was undone in each profile after comparison. No missing Macro/encoder control was established by that inspection. INT.2 is already accepted; export/provider output remains with the active F8.2 owner. Do not create duplicate Keymap tickets from this observation.

RF handoff: retain these source-to-UI composition and evidence-accounting observations under existing RF-009. No new RF item is proposed. Shared footer actions and this visual correction do not accept F3.3/F3.6/F5/F6/F7 or close their broader joins.

## 34763 placement/disclosure qualification (RED; source correction pending)

Candidate: `http://127.0.0.1:34763/`, source `99ec041a2895e5ab23880be25501beb9360487db`, provenance SHA-256 `d3c03d6db107ab7a55753b5798d99444f7d0843540ef96530c00d9f44296fe08`; root reports 1,371 inputs / 145 assets with no drift, 8 fresh + 22 inherited packaging commands (95.62 s), strict page WASM Clippy 17.97 s PASS. The same `Sofle v2` demo was open in React 5173 and candidate at 1280×577; this replay only inspected footer placement and the Project disclosure, with no edit, camera, grid, or history action.

React measured: history x14–87, coordinates x99–204, Grid/Snap x471–621, fit/zoom x633–877, Layout findings x1143–1266. Candidate measured: history x12–84, coordinates x96–196, Grid/Snap x553–692, fit/zoom x704–924. Thus coordinate order/placement is aligned; the camera group remains 47–82 px to the right and the real React Layout findings entry has no Dioxus counterpart. No empty spacer workaround or fake findings control was added; the measured offset remains open pending the actual Layout findings route/appropriate shared layout.

The candidate Project menu displays `Revision 3 · Saved`, but DOM inspection shows it as a sibling of the `.m1-project-current` section (`parent=m1-library m1-project-menu-library`); the section itself contains only the name label/input. Source commit `68bf84a4dcfa1a276ff7a2ee660851d246344df3` moves that status into the existing Current project section. It is not yet packaged/browser-qualified. Evidence: `candidate-layout-postfix.png` (SHA-256 `6c910fae17d12bcd978d17a0138c5d177aab2d028b098a10377f1f0b540ec631`), `react-layout-postfix.png` (`61ad788aacaaed16f76c4f3ba6aee8afe5cbcfe1d22fb030d2aaa4185fa3ada3`), and `candidate-project-disclosure-34763.png` (`ebf401d1eb61d049732d44bc67b98a0b2ea82ccb651b5ac1185da31b740bdbe6`).
