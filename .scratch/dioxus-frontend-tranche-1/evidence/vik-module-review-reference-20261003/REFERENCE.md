# VIK module review — pinned React reference

- URL: `http://127.0.0.1:5173/`
- React source supplied by the coordinator: `5a472a9426e6e38993361da402cd4ec730feb369`
- Isolated browser sessions: `parts-next-module-review-cards-react-20261003` and `parts-next-module-review-react-20261003`
- Public card action: `Start VIK module review · above and below`
- Card title/summary: `VIK module review · above and below` / `15 keys · Single board`
- Card image: [`react-card.png`](react-card.png), SHA-256 `9dd8c5e7f32122223fd183ef702fd3d55cc4c3d23f03454918314b95e707f0fe`

Starting the reference card opened project ID `vik-module-review`, revision 6, one `Main board` with 33 parts. The document contains three mounted instances: `review/splitter-above` (front host face, back-facing surface, 0°) and `review/splitter-below` (back host face, front-facing surface, 180°), both from the same VIK splitter definition; and `review/ec11-rotary` (front/back, 0°) from the EC11/EVQWDG001 source. The document also includes an embedded haptic circuit, three real host connectors, review keymap/macro data and a `moduleHumanReview` note. This is source-backed review data, not fabrication or electrical qualification.

Observed source revision and hashes in the accepted module definitions:

| Source variant | Upstream revision | SHA-256 | Status |
| --- | --- | --- | --- |
| `pcb/vik-splitter/vik-splitter.kicad_pcb` | `cd5d16e4cd9137a229fc673412a89d75f4e64553` | `5f0ae2b03b296d7edae170a11d44920095c89d02eaff6819ad0470d2627b9a26` | Untested |
| `pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb` | `cd5d16e4cd9137a229fc673412a89d75f4e64553` | `a00cc6dadd021241d3460e5173bd9af0750f5948a4986297afa07e18bd8f15e2` | Complete |
| `pcb/ec11-evqwgd001/ec11-evqwgd001.kicad_pcb` | `cd5d16e4cd9137a229fc673412a89d75f4e64553` | `2b22c0f0b2b99206ac25029b8acd05c75146d9246cb8e3d9ceec5deacae4e033` | Complete |

The reference project currently uses stable demo ID `vik-module-review`; its UI open action calls `openModuleReviewDemo` directly. The Dioxus Project card follows the already-qualified `open_fixture` path and supplies a fresh accepted ID while retaining the source instance identities. The view and project data were inspected read-only; the 3D route was still showing `Preparing 3D geometry…` at the short capture and is not used as evidence for Issue 06.
