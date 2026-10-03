# VIK module source-preview correction — 34765

Date: 2026-10-03

## Pin

- Served application: `http://127.0.0.1:34765/boardstudio/`
- Served source: `17ba32b984c13f3621a2145efe1dd478862ab5cd`
- Build: `frontend-context-findings-footprints-20261003`
- Provenance SHA-256: `273ca2fa2315b32fec91b0490d2f70fb03b5426d759310bced6e934ad73947d5`
- Browser session: `parts-vik-preview-34765-2318ea1a1376`
- Prior Inspector proof reused: [34764 receipt](../issue02-vik-modules-34764/RESULTS.md)

## Changed-path result

Opened the Sofle v2 copy, selected the VIK group “various display adapters in different sizes,” then switched from `pcb/2inch-0.8mm/pcb` to `pcb/1.47inch/pcb`. The Parts center changed from the previous MX footprint canvas to a “Module source preview” for the selected group. The visible board contour, component courtyards, drilled features and mounting holes changed with the exact selected variant. The preview footer reported `1.6 mm PCB · 17 source components · 2 mounting holes`. The Inspector retained the selected variant and readiness summary (including Case moving from one item to review to two), with the pinned source attribution still visible.

This qualifies the repaired selected-module center-preview path for the exercised group and variant. It does not qualify every VIK module, placement, wiring, electrical readiness, or the full Issue02 parent acceptance.

## Captures

- `first-variant.png` — SHA-256 `6ba278fb9b0f44c1ffd3db07f5239cc57b394fc1cc81e532f0dfcf492b0f9db0`
- `variant-147-inch.png` — SHA-256 `e1d0e408f3555c74cc27134fe32efbdc54663f0e2bbe9484b427306ab7100a06`

Package source-input, route-asset, and strict Clippy evidence remains in the root candidate record. The Inspector assertions reuse 34764 and are not presented as a second full Issue02 replay.
