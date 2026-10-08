# Separate app: PCB settlement

Assignment: PCB half of [Parts and PCB panels settle through PendingEdits](issues/08-parts-and-pcb-onto-pending-edits.md).

Worktree: `/home/chris/01_Projects/ts-boardstudio2/.worktrees/08-pcb-pending-edits`  
Branch: `deepening/08-pcb-pending-edits`  
Starting dev commit: `8ce860a0950304ddfae34f12d5891b8c6f267f2f`

## Read first

Read current root and worktree AGENTS.md, CONTEXT.md,
[parallel-run rules](handoff.md#rules-every-agent-follows), the parent ticket,
ADR-0005 and its three amendments, and the Outcomes of
[keyed collection](issues/15-keyed-pending-edits.md#outcome),
[Signal helpers](issues/16-pending-edit-ui-helpers.md#outcome),
[Matrix consumer](issues/18-matrix-field-pending-edits.md#outcome) and
[native preparation](issues/19-parts-pcb-native-test-preparation.md#outcome).
Use tdd, codebase-design, applicable Rust skills and final code-review.

## Ownership and work

Own `web/crates/pcb/**` only. Parts runs independently in its existing worktree.
Runtime, UI-shared and tracker files are outside this stream; report contract gaps.
Confirm actual branch/HEAD/status and preserve all existing work when resuming.

Move every PCB settlement site onto PendingEdits/PendingEditSignals and remove the
per-panel settlement layer:

- pcb_wiring/mode.rs: ModeTickets; pins.rs: PinTickets; apply.rs: ApplyTickets.
- pcb_wiring/controller.rs: FirmwareTickets and PartNetTickets.
- pcb_wiring/part_input_settings/owner.rs: remove PartInputFeedbackState and the
  named owner_is_live function while preserving caller-owned liveness semantics.
- pcb_module_inspector.rs, pcb_board_reference.rs, pcb_physical_setup/controller.rs.
- Their render sites: pcb_wiring.rs, part_input_settings.rs, firmware_positions.rs.

Use bounded logical keys by feedback target, assignment id or action kind. Keep
pending_mode, pending_pin and InputDrafts only for domain projection memory; the
shared helper owns submitted-text bookkeeping and settlement writes. Keep schema
preparation queues. Bind real stable draft/failure Signals and submit exact text
before parsing or trimming. Preserve newer text beside an older inline failure,
owner retirement and latest-per-key replacement with mounted held-result tests.

Remove Saved/landed presentation statuses, including “Wiring mode saved.”,
“Wiring plan applied and saved.”, “Connection saved.”, “Placement saved.” and
“Physical setup saved.” Update removed-status assertions in physical setup and
module inspector tests. Keep ticket 19's mode_owner_tests queue, target-departure,
Apply eligibility and domain assertions intact. Preserve exact Landed follow-ups.

**Do not edit pcb_wiring/remap.rs.** Protected electrical remap retains its strict
captured-revision route.

## Verification and report

Use `CARGO_BUILD_JOBS=2`. If rustup reports “unknown proxy name”, prepend
`/home/chris/.rustup-toolchain-shim` to PATH; do not change unrelated configuration.

```sh
cargo test -p boardstudio-web-pcb --locked
python3 scripts/check.py lint typecheck test browser
```

Native preparation had 21 passing PCB tests; execute the current tree and report
actual counts. Request the exclusive Chrome lease from root before browser execution;
continue code/native/review work while another stream holds it. Release the lease
when the runner exits. If test stops at the known CAD gasket-volume failure
(rotated-concave/bottom expected 80481.2399), run browser separately under the lease.
Compilation alone does not verify mounted WASM behavior.

Follow mandatory impact/change-analysis rules in the linked run rules. Root owns
canonical reindexing. Stage explicit paths only; preserve unrelated work.
Run parallel Standards/Spec reviews pinned to base `8ce860a09` and final HEAD.
Fix findings, rerun affected checks and report commits, every migrated site, executed
checks, review verdicts, skipped/blocked gates and an Outcome draft. Root coordinates
integration and any approved history operation; do not automatically rewrite history.
The parent ticket closes only after both Parts and PCB halves are accepted.
