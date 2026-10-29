# 08: Dead dependencies

Status: ready-for-agent
Type: research
Parent: [CI wall time map](../map.md)

## Question

Which dependencies can go without losing protection, itemized with per-item evidence?

## Pointers

- Scale: 18 workspace members plus `cad/wasm`, 427 locked packages.
- Leads, not conclusions: the unused `renderer/vendor/cgmath-0.18.0` fallback (patched to
  `cgmath-next`), full-catalogue loading, repetitive mechanical-settings field definitions.
- Method (no new tooling required): `cargo metadata` build-graph analysis across all features
  and cfgs — an item qualifies only if no feature/cfg combination references it.
- Removals graduate as follow-up tasks with per-item justification and a green full gate;
  lockfile churn lands after 03 scores the warm baseline and re-measures after.
- Deliverable: the itemized list with evidence per item, so removals need no further research.
