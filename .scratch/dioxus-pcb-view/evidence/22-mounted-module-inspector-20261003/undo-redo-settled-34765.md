# Mounted module Inspector accepted-history sync — settled candidate check

Date: 2026-10-03

Candidate: `http://127.0.0.1:34765/boardstudio/`
Frozen source: `17ba32b984c13f3621a2145efe1dd478862ab5cd` (parent `96ab3eb0f4710ac0b5e80d672133c8f6c8ecb9c3`)
Candidate provenance SHA256: `273ca2fa2315b32fec91b0490d2f70fb03b5426d759310bced6e934ad73947d5`
Fixture archive SHA256: `952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`

## Settled Undo/Redo result

Using the existing named profile `pcb-mounted-module-inspector-20261003`, with the exact VIK fixture already imported and its mounted module selected:

1. Before the history action the mounted-module Inspector showed X `35`, Y `20`, yaw `0`, gap `5`, host/facing face Back, attachment Case.
2. Clicked the app's `Undo` button and waited 800 ms. The same selected module remained in the Inspector; X updated to `32.5`, and the other values remained unchanged.
3. Clicked `Redo` and waited 800 ms. The same selected module remained in the Inspector; X returned to `35`, with the other values unchanged.

Settled screenshots:

- `history-undo-34765.png`, SHA256 `eefd83df06a876586526e7c12ef1f843cda05a94b39bb152a2faffc62686c840`
- `history-redo-34765.png`, SHA256 `495edbd7b97b161c493a76c25f82eda792233f36156dfcd3a36b7eac76b8f75d`

The frozen source at `web/src/presentation/pcb_module_inspector.rs` lines 70–76 tracks the accepted instance as the effect input and reads the local draft with `draft.peek()`, so draft edits do not subscribe the effect to itself. The settled candidate history transition above is GREEN. Save/reopen evidence and package checks remain in their prior receipts.

## Disposition of earlier stale-looking observation

An earlier interaction sequence produced an immediate Inspector display that looked stale after Undo, but it was not followed by a settled snapshot and was reported against the same 34765 candidate lifetime that contains the `draft.peek()` fix. No distinct older source/build provenance supporting a RED is retained. Therefore this is recorded as an unsettled/uncertain observation, not a confirmed product defect. No source edit or extra regression test was made.

This check covers one mounted module and one accepted Undo/Redo transition. It does not claim complete F5.5 module editing, F4 definition/readiness parity, service-clearance/support editing, or all62 parent acceptance.
