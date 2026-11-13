# Delivery queue

## Current status (2026-10-17)

| Effort | Status-bearing tickets | Pending | Current state |
| --- | ---: | ---: | --- |
| Check speed | 13 | 3 | 10 resolved (full check reduced from ~15 min to ~5.5 min). Remaining browser measurement/structural tickets: 11, 12, 13. |
| [Edit settlement](../design/edit-settlement-contract.md) | 20 | 0 | Complete (20/20 resolved). |
| Generator parameters | 6 | 0 | Complete (6/6 resolved). |
| Matrix layout consistency | 12 | 0 | Complete (12/12 resolved). |
| Module deepening | 19 | 0 | Complete (19/19 resolved). |
| Typed Core edits | 155 | 3 | 84 resolved (including full mirrored-half reconciliation 19, 105–117, 139, 143–160), 68 `declined` (retired shipped demo boards under 138). Only 140, 141, and 161 remain open. |
| [CAD backend replacement](cad-backend-replacement/map.md) | 14 | 11 | 2 resolved ([003](cad-backend-replacement/issues/003-contained-blind-prism-cuts.md), [005](cad-backend-replacement/issues/005-case-profiles.md)), 1 `declined` ([004](cad-backend-replacement/issues/004-valid-bounded-dish-cutters.md)), 11 active ([001](cad-backend-replacement/issues/001-certify-closed-step-boundaries.md), [002](cad-backend-replacement/issues/002-preserve-shared-angular-meshing.md), [006](cad-backend-replacement/issues/006-complete-case-features.md)–[014](cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md)). |

## Open tickets

- **UI & Library starters:** 140 (execution plan) and 161 (three synthetic layout-only Library starters + Launchpad/shelf + Setup Guide accordion + centered canvas toolbar pill).
- **Outline gap diagnosis:** 141 (single-outline mirrored gap control repro).
- **Browser check speed:** 11, 12, 13.
- **Monstertruck CAD backend replacement:** tracked on `t3/monstertrunk-step-support-research` under `docs/plans/cad-backend-replacement/map.md`.
