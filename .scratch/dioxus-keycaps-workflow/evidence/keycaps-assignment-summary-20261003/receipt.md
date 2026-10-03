# F6C.2 Keycaps assignment summary source receipt

The mounted Keycaps Inspector test first failed because the heading selector did not exist, then passed after the Inspector began rendering `Keycaps` and the accepted projection's assigned/total values. The test replaces the mounted projection from 0 to 1 assigned key and observes `0/2 assigned` then `1/2 assigned`.

- Regression test: `presentation::keycaps_workspace::tests::inspector_header_shows_accepted_assigned_and_total_key_count`
- Expected-red log: `expected-red.log`
- Green log: `green.log`
- Formatter and `git diff --check`: passed before the green run.
- This is a mounted Inspector projection test, not a production workspace browser journey. Paired browser assignment/Undo/Redo/board-switch/save-reopen and transparent/unassigned checks remain open. The prior matrix-profile browser journey remains separate bounded evidence.
- No new RF observation beyond RF-001's contextual Inspector gap; keep the separate shared Objects-pane mismatch open for root's ledger.
