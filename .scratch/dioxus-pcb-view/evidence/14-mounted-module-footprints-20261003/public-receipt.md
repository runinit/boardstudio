# Mounted-module Footprints: bounded paired receipt

Reference: React5173, source `5a472a9426e6e38993361da402cd4ec730feb369`.
Candidate: Dioxus34748, source `a76fa2bdee3379be1d9d2c2133f9428a87a75fb0`,
provenance SHA-256 `7e764747832a5ce20ba2c9091938eeeaee06283583a9071a89d8321c6dc88299`.
Root/subpath200 COEP; full22 build commands succeeded. Combined affected strict
page WASM all-target Clippy passed before packaging; no new unit tests.

Own browser session `root-pcb-module-20261003`, viewport1280×940. On React, selected
**Start VIK module review · above and below**, opened PCB and Layers. Saved the
portable project and imported that exact archive into Dioxus34748. Fixture SHA-256:
`952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`.

Both show three source module overlays,26 footprints,159 pad groups,45 drills,
118 silkscreen artwork shapes and25 fallback references. The retained DOM comparison
matches all373 source geometry leaves, including ancestor pose/face transforms,
source courtyards/pads/drills/artwork/reference text. It normalizes class prefixes,
attribute whitespace and numeric coordinates to9 decimal places; it excludes title
and interactive attributes because owning-module navigation remains unimplemented.
This is geometry evidence, not full pixel or navigation acceptance.

Clicked the separate mounted-module Footprints control off/on. Hiding module
footprints removes those source shapes while retaining56 host pads and117 host holes.
Host Pads off removes module pad rectangles and host pads; Host Holes off removes
module drills and host holes. The candidate's module-hidden state survives
PCB→Layout→PCB. After restoring the host/module toggles, Save .boardstudio project
returns the exact same ProjectDoc as the reference fixture, including revision6;
asset archive container bytes can differ. See saved-document-comparison.json.

One bounded accessible-label defect is recorded:34748 says `Hide module-footprints`
where React says `Hide Footprints`. Root correction `b3e020ed` uses the visible label
for the module row; its changed-label replay is pending34749. Visible label/toggle
behavior already matches. Module-free boards, board-switch retention, additional
module categories, owning-object navigation, full visual parity and parent joins
remain open. This receipt does not close PCB14 or F5.4.

Full raw captures, screenshots, archive and comparator are retained at
`/home/chris/.local/share/boardstudio/reviews/pcb-module-footprints-20261003/`.
No new refactoring takeaway: preserve RF-001 module composition and RF-006/RF-009
scope/identity evidence; no source circuit is copied into the host document.
