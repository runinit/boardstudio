# PCB02 public paired qualification — 2026-10-02

## Candidate and fixture identity

- React reference: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34735/`; the `/boardstudio/` route was also opened and exercised after reimport. Build `frontend-authoring-layers-integrated-20261002`, source `7d09d0a60fbb2cc12e259541614b9483ee618d29`, `provenance.json` SHA-256 `aabc367a69f40dce0601226a7909a21f71950824534137a2d41a7253d66b7a66`.
- Root reports 22 successful package commands, 1,342 maintained inputs, 145 route assets, zero input drift, and root/subpath route asset plus COOP/COEP verification. Independent package review: `/home/chris/.local/share/boardstudio/reviews/frontend-authoring-layers-full-package-review-7d09d0a6-sol-20261002.md`, SHA-256 `c5a8009af0c35c7bc11fda3aa0f9a41c8a3ca673af7341c6d2c15edad32443ca`.
- Project in each fresh named browser session: layered Sofle export, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. Both opened Left PCB with 70 parts at 1280×577. Browser profiles were `pcb02-react-2318ea1a1376` and `pcb02-dioxus-2318ea1a1376`.
- The exact candidate archive exported before and after the visibility journey is retained as `dioxus-before-layers.boardstudio` and `dioxus-after-layers.boardstudio`. Both are SHA-256 `d68e6651757b00ce6269ee521bd10acc7cc175d0faafe22813d3abb98614211c`. This is equality of the candidate before/after exports; it does not assert equality with the original input archive.

## Host layer behavior

Both apps showed `Copper`, `Technical`, and `Objects` groups. Candidate controls matched the reference host controls and supported names for `B.Cu`, `F.Cu`, `B.SilkS`, `Dwgs.User`, `F.CrtYd`, `F.Fab`, `F.SilkS`, `Edge.Cuts`, `Courtyards`, `Pads`, `Holes`, and `References`. The browser snapshots and menu screenshots are retained in `screens/`.

On the Dioxus scene, each control was hidden and restored, with DOM geometry counts observed in the named session. Baseline counts were 160 pads, 198 drills, 70 reference labels, 1 outline and 70 courtyard surfaces; emitted generated layer counts were 232 B.SilkS, 20 Dwgs.User, 4 F.CrtYd, 4 F.Fab and 47 F.SilkS. Observed changes:

| Control | Observed geometry change when hidden | Restored |
| --- | --- | --- |
| B.Cu | pad paths 160 → 102 | yes, 160 |
| F.Cu | pad paths 160 → 102 | yes, 160 |
| B.SilkS | B.SilkS paths 232 → 203 | yes |
| Dwgs.User | paths 20 → 0 | yes |
| F.CrtYd | paths 4 → 0 | yes |
| F.Fab | paths 4 → 0 | yes |
| F.SilkS | F.SilkS absent; mapped companion count 232 → 29 | yes |
| Edge.Cuts | outline 1 → 0 | yes |
| Courtyards | courtyard surfaces 70 → 0 | yes |
| Pads | pad paths 160 → 0 while drills remained 198 | yes |
| Holes | drills 198 → 0 while pads remained 160 | yes |
| References | labels 70 → 0 | yes |

Each accessible button changed between `Hide …` and `Show …` as expected. Geometry was observed in the candidate browser; the table is not a pixel-diff claim against React. The React menu and Dioxus menu screenshots preserve the paired affordances.

## Shared visibility and non-editing behavior

Hiding B.Cu and F.SilkS in candidate PCB changed the controls to `Show B.Cu` and `Show F.SilkS`. Moving to Layout preserved the PCB visibility choices in the shared owner; Layout's separate `Footprints` button remained independently toggleable and returned to its original state. Returning to PCB retained the hidden host layers. Switching Left → Right → Left retained the host hidden-layer choices and repopulated Left's generated layer list after its async source discovery settled; no Right-board-only stale rows were observed in the settled screenshot `screens/dioxus-pcb-left-roundtrip-hidden.png`. `screens/dioxus-pcb-left-restored-hidden.txt` is an interim snapshot captured before technical rows had repopulated, and is not the settled-state oracle.

The candidate stayed at `Sofle v2 · Revision 9 · Saved` during visibility changes. Before/after archive equality above confirms no serialized project change. The Undo/Redo controls were present, but their enabled/disabled state was not used as an assertion. No source-document edit was invoked. Opening `/boardstudio/` directly returned to the app's initial “Open a keyboard” state; reimporting the same fixture then showed the expected 70-part layer menu. This is route coverage, not reload persistence.

## Remaining parity gaps found in the same paired view

1. **Mounted modules visibility row:** React exposes a distinct `Mounted modules` group and `Footprints` control (`Hide Footprints` in the baseline snapshot). Candidate has host layer groups but no mounted-module group/row. React source keeps this under a separate `hiddenModuleLayers` owner; its PCB host scene is distinct from host source parts and from Layout's `showFootprints`. The candidate's Layout `Footprints` control therefore does not satisfy the PCB row. The 5b Sofle fixture contains zero `.wb-module-*` rendered geometry, so clicking the React row proves the affordance and separate state, not a visible geometry effect. The missing row remains open; a module-bearing selected-board fixture is required to qualify the hide/show geometry effect.
2. **PCB edit toolbar:** React exposes `Select: Key`, `Transform`, `Align`, and `Snap`; candidate does not.
3. **Objects tree / Add object:** React offers `Add object` and individual flat parts entries; candidate shows Matrix group entries (for example `keys Independent`, `thumbs Independent`) and no equivalent `Add object` action. Keep this distinct from host layer controls and from the existing F3.2c Parts placement contract.
4. **Wiring actions:** React exposes `Wiring mode` and `Apply wiring`; candidate has an `Electrical wiring` section and Resolve action but no mode selector or Apply action. Existing F5.2/F5.3 authoring criteria remain open; do not count the read-only resolver preview as Apply parity.

The snapshots are evidence of visible controls at this viewport, not full behavior qualification of toolbar, tree, wiring, keyboard/focus, compact layout, theme, or assistive technology. Keep canonical PCB07/08 tickets and all 62 parent joins open.

## Evidence files and hashes

Retained candidate exports: the two archive hashes above. Key snapshot hashes:

- `screens/react-pcb-before.txt`: `f16954c8546d8d9661959b0cd738904403a045b9a357281175c39e8e134169a4`
- `screens/react-pcb-layers-full-snapshot.txt`: `871a9f8fee763fceece59ead8b83db2daa5926c3b8aa9fae69ccb39f6640e0cc`
- `screens/dioxus-pcb-before.txt`: `c9e10ac52facb1dd74d3acf3aa39e7f5f1f2ae8b2f98cbdeb35a3a95a6c4f3a7`
- `screens/dioxus-pcb-layers-full-before.txt`: `eb91564ef486d5eada4303283ce06df1c95507834636ef91808a9099450d3ae0`
- `screens/dioxus-pcb-retention-hidden.txt`: `5f2cf3cbdfa068f11b735c4dedcf08f7a33c806b810a52f56a41f9445403e002`
- `screens/dioxus-layout-controls.txt`: `1c316780c7a0df1291f7653c1c3476766bdcaf02eb8be621918eb6039ada3950`
- `screens/dioxus-pcb-after-layout.txt`: `a3eb1403c1976d341bb868d532385ffe31aeb332bc1bc50e2b1d6241058821c5`
- `screens/dioxus-pcb-right-board-hidden.txt`: `794d42b3cb5519408f82681c5e5ef4c62ad5792930f4a67e3bc4989e740f6893`
- `screens/dioxus-pcb-left-restored-hidden.txt`: `40364e4076d097a29f6300d688fff5654c2dc260a963bb81625e4a587600c313`
- `screens/react-pcb-layers-menu.png`: `3dc25e9273eecada814480334f02f40d300b9cf61cd3e4bc635fcc43f5825ca9`
- `screens/dioxus-pcb-layers-menu.png`: `c84290436d65cbbf08d7dd33f24c3f07c5329ccc8e10b0297a76cea251f83f75`
- `screens/react-pcb-hide-footprints.png`: `999df5d024de7bef682088513df9db96a870d65e9a2dd7f6cb62ccc737bb0e57`
- `screens/dioxus-pcb-right-board-hidden.png`: `332598c6baaa4c264c93acdbf85a2eb8cbabb031752399cd4f048d03c6938b5c`
- `screens/dioxus-pcb-left-roundtrip-hidden.png`: `6cc07347e67ab8064c1769dcd8ca7ae9ba5f35e5c32e1dc4eef7d5a9b4ecae2d`

Reference browser snapshots capture exact interactive names/order; candidate full and transition snapshots retain actual menu and currentness states. The external evidence staging copy under `/home/chris/.local/share/boardstudio/evidence/pcb02-public-qualification-20261002/` is the original named-session output.

## Review and remaining gates

Source review is reused only for the mounted PCB02 layer implementation: source leaf review `32a3b34` was CLEAR on both axes; serial root join `7d09d0a6` was independently CLEAR. The package review above is independent build/provenance review. This document supplies paired public layer evidence, not source re-review or parent acceptance. F5.1/F5.2/F5.3, PCB07/08, compact/theme/accessibility checks, full browser history/reopen journeys, and all 62 canonical parent acceptance joins remain open. No new refactoring takeaway observed; carry forward RF-001, RF-006 and RF-009 without changing their ledger entries.
