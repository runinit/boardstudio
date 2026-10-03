# Private PCB owner admission probe — 2026-10-03

This extends [the public import/reload reproduction](import-reload-reproduction.md) without changing its observed result or proposing a speculative repair.

`web/src/presentation/pcb_wiring/mode.rs` now factors the production `current_snapshot` predicate through a private `current_snapshot_probe`. Production callers still receive only `Option<AcceptedSnapshot>`; the probe's private blocker enum records which existing check rejected a request: workspace, instance-selection freshness, accepted snapshot, lifecycle, preview, gesture, durability, board/instance, session/document, action context or board membership. It adds no Runtime state, logging or public API.

The mounted owner test `mounted_owner_refreshes_from_a_real_session_open_and_board_navigation` advances a real `Session` through `Open`, Core completion, committed persistence and board navigation, feeding each read model to the production owner hook. The owner is unavailable while Open is pending; after the accepted document is Saved and navigation establishes the current board scope, the same production predicate succeeds and the mounted wiring action becomes editable. The test also captures the pre-save rejection as Lifecycle or Durability rather than treating an intermediate state as a defect.

This test narrows the source investigation: the normal `Session::Open` → Core → SaveCommitted → Navigate sequence does not leave the production owner stuck disabled. It does **not** reproduce the public archive's disabled control after the UI displays Ready/Saved, does not observe the actual browser predicate result, and does not explain the reload-only recovery. The issue remains open and timing-dependent; no source fix is attributed to this probe. Re-run the paired public journey on the integrated candidate and preserve the actual failing gate before changing behavior.

The adjacent Issue 11 pin/lock control uses the same existing action-context and current-snapshot guards. Its mounted tests are separate from this import diagnosis.
