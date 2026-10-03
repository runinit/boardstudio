# PCB02 paired public evidence review — 2026-10-02

Reviewer: independent Sol 6.1 High. Bounded receipt/artifact review without source edits, root edits or another browser rewalk.

- Final frozen packet: `53fbbe8f04e0e3c302738fbb39acc9e061b7271b`, correcting initial packet `77145c15f84b349c2445a018d2b5a53cb421d249`.
- Base/package source: `7d09d0a60fbb2cc12e259541614b9483ee618d29`.
- Worktree: `/home/chris/.local/share/boardstudio/worktrees/pcb02-public-qualification-20261002`, clean at review.
- Receipt: `.scratch/dioxus-pcb-view/evidence/pcb02-public-qualification-20261002/paired-walkthrough.md`, independent SHA-256 `3e322f93400a4d2e07a71ff2bb62dd13228b85908ece0821a65bb05872397c65`. Initial receipt SHA-256 `5374dc4a4b516f919dc86950b365b4fea7ca0de3b878ed453fa03787be30a0b6` is historical.

| Axis | Verdict | Bound |
| --- | --- | --- |
| Standards | CLEAR | Truthful bounded evidence, preserved ownership and explicit unclosed gates. |
| Spec | CLEAR | Supported host-layer evidence and missing module/workbench criteria are recorded without full parent closure. |

## Standards

The packet changes documentation and frozen evidence only. It preserves source implementation, existing ticket ownership, original fixture identity and complete canonical-parent acceptance requirements. All relevant issue checkboxes remain open. Mounted-module ownership is explicitly separated from host layer state and Layout Footprints, consistent with pinned React source.

Initial receipt attribution was imprecise: the Left-return text snapshot was captured before technical layers repopulated, while the later PNG shows the settled menu. Final 53fbbe8 changes only that sentence, explicitly names the settled screenshot as the oracle and identifies the text snapshot as interim. It preserves the original capture instead of hiding it or attributing a transient missing inventory to a product failure. This resolves the review observation.

No new refactoring takeaway was observed in this bounded packet review; RF-001/RF-006/RF-009 and parent joins are retained.

## Spec

The paired baseline screenshots/snapshots show the same Sofle v2 Left PCB, 70 parts, actual Copper/Technical/Objects menu and twelve corresponding host controls. The candidate screenshot visibly separates Back copper/Front copper, real technical layers and Board outline/Courtyards. The candidate retention snapshots preserve Show B.Cu/Show F.SilkS after Layout; the Right-board snapshot changes selected board while retaining hidden B.Cu. The settled Left-return PNG visibly restores the technical rows with F.SilkS still hidden. The interim text snapshot appropriately excludes those rows while discovery is pending.

The receipt records candidate geometry hide/restore observations for all twelve host controls and explicitly distinguishes these author-observed DOM counts from React pixel parity. Independent archive inspection supports its baseline and copper deltas: Left has 70 parts, 70 nonempty courtyards, 70 references, 160 plated pads and 198 drills; resolving pad/part side plus through-hole either-face visibility gives 102 remaining plated pads when either B.Cu or F.Cu alone is hidden, exactly the recorded table. Independent byte identity of the before/after candidate archive confirms no serialized project mutation during this visibility journey.

The separate React Mounted modules/Footprints control is present in the paired React full snapshot and omitted from the candidate. Pinned Workbench.tsx maintains `hiddenModuleLayers` independently from `hiddenLayers` and Layout `showFootprints`. The frozen exported project has zero modules, corroborating the fixture limitation. The packet correctly retains this missing row/effect as an open criterion requiring a module-bearing selected-board fixture; Layout Footprints and host generated graphics are not used as substitutes.

Toolbar, Add/tree and Wiring mode/Apply gaps remain explicit with existing F3/F5 ownership. React and candidate snapshots demonstrate the control-presence observations; they do not qualify those controls' edit behavior. The packet also preserves compact/theme/accessibility, complete history/reopen and all parent joins as open.

## Verification

- Exact base-to-final packet diff check: passed.
- Final correction diff: one receipt sentence only; no source change.
- Independently matched all **14 listed snapshot/PNG digests** in the receipt.
- Six frozen PNGs are each **1280×577**. Inspected React menu, candidate baseline menu and settled candidate Left-return PNG visually.
- Exact original layered Sofle fixture SHA-256 independently matches `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Candidate before and after exported archives: each **1,585,879 bytes**, identical SHA-256 `d68e6651757b00ce6269ee521bd10acc7cc175d0faafe22813d3abb98614211c`. This is candidate before/after equality, not equality with imported archive bytes.
- Archive project independently identifies `Sofle v2`, revision **9**, two 70-part boards and zero modules. Existing model assets and project JSON are preserved in both identical archives.
- Reused independent exact root/source clearance and [full package integrity review](frontend-authoring-layers-full-package-review-7d09d0a6-sol-20261002.md), whose cited SHA-256 `c5a8009af0c35c7bc11fda3aa0f9a41c8a3ca673af7341c6d2c15edad32443ca` matches the independently produced report. Candidate package source/provenance are the same frozen 7d09 build reviewed there; no fresh package rebuild or broad suite was needed.

Browser session interaction counts remain author observations; this independent review validates their frozen artifacts, cross-checks geometry expectations against the actual exported fixture, and verifies the acceptance bounds. It does not claim a second live execution of every toggle.

## Open joins

The final evidence packet is clear to join. It establishes the reported supported host-layer journey for the recorded fixture and view, while **full PCB02 and parent acceptance remain OPEN**. Separate mounted-module row/geometry, compact/theme/assistive checks, complete paired output/history/reopen qualification, adjacent toolbar/tree/wiring workflows, absent provider board-level categories and canonical parent/RF acceptance joins are preserved. The subpath import journey is route coverage; it is not reload persistence of transient visibility.

Standards: zero findings. Spec: zero findings in the corrected bounded evidence packet.
