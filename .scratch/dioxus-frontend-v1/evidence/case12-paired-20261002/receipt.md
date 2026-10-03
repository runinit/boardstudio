# Case12 paired browser receipt

Date: 2026-10-02 (local Toronto time)

## Provenance

- React reference: `http://127.0.0.1:5173/`, live pinned TypeScript app.
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

This is a bounded Case12 browser journey, not full Case acceptance. The 90/90 count and visible board models do not prove every source model identity, per-row transform, all 90 picks, portable archive reopening, stale-source races, or complete contextual construction/hardware parity. The Dioxus manual-generation/reselection behavior after an invalid mechanical edit and post-reload deserves a targeted parity decision/issue link before calling the Case generation journey equivalent. The 2.0 mm edit intentionally triggered the same mechanical findings gate in both.

`agent-browser find label "Part colour" fill "#ffcc00"` was tried in both Chromium pages; both native color inputs normalized the fill to black. This automation path is not a valid app color-picker interaction and is excluded from product findings. Reset colour restored the original fixture value (`#b4bac2`) in both.

## Captures

All captures are 1280x960 and stored alongside this receipt:

- `react-case-plate-inspector.png` — initial React generated assembly / layer Inspector.
- `dioxus-case-generated.png` — Dioxus generated assembly and 90/90 readout.
- `dioxus-case-plate-hidden.png` and `react-case-plate-hidden.png` — paired visibility result.
- `dioxus-case-after-plate-edit.png` and `dioxus-case-after-retry.png` — invalid thickness edit and blocked generation states.
- `dioxus-case-reopen-plate.png` — reopened candidate, regenerated 90/90 assembly, selected Plate, resolved 1.50 mm.
- `react-case-initial.png` — initial React Case workspace before mechanical stack setup.

SHA-256 for the PNG captures is recorded by `sha256sum *.png` in the retained session log; see hashes in the parent handoff.
