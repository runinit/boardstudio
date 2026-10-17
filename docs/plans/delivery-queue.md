# Delivery queue

## Current status (2026-10-17)

| Effort | Status-bearing tickets | Pending | Current state |
| --- | ---: | ---: | --- |
| [Check speed](check-speed/map.md) | 13 | 3 | 10 resolved (full check reduced from ~15 min to ~5.5 min). Remaining browser measurement/structural tickets: [11](check-speed/issues/11-browser-measure.md), [12](check-speed/issues/12-browser-quick-wins.md), [13](check-speed/issues/13-browser-structural.md). |
| [Edit settlement](edit-settlement/map.md) | 20 | 0 | Complete (20/20 resolved). |
| [Generator parameters](generator-parameters/map.md) | 6 | 0 | Complete (6/6 resolved). |
| [Matrix layout consistency](matrix-layout-consistency/map.md) | 12 | 0 | Complete (12/12 resolved). |
| [Module deepening](module-deepening/map.md) | 19 | 0 | Complete (19/19 resolved). |
| [Typed Core edits](typed-core-edits/map.md) | 154 | 2 | 84 resolved (including full mirrored-half reconciliation [19](typed-core-edits/issues/19-reconcile-mirrored-halves.md), [105](typed-core-edits/issues/105-mirrored-half-derivation-kernel.md)–[117](typed-core-edits/issues/117-prepare-mirrored-preview-history-performance.md), [139](typed-core-edits/issues/139-generalized-mirrored-constraint-model.md), [143](typed-core-edits/issues/143-part-offset-constraint.md)–[160](typed-core-edits/issues/160-show-paired-preview-electrical-loss.md)), 68 `wontfix` (retired shipped demo boards under [138](typed-core-edits/issues/138-retire-shipped-demos.md)). Only [140](typed-core-edits/issues/140-ui-design-impeccable.md) and [141](typed-core-edits/issues/141-mirrored-outline-gap-controls.md) remain open on this branch. |
| [CAD backend replacement](cad-backend-replacement/map.md) | 14 | 11 | 2 resolved ([003](cad-backend-replacement/issues/003-contained-blind-prism-cuts.md), [005](cad-backend-replacement/issues/005-case-profiles.md)), 1 `wontfix` ([004](cad-backend-replacement/issues/004-valid-bounded-dish-cutters.md)), 11 active ([001](cad-backend-replacement/issues/001-certify-closed-step-boundaries.md), [002](cad-backend-replacement/issues/002-preserve-shared-angular-meshing.md), [006](cad-backend-replacement/issues/006-complete-case-features.md)–[014](cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md)). |

## Open tickets

- **UI design:** [140](typed-core-edits/issues/140-ui-design-impeccable.md) ([execution plan](ui-design/plan.md)).
- **Outline gap diagnosis:** [141](typed-core-edits/issues/141-mirrored-outline-gap-controls.md) (single-outline mirrored gap control repro).
- **Browser check speed:** [11](check-speed/issues/11-browser-measure.md), [12](check-speed/issues/12-browser-quick-wins.md), [13](check-speed/issues/13-browser-structural.md).
- **Monstertruck CAD backend replacement:** tracked on `t3/monstertrunk-step-support-research` under `docs/plans/cad-backend-replacement/map.md`.
