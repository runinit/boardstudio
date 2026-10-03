# Keymap canvas fit controls paired evidence

## Baseline observation

- Reference: pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5173/`.
- Dioxus: public candidate `34747`, served at `http://127.0.0.1:34747/`; its packaged source is `f2d70ea627d851ea461033b44052bbf3740d37a1`.
- Fixture: the default Sofle v2 project, Keymap workspace, Base layer, 29 supported keys; both browser profiles were reloaded and the Keymap route allowed to settle before capture.
- Viewport: 1280 × 577.
- At this route, React exposes `Fit board`, disabled `Fit selection` with no selection, `Zoom out`, zoom percentage, and `Zoom in`. It also exposes the `Grid ¼u` status control, disabled in the observed no-selection condition. Dioxus exposes only the shared Undo/Redo and zoom percentage in its Keymap footer. Its 2D/3D/Footprints selector and the Keymap projection/layer controls are present.
- The narrow repair adds Fit board and Fit selection only. Zoom and Grid/Snap remain open under their existing viewport/Snap ownership; no full footer parity is claimed.

## Captures

- React baseline: `react-before.png`, SHA-256 `3b09a55e3ddc240593ab1095e2d3d9ee358865f2feba619dc5d94d3a0bc74e75`.
- Dioxus baseline: `dioxus-before.png`, SHA-256 `e53b218bd527d5954f5d3fdb61035094da1e03aa1c3b052045f0f902535693e5`.

## 34749 before-repair Fit board comparison

- Candidate: `http://127.0.0.1:34749/boardstudio/`, frozen source `dd7697ed99864040697c905b3d0e28209301d11b`, provenance SHA-256 `1f3584f7b41b9c2d912381a302657ccf202abc7ebcbf04622ef68bf62c0f932e`.
- Both named browser profiles were set to 1280 × 577 and opened the same local Sofle v2 project on Keymap, Base, with no key selected. One `Fit board` action was taken in each app.
- React reported 80% zoom and its canvas toolbar measured 44 px. Dioxus reported 100% and its canvas toolbar measured 42 px. The board rendering was visibly larger in Dioxus after the action, confirming the pre-repair camera math did not reserve the same toolbar/footer clearances or target padding.
- Before-repair action captures: `react-34749-fit-board-before-repair.png`, SHA-256 `c974b5719bf15d6bc12875c10e0ff307c3e28af577e531ae74c51113d9381139`; `dioxus-34749-fit-board-before-repair.png`, SHA-256 `06b3dc8d227c46f716e7ed47a168db1c4397652183d43bd339abb6a7b84dae37`.

## Changed journey

Pending the next page-only repair candidate containing the corrected fit math. Use the same two agent-browser profiles and fixture once:

1. In Keymap 2D with no selection, verify both routes expose Fit board and disabled Fit selection; click Fit board in each and compare board centering/scale.
2. Select `left-keys-SW1`, verify Fit selection enables, click it in each route, and compare selected-key centering/scale.
3. Verify the camera actions did not change the document revision or Undo/Redo state. Record candidate URL/source, screenshots, and the final observed result here.

This is one paired control journey, not full F6K.1 acceptance. F3.1 shared-selection acceptance and the remaining viewport controls stay open.
