# 08: Dead dependencies

Status: claimed
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

## Comments

- Claimed for read-only research delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3Afbfc6f4e-f656-4738-a312-a2dcbd3a3b11`
  (findings go to `docs/research/2026-10-29-dead-deps.md` on branch `research/dead-deps`,
  pushed, no PR).
