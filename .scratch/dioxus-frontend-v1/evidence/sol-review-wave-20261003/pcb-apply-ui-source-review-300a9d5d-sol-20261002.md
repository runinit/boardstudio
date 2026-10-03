# PCB Apply mode repair — 300a9d5d

**Standards CLEAR; Spec CLEAR for bounded repaired source**, exact commit `300a9d5dceb4f08ee24de5c898f2420ef92b80b3`, evidence-only commit `2e02cdf4e81a60411904657bb61ef6d4b31ec470`, clean isolated `/home/chris/.local/share/boardstudio/worktrees/pcb-wiring-mode-apply-20261002`. Preserve the prior `e6870254` HOLD report as history. Governing corrected F5.2d planning remains `73b687ba83c6fa6db7685acb809f0b4ca31bd975`.

The repair closes the one actionable source finding: both rendered eligibility and the actual on_apply callback use the accepted snapshot document to compare `plan.mode` with the selected board's electrical configuration, defaulting to Matrix when absent. This cannot apply a valid Direct plan to an accepted default-Matrix board. It retains exact Current plan identity/revision/board/canonical-scope/error checks and the same private materialize→one strict-revision ReplaceDocument operation. No mode mutation policy, public API or second writable owner is added.

The positive fixture now contains real switch, diode, controller definitions and a board-bound matrix, and obtains an error-free Matrix plan from Core electrical::resolve. The mounted production hook verifies its exact generated nets/board net IDs and retains Matrix. Negative coverage uses an error-free Core Direct plan against the accepted default-Matrix document; also rejects error-bearing Current, Pending and Failed states. Both the UI editable flag and callback dispatch are checked. The existing context-replacement, failure and saved-settlement tests continue in the same suite.

Independent verification: inspected the complete two-file frozen diff and production call sites, checked clean state and source-range diff whitespace, hashed/read immutable logs and addendum. Reused the exact native VirtualDom production-hook suite (9 passed) and the disposable old-equality-guard mutant (fails specifically `mode-mismatch plans must disable Apply`). No heavy rerun was needed. Evidence accurately says the Runtime is a native stub and outcomes are manually settled; this is not a real Session or packaged app test.

Exact SHA-256 values:

- `web/src/presentation/pcb_wiring/apply.rs`: `e15cecfcd597d258d86504fffe03a68d69945c4ef902d1f4b9007338d46e2e10`.
- `web/src/presentation/pcb_wiring/mode_owner_tests.rs`: `e1f45ba1b5f3f82af9d66854ae74403816d5409f1fe483835ead5e1dce6730e6`.
- Repair addendum: `97b386d08d175461f615e3769a059eb911777fd648b08e25cdcd0bd3729c5e34`.
- Expected red: `786a371be6dfefdf9382fbd5b63194537748557f66884400a996c40b9ccc2485`.
- Restored green: `cce70329c31616e26885236b66f8e598ada327810b1aee10a56db16908fc17f9`.

Fresh affected WASM and combined strict all-target qualification remain required at root integration; the author explicitly has not run them on this new source. This clearance permits the serial join and does not claim those checks passed. The separate fresh-import PCB-disabled defect, real packaged Apply/history/save-reopen, and parent F5 joins remain open. Core09 expected-green persisted-lock coverage remains separately cleared and does not establish UI acceptance. No new refactoring finding beyond the existing private ownership/shared-composition records.
