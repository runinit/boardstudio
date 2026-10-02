# PCB07 exact correction independent Spec/Standards re-review

Exact source: `0a1d08ad7c72592186e11c00c06c9d1c6b684699` in clean worktree `pcb07-contextual-part-inspector-20261002`. Original reviewed source: `4f1338d931d14d0260f53b784990ec2964d087e3`.

Disposition: **original classification finding corrected; source clearance remains held for two newly introduced asynchronous ownership defects**. Exact commit-range diff check passed. Existing packaged `isErgogen` is now called for actual generator source, and the boolean guards both rendered standalone rows and proposal admission. Arbitrary project ID with supported source and Ergogen-looking non-generator ID are covered by the author regression; the separate real packaged-module test proves source recognition. Reused reported red/green and strict evidence; no broad rebuild was needed to identify the following actual source paths.

## P1: detached classification callback accesses dropped Editor signals

`web/src/presentation/pcb_wiring/controller.rs:422-440` launches `wasm_bindgen_futures::spawn_local`, awaits dynamic module classification, then calls `instance_is_current()` and reads scope_generation/pending. There is no alive guard or scoped-task cancellation in `use_pcb_part_net_edits`. The actual caller in `presentation.rs:1333` captures InstanceSelection, whose `is_current` reads its Signal (`presentation.rs:104`). After Editor teardown, the detached callback can therefore throw ValueDroppedError before even reaching an identity rejection. Use the established private alive/use_drop guard before any signal-backed callback/read after await, or a correctly scoped pre-admission task. Add a mounted regression that leaves classification pending, tears down the actual owner, then resolves it. Do not cancel an already admitted exact operation's settlement merely because its leaf hides.

## P2: post-await workspace check is a captured constant

At controller.rs:421 `let workspace = workspace()` shadows the live Signal. The check at line438 tests the captured string, which is always PCB for an admitted request. Switching away from PCB while classification is pending therefore does not reject the stale UI intent, if the selected part/scope/document stay unchanged. Workspace switching directly changes WorkspaceState and need not change the selection adapter generation. Retain the live signal (read only after lifetime check) and recheck it after await; add a production callback regression for PCB -> Layout during delayed classification with no accepted document change.

Both findings arise in the new admission await, not in the already reviewed exact outcome settlement. No provider/API redesign is needed. Current root CONSTRAINTS at d9d4bc38 permits the narrow private repair without an extra approval quiz. Retain RF-006 and RF-009, plus RF-001 shared ownership. Public mapping/history/Undo/reopen, integrated strict and full parent gates remain open.

This is a read-only source review; findings are source-backed and still require author regression red/green. No source files were edited. The historical /tmp/pcb07-source-independent-review-20261002.md was readable early in this pass but was no longer present when appending; this separate report preserves the exact current disposition without claiming to reconstruct that historical file.
