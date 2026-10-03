# Case12 paired browser receipt

Date: 2026-10-02 (local Toronto time)

## Provenance

- React reference: `http://127.0.0.1:5173/`, pinned source commit `5a472a9426e6e38993361da402cd4ec730feb369`. The source mapping is pinned; no separate retained launch/session log was available for this run.
- Dioxus candidate: `http://127.0.0.1:34734/`, root and `/boardstudio/` routes from build `frontend-contextual-case-mirror-keycaps-20261002`.
- Candidate source: `f261a327858f51a1de4928374bc65b669e2792a3`.
- Candidate build provenance SHA-256: `f77c5920371ae6a69e42ae42e20f0eef2519f7822f1eb404f4b580c17b38e066` (22 commands successful, 1,331 source hashes and 145 assets per route, per root build receipt).
- Fixture imported into both app origins: `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Browser automation used an isolated named `agent-browser` session (`case12-paired`) at a 1280x960 viewport; React and Dioxus occupied separate tabs.

## Journey and observations

1. Imported the same archive into both apps and opened Case. React entered Case with no physical generation until `Configure mechanical stack`; its live preview started after opening settings. Dioxus displayed the accepted PCB preview and needed `Configure mechanical stack`, followed by an explicit `Generate case` action. Both then reported 90/90 models and rendered the Sofle assembly.
2. Selected generated `Plate`. Both showed a 1.5 mm plate thickness field, color control, and visibility control. The Dioxus screenshot visibly showed `Resolved thickness 1.50 mm`; React showed the same readout. The accessibility snapshot did not include this static readout, so visual screenshot evidence is retained.
3. Changed Plate thickness from 1.5 to 2.0 mm. Both blocked a new case generation due to mechanical findings and kept prior geometry visible. React retained generated layer rows and the Plate inspector; Dioxus returned to assembly settings and omitted generated rows while preserving old geometry. Pressing Undo restored the 1.5 mm configuration in both. Dioxus then required `Generate case` to rebuild the five-layer stack and restore generated rows; React returned directly to current geometry.
4. Hid Plate using the Inspector `Visible` control. Both showed an unchecked control, marked Plate hidden in Objects, and rendered the assembly with the plate removed. Reset its color and restored visibility in both.
5. Selected generated `Bottom case`. Both exposed matching Bottom thickness (3 mm), wall thickness (2 mm), clearance (0.3 mm), color/reset, and visibility controls. `Assembly settings` cleared the layer selection and returned to global mechanical construction settings in both.
6. Reloaded each app to check browser-local durability. React regenerated its saved Case preview automatically. Dioxus retained the 1.5 mm mechanical values and previewed saved board, initially showing zero generated stack rows until `Generate case` ran; afterward it again reported 90/90 decoded and five resolved layers. Re-selecting Plate showed 1.5 mm, `Resolved thickness 1.50 mm`, and Visible checked.

## Qualification

This is a bounded Case12 browser journey, not full Case acceptance. The 90/90 count and visible board models do not prove every source model identity, per-row transform, all 90 picks, portable archive reopening, stale-source races, or complete contextual construction/hardware parity. The remaining reload/context-loss readiness mismatch is tracked under [Case Issue 11](../../../dioxus-case-workspace/issues/11-case-generation-readiness-admission.md): React regenerates its saved Case preview on reload, while Dioxus requires an explicit Generate case action. This receipt records that open mismatch and does not claim Issue 11 or full Case acceptance. The Dioxus manual-generation/reselection difference after an invalid mechanical edit is also still open for parity follow-up. The 2.0 mm edit intentionally triggered the same mechanical findings gate in both.

`agent-browser find label "Part colour" fill "#ffcc00"` was tried in both Chromium pages; both native color inputs normalized the fill to black. This automation path is not a valid app color-picker interaction and is excluded from product findings. Reset colour restored the original fixture value (`#b4bac2`) in both.

## Captures

All captures are 1280x960 and stored alongside this receipt:

- `react-case-plate-inspector.png` — initial React generated assembly / layer Inspector.
- `dioxus-case-generated.png` — Dioxus generated assembly and 90/90 readout.
- `dioxus-case-plate-hidden.png` and `react-case-plate-hidden.png` — paired visibility result.
- `dioxus-case-after-plate-edit.png` and `dioxus-case-after-retry.png` — invalid thickness edit and blocked generation states.
- `dioxus-case-reopen-plate.png` — reopened candidate, regenerated 90/90 assembly, selected Plate, resolved 1.50 mm.
- `react-case-initial.png` — initial React Case workspace before mechanical stack setup.

SHA-256 for each PNG capture:

| Capture | SHA-256 |
| --- | --- |
| `dioxus-case-after-plate-edit.png` | `5bc690b7b11f55e7dab861dddf2e0d362074b1f0b0d1b248fe5c4c178b539038` |
| `dioxus-case-after-retry.png` | `27def418ac93e53f0ecf0a8d7497c987b0bb6ceccc3bc5e71b27ab5e8928ca73` |
| `dioxus-case-generated.png` | `d6dd366e51403ae75a337376e5457b6222dd4667921a7e6e18b206d9a4e4da79` |
| `dioxus-case-plate-hidden.png` | `6a5c3d3611c4a1eb28a6ee5394f5dbb31cb67b3c73aec3b5ab50cafa425e9172` |
| `dioxus-case-plate-inspector.png` | `3be152c56ccbc66a26f1ff0b35b281583a9a75bb7f41dcf565082e4f9a98f377` |
| `dioxus-case-reopen-plate.png` | `df2a5139110f43688b183c367d37df42a4fcf2cc4f3d0bafd638a147fb35273f` |
| `react-case-initial.png` | `b002e78a3b936f10fc3c8ed582815993e2ddaa601dcea9fed986e00bcbab8fb7` |
| `react-case-plate-hidden.png` | `8484c24063c03e25aa626a5e92c27064791c2ded8207cd709315aa4d31f7b02a` |
| `react-case-plate-inspector.png` | `6eeaf44af8dc2b8d6e0ec0f085132a52a5d38c20e947c0bdb645f9ceee5377dd` |
