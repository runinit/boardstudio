# Case Issue13 battery envelope implementation handoff

Issue13 planning and capability-start review: commit `0b7d27c8bf8fb363539f8e9127e18afda77238fd`; both-axis capability-start review at `/home/chris/.local/share/boardstudio/reviews/case13-battery-capability-start-review-0b7d27c8bf8fb363539f8e9127e18afda77238fd-sol-20261002.md` (SHA-256 `2a06d898783d24b3bb53747a3847ab5a445742f9cf55b9a1e3e76b7ee0fbc454`). Implementation commit `e3fdb7ee13bb6ab64cf9c033b6889405f4916e64`, based on `f261a327858f51a1de4928374bc65b669e2792a3`.

## Scope and behavior

The private Case mechanical settings surface now projects transport plus the current effective configuration's optional battery. Wired settings expose an include/remove checkbox and, when present, eight numeric fields: width, depth, height, cable width, position X/Y and cable exit X/Y. Adding uses the reviewed 30×20×6 mm, origin position/exit and 2 mm cable defaults. `None` cable width displays the reference 2 mm fallback. Wireless settings retain their effective battery fields and guidance, without showing the manual include toggle. Dimension/cable fields use the reviewed 0.1 mm minimum; coordinates use the reviewed −1,000,000 mm minimum. All edits remain `MechanicalSettingsPatch` requests through the existing scoped controller, accepted-source check, configuration resolution and operation/persistence path.

The change does not broadcast a physical-instance battery edit to sibling instances, change canonical/physical ownership rules, or alter construction-linked propagation. Existing construction and manufacturing normalization remains active for its owning patch families. Battery-only changes bypass that unrelated process normalization, preserving the rest of the current configuration exactly. The broader `apply_patch` funnel and normalization policy are left for the post-port refactoring phase.

## Test evidence

- `wired-toggle-red.log` reproduces the missing wired toggle before UI implementation (SHA-256 `69f676a46e0e81c825e69c119c7627441800b68bf1bb2f3e182df0bfc92428d8`).
- `process-normalization-red.log` runs the exact battery-removal configuration-equality regression against the unguarded existing normalization call. It fails because a removal adds `partProcesses: []` when the accepted configuration had `None` (SHA-256 `dde9d9b06a7b5e33427238fdabad61006613a1769b649d4ff7e235919da3e6dd`).
- `controller-green.log` passes all four focused browser-WASM configuration tests: defaults, exact non-battery preservation on removal, all eight field projections and validation, plus wireless rejection (SHA-256 `640d7a648c640460c52d7f2e9e627f8028ac3a1b7eebdc502a0bcb08f3c6b759`).
- `ui-suite-green.log` passes both mounted mechanical Inspector tests, including wired toggle request emission, wireless guidance/field visibility, all eight minimum/step attributes and the 2 mm cable-width fallback (SHA-256 `0b36be4c7eafc819d2f4ccd42127a447036be61cc09427d3da51f7c336e6d12a`).
- `wasm-clippy.log` records strict all-target WASM Clippy success with `-D warnings` (SHA-256 `73dae56d622099e09c8c8cadbc146bef9452357abe8494c44256e648f72747f6`).

The equality regression uses a direct `MechanicalConfiguration` controller fixture. These checks establish the private controls/patch semantics, not the public paired Issue13 acceptance: full runtime edits, accepted payload, save, Undo/Redo, reopen, scope-change behavior, and hardware-context journey remain open.

## Refactoring ledger handoff

This source pass confirmed one bounded incidental mutation in the existing mechanical-settings patch pipeline: its unconditional manufacturing-process normalizer created an empty process override vector for a battery-only removal. The current correctness patch avoids that unrelated normalization for battery-only toggles/fields while preserving construction/process coupling for the patch types that own it. Record the observation under existing RF-001; do not create a new RF ID or broaden this implementation into a normalization redesign. Future refactoring should assess how the shared patch funnel separates field ownership and derived process updates, with operation/history and selected-instance behavior held constant.
