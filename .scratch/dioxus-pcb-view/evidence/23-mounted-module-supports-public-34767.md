# Issue 23 paired public receipt — mounted-module supports and clearance

Date: 2026-10-03  
Candidate: `http://127.0.0.1:34767/boardstudio/`  
Build: `frontend-six-stream-parity-recovery-20261003` / port `34767`; full 22-command package PASS, 1,372 sources and 146 assets served with no route mismatch, per root package verification.  
Source: `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd`  
Provenance SHA-256: `96d7bc6289aedaadd8198bb9038c67d24fc659fde847362b24c7504313cbc733`  
React reference: `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`.

## Fixture and selected placement

Both named agent-browser sessions imported the same local archive through the visible import control:

- Fixture: `/home/chris/.local/share/boardstudio/reviews/pcb-module-footprints-20261003/vik-module-review.boardstudio`
- SHA-256: `952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`
- Target: `review/splitter-above`, definition `vik:vik-splitter:pcb-vik-splitter-vik-splitter`

On Dioxus, the route was PCB → focus and Enter on “Select mounted module module to split one vik in connector to two vik outs.” The selected inspector reported Main board / Board-wide / Front host face / Back facing face, X29, Y20, yaw0, gap3, Board attachment, connected and not detached. The React inspector reported the same placement as Main board / Every instance / Above PCB / Module back, X29, Y20, yaw0, gap3, Travels with PCB.

Baseline on both sides: service clearance 4 mm; board standoffs PART0 and PART3 each OD6 / ID2.8 / Z−3.8 / height3 mm. Accepted resolved geometry showed PART0 center20.06,30.55 and PART3 center38.05,9.56, each OD6 / ID2.8 / height3 mm. Source-hole choices were PART0 and PART3 from the mounted module definition.

## Paired change and result

In the same selected-module draft on both sides, I removed the existing PART3 standoff, changed extra service clearance from 4 to 4.5 mm, then added PART3 again with OD6.5 / ID2.8 / Z−3.8 / height3 mm. PART0 remained unchanged. After Save, both inspectors showed the new accepted values and resolved geometry: PART0 center20.06,30.55 / OD6 / ID2.8 / height3; PART3 center38.05,9.56 / OD6.5 / ID2.8 / height3. The project remained connected; the selected placement’s connector, board/face, pose, gap, attachment and detached state were unchanged.

On Dioxus, Undo returned the selected placement to clearance4 and both original OD6 standoffs; Redo restored clearance4.5 and PART3 OD6.5. After a page reload, reopening the locally saved VIK project and selecting the same module showed the changed accepted values and resolved geometry. The exported project archive after Save was revision7, SHA-256 `498f670b158a7273f4530c95b192564e329746c75da6ed1eb2031e85e6e3b588`; after Undo/Redo and reload it was revision9, SHA-256 `ff2c0ed5ac0e01203f02f39f7604490453f006f55172a8a2cda2c897635758ae`. The two archive `project.json` files are identical except revision (7→9). Reverting only target `serviceClearance` and `mountSupports`, and normalizing revision, makes the complete changed project document byte-structure-equivalent to the original fixture JSON. This confirms the paired edit preserved all other project fields and module placements.

## Receipts

All files are under `/home/chris/.local/share/boardstudio/reviews/pcb-mounted-module-inspector-20261003/`.

- React screenshots: `react-supports-inspector-before-34767.png` (SHA `b98eefd0706b1e57c7e85fadb259fa8026e2448cb5d5e319614649928be6a114`), `react-supports-draft-34767.png` (SHA `a3946187fa6d1af3a97cef9d7fa35de96c502b8808afdad16c4679f57d6e7693`), `react-supports-accepted-34767.png` (SHA `b2a8c3c8c3a2c63c37140bf7299554eda26ed1de99abe8461fe6237483213359`). React accepted archive: `react-supports-after.boardstudio` (SHA `0094b72a1dc138537c01c38a8a73fbf86bf119545d79b232164187a58d5610ab`).
- Dioxus screenshots: `dioxus-supports-before-34767.png` (SHA `cbe9ddeab080f26492601c3a67502a1787257124bd85731a615ed9793bb5e984`), `dioxus-supports-draft-34767.png` (SHA `79188ccf5c1c4f68beca211222b9b52f4e50c23d7cbc01f2fa477c96bef95909`), `dioxus-supports-accepted-34767.png` (SHA `4a9a0b692a047585e940022b53f6618e94fd85c4e234a1099781d6a7200b5404`), `dioxus-supports-undo-34767.png` (SHA `75a68b27f9e0ee4814240b59fdae0c2d4fe7da9e295f66c63db07d49ea7e18ef`), `dioxus-supports-redo-34767.png` (SHA `d5f8cfa211816b58bea264658d1d62bb78c984f66711e1ce6dcac021489e9f85`), `dioxus-supports-reloaded-34767.png` (SHA `80062ff8d786274336c387f35cccbf9e0a61f406428b876465c9194abfc563d7`).
- Dioxus archives: `dioxus-supports-after-save.boardstudio` (SHA `498f670b158a7273f4530c95b192564e329746c75da6ed1eb2031e85e6e3b588`) and `dioxus-supports-after-reload.boardstudio` (SHA `ff2c0ed5ac0e01203f02f39f7604490453f006f55172a8a2cda2c897635758ae`).
- Session names: `pcb-module-supports-reference-20261003-2318ea1a1376` and `pcb-support-clearance-candidate-34767-20261003-2318ea1a1376`.

## Scope and architecture notes

This qualifies the changed clearance/support interaction on this fixture, including one ordinary saved edit, its Undo/Redo and reload persistence. The resolved geometry display agrees with accepted source-ring inputs for this case. These designer-selected values are not vendor specifications. This receipt does not establish source readiness, universal PCB/case physical-contact validity, manufacturing readiness, module circuit/connection authoring, component editing, module profile editing, or completion of F5.5 or other F5.4 behavior.

The Dioxus selection entry is the PCB canvas’s existing mounted-module selection action, and the edit remains scoped to the selected placement through the existing project/Session/Core path. No new store, API or durable UI state was introduced by this journey. Findings remain under RF-001 shared Editor composition, RF-006 selected project/module/physical-instance scope, and RF-009 paired feature accounting.
