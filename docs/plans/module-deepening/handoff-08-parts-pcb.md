# Third app: Parts settlement

Assignment: [Parts and PCB panels settle through PendingEdits](issues/08-parts-and-pcb-onto-pending-edits.md).

Worktree: `/home/chris/.codex/worktrees/boardstudio-module-deepening/08-parts-and-pcb-pending-edits`  
Branch: `deepening/08-parts-and-pcb-pending-edits`  
Starting dev commit: `8ce860a0950304ddfae34f12d5891b8c6f267f2f`

## Ownership

Implement only the Parts half: `web/crates/parts/**`. The separate
[PCB stream](handoff-08-pcb.md) owns `web/crates/pcb/**`. Preserve this existing
worktree, branch, commits and any unfinished work when resuming. Runtime, UI-shared
and tracker files remain outside this stream. The parent ticket resolves only after
both halves are reviewed and integrated.

## Work

1. Confirm the worktree branch, HEAD and clean starting state. Read the current root `AGENTS.md`, the worktree `AGENTS.md`, [parallel-run rules](handoff.md#rules-every-agent-follows), your ticket, and its decision links. Your branch starts after the PendingEdits tracer gate; the gate is the prerequisite for source changes.
2. Inventory every settlement site in the ticket’s owned paths. Use `tdd`, `codebase-design` and the applicable Rust skills. Replace settlement policy through the shared module and remove the shallow layer it replaces. Keep domain behavior and authoritative Session work intact. Account for every site in the final report.
3. Reuse the merged native preparation tests from ticket 19; keep every domain/Undo/Redo assertion and queue/target-departure scenario. Exact post-landing selection remains required; the protected electrical remap route belongs to the PCB stream. Runtime/helper interfaces are already reviewed; report contract gaps to their owner before editing shared interfaces.
4. Run the ticket’s native checks, lint, typecheck and full test step. Request an exclusive Chrome lease from the orchestrator before browser checks; continue source/native work while another app holds it. Release the lease when the runner exits, and preserve unrelated browser processes. If a failing native step short-circuits check.py, run the browser step separately under the lease. Report native and executed browser coverage separately, plus baseline failures and unexecuted gates.
5. Pin your absolute worktree, base and final HEAD for parallel Standards and Spec reviews using `code-review`. Address findings, rerun checks affected by fixes, and report final verdicts. Read-only reviews can run in parallel with checks.
6. Report your commits, a clean working-tree status, every migrated site, executed checks, review verdicts and an Outcome draft. The orchestrator handles integration with current dev, tracker changes and any history operation requiring approval.

## Shared contract

Reuse the merged [keyed collection contract](issues/15-keyed-pending-edits.md#outcome), [Signal helper contract](issues/16-pending-edit-ui-helpers.md#outcome), and [complete Matrix tracer](issues/18-matrix-field-pending-edits.md#outcome). Read the source example before choosing your panel key type. The helper retains bindings, so use bounded logical keys and keep bound Signals alive for the observation lifetime. A real panel unmount must retire its observation even when editor/selection remain; accepted revisions alone keep it live. Field helpers own submitted-draft memory and settlement writes to bound draft/failure Signals; callers own admission, accepted projection, owner liveness, error placement and precise Landed follow-ups.

For text-field migrations, bind the actual stable draft/failure Signals before submission and pass the exact typed text before parsing. Remove the panel's submitted-request tracking and settlement restoration once the helper owns them. Accepted-projection effects must preserve a newer draft after the helper drains an older result. Prove this in a mounted held-result test, including an older failure that still appears inline beside the newer text.

## Ownership and coordination

This app owns this assignment’s source and tests in its isolated worktree. The orchestrator owns `dev`, ticket claims/resolutions and the canonical GitNexus index. Follow the current [GitNexus rules](handoff.md#rules-every-agent-follows), including impact before function edits and complete change analysis before commits. Reindexing belongs to the orchestrator. Report a shared Runtime/helper contract gap before changing another ticket’s files. Stage explicit paths, preserve unrelated edits, and use the repo’s Git safety rules.

## Known verification limits

The documented native CAD failure is `core_internal_gasket_fixtures_export_connected_positive_regions` (rotated-concave/bottom volume). Treat that gate as failing if reproduced; list the actual result. AppImage-hosted KiCad failures reported by the helper app are a separate environment issue, not the CAD failure. Run the current tree and report what actually happens. Browser presentation compiles only for WASM; native tests alone do not verify it.
