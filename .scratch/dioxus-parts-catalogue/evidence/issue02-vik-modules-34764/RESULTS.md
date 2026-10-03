# VIK module catalogue Inspector journey — 34764

Date: 2026-10-03

## Pin

- Served application: `http://127.0.0.1:34764/`
- Served source: `661296fe5633e2327488cd56738c14ab81c22341`
- Build: `frontend-board-geometry-vik-20261003`
- Provenance SHA-256: `f597c5d1cd45a8222b5ce8628ffbb5508e7400094cf1f53bcf6518e1681519c0`
- Browser profile: `parts-vik-modules-candidate-34764`
- React reference: `.scratch/dioxus-parts-catalogue/evidence/issue02-vik-modules-reference-20261003/RESULTS.md`

## Result

The Parts Inspector journey passed for the mounted VIK module catalogue row and its exact variants. The Sofle v2 project copy exposed “various display adapters in different sizes” with three variants: `pcb/2inch-0.8mm/pcb`, `pcb/1.47inch/pcb`, and `pcb/1.28inch-round/pcb`. Selecting the 1.47-inch variant retained its module identity and source attribution (`MIT · 114070d99365 · Upstream: Complete`) and changed the readiness summary from one Case item to review to two. The initial selection showed Footprint and Electrical clear, one Case item to review, one 3D model, and one Firmware item. No browser console errors were observed.

The module row and Inspector updated, but the center canvas remained the MX switch preview. This reproduces the source-preview gap and means this run does not qualify the full Issue02 module-preview acceptance. The center-canvas repair is being assessed separately on the next candidate; this receipt records only the Inspector row, variant identity, provenance, and readiness behavior.

## Captures

- `first-variant.png` — SHA-256 `1739ed1c5d66e3939d8a85bab81cc8edd8e229135cd4af5fc91d199192f4cf74`
- `variant-147-inch.png` — SHA-256 `d0fa0247e9662e1ac36f840b0305a9579cd5c54e501911c3c669d69c6a4a23b7`

The package-level source-input, route asset, and strict Clippy evidence belongs to the root packaging record; this browser receipt does not repeat or replace it.
