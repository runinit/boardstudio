# Delivery queue

Historical snapshot, not a live queue. Installed BMad skills and their configured
ticket tree are the sole planning and task-state authority. Retain the dispositions
below, including canceled plans, as historical evidence.

## Current status (2026-10-17)

| Effort | Status-bearing tickets | Pending | Current state |
| --- | ---: | ---: | --- |
| Check speed | 13 | 3 | 10 resolved (full check reduced from ~15 min to ~5.5 min). Remaining browser measurement/structural tickets: 11, 12, 13. |
| [Edit settlement](../design/edit-settlement-contract.md) | 20 | 0 | Complete (20/20 resolved). |
| Generator parameters | 6 | 0 | Complete (6/6 resolved). |
| Matrix layout consistency | 12 | 0 | Complete (12/12 resolved). |
| Module deepening | 19 | 0 | Complete (19/19 resolved). |
| Typed Core edits | 155 | 3 | 84 resolved (including full mirrored-half reconciliation 19, 105–117, 139, 143–160), 68 `declined` (retired shipped demo boards under 138). Only 140, 141, and 161 remain open. |

## Open tickets

- **UI & Library starters:** 140 (execution plan) and 161 (three synthetic layout-only Library starters + Launchpad/shelf + Setup Guide accordion + centered canvas toolbar pill).
- **Outline gap diagnosis:** 141 (single-outline mirrored gap control repro).
- **Browser check speed:** 11, 12, 13.

## Canceled plans

- **CAD backend replacement:** canceled by the user on 2026-11-15; the map, spec and tickets were removed. Implementation and accepted ADRs were not reverted. Historical planning records remain in Git.
